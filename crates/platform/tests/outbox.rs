//! Relay behaviour against a real Postgres, including two concurrent relays.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use platform::{
    event_bus::{EventEnvelope, EventHandler, HandlerError, Subscriptions},
    outbox::{NoBroadcast, OutboxTable, Relay, claim},
};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use uuid::Uuid;

async fn pool() -> (PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &platform::migrator())
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE SCHEMA testmod;
         CREATE TABLE testmod.outbox (
             id uuid PRIMARY KEY, event_type text NOT NULL, payload jsonb NOT NULL,
             created_at timestamptz NOT NULL, published_at timestamptz);
         CREATE TABLE testmod.effects (event_id uuid PRIMARY KEY, n int NOT NULL DEFAULT 1);",
    )
    .execute(&pool)
    .await
    .unwrap();
    (pool, container)
}

async fn emit(pool: &PgPool, count: usize) -> Vec<Uuid> {
    let mut ids = Vec::new();
    for _ in 0..count {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO testmod.outbox VALUES ($1, 'Thing', '{}'::jsonb, now())")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        ids.push(id);
    }
    ids
}

async fn published(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM testmod.outbox WHERE published_at IS NOT NULL")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn effects(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT coalesce(sum(n), 0)::bigint FROM testmod.effects")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Applies an effect (one row per event) guarded by `claim`.
struct Recording {
    pool: PgPool,
    fail: AtomicBool,
    calls: AtomicUsize,
    /// Events whose payload has `"poison": true` fail with a permanent (or transient) error.
    poison_is_permanent: bool,
}

#[async_trait]
impl EventHandler for Recording {
    fn consumer(&self) -> &'static str {
        "recording"
    }
    fn event_type(&self) -> &'static str {
        "Thing"
    }
    async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if event.payload["poison"] == true {
            return Err(if self.poison_is_permanent {
                HandlerError::Permanent("bad event".to_owned())
            } else {
                HandlerError::Transient("flaky".to_owned())
            });
        }
        if self.fail.load(Ordering::SeqCst) {
            return Err(HandlerError::Transient("boom".to_owned()));
        }
        let io = |e: sqlx::Error| HandlerError::Transient(e.to_string());
        let mut tx = self.pool.begin().await.map_err(io)?;
        if claim(&mut tx, self.consumer(), event.id)
            .await
            .map_err(io)?
        {
            sqlx::query("INSERT INTO testmod.effects (event_id) VALUES ($1)")
                .bind(event.id)
                .execute(&mut *tx)
                .await
                .map_err(io)?;
        }
        tx.commit().await.map_err(io)
    }
}

fn relay(pool: &PgPool, handler: &Arc<Recording>) -> Relay {
    let mut subs = Subscriptions::new();
    subs.add(handler.clone());
    Relay::new(
        pool.clone(),
        vec![OutboxTable::new("testmod").unwrap()],
        Arc::new(subs),
        Arc::new(NoBroadcast),
    )
}

fn handler(pool: &PgPool) -> Arc<Recording> {
    Arc::new(Recording {
        pool: pool.clone(),
        fail: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        poison_is_permanent: true,
    })
}

#[tokio::test]
async fn delivers_and_marks_published() {
    let (pool, _c) = pool().await;
    emit(&pool, 3).await;
    let n = relay(&pool, &handler(&pool)).tick().await.unwrap();
    assert_eq!((n, published(&pool).await, effects(&pool).await), (3, 3, 3));
}

#[tokio::test]
async fn failing_handler_leaves_event_unpublished_and_retries() {
    let (pool, _c) = pool().await;
    emit(&pool, 1).await;
    let h = handler(&pool);
    h.fail.store(true, Ordering::SeqCst);
    let r = relay(&pool, &h);
    assert_eq!(r.tick().await.unwrap(), 0);
    assert_eq!(published(&pool).await, 0);
    h.fail.store(false, Ordering::SeqCst);
    assert_eq!(r.tick().await.unwrap(), 1);
    assert_eq!((published(&pool).await, effects(&pool).await), (1, 1));
}

#[tokio::test]
async fn redelivery_after_crash_does_not_repeat_effect() {
    let (pool, _c) = pool().await;
    let ids = emit(&pool, 1).await;
    let h = handler(&pool);
    // The handler ran, then the process died before `published_at` was set.
    let env = EventEnvelope {
        id: ids[0],
        event_type: "Thing".to_owned(),
        payload: serde_json::json!({}),
    };
    h.handle(&env).await.unwrap();
    assert_eq!(relay(&pool, &h).tick().await.unwrap(), 1);
    assert_eq!((published(&pool).await, effects(&pool).await), (1, 1));
}

#[tokio::test]
async fn two_relays_handle_each_event_once() {
    let (pool, _c) = pool().await;
    emit(&pool, 20).await;
    let h = handler(&pool);
    let (a, b) = (relay(&pool, &h), relay(&pool, &h));
    let drain = |r: Relay| async move {
        for _ in 0..50 {
            r.tick().await.unwrap();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::join!(drain(a), drain(b));
    assert_eq!((published(&pool).await, effects(&pool).await), (20, 20));
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        20,
        "each event is handled exactly once"
    );
}

#[test]
fn invalid_schema_is_rejected() {
    for bad in ["a;drop table x", "", "Upper", "1abc", "a-b"] {
        assert!(OutboxTable::new(bad).is_err(), "{bad}");
    }
    assert!(OutboxTable::new("documents").is_ok());
}

async fn emit_poison(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO testmod.outbox VALUES ($1, 'Thing', '{\"poison\": true}'::jsonb, now() - interval '1 second')",
    )
    .bind(Uuid::now_v7())
    .execute(pool)
    .await
    .unwrap();
}

async fn parked(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM platform.outbox_failures WHERE parked_at IS NOT NULL")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn permanent_failure_is_parked_and_later_events_flow() {
    let (pool, _c) = pool().await;
    emit_poison(&pool).await;
    emit(&pool, 1).await;
    let n = relay(&pool, &handler(&pool)).tick().await.unwrap();
    assert_eq!(n, 1, "only the healthy event counts as delivered");
    assert_eq!(
        (
            published(&pool).await,
            parked(&pool).await,
            effects(&pool).await
        ),
        (2, 1, 1)
    );
}

#[tokio::test]
async fn transient_failure_is_parked_after_max_attempts() {
    let (pool, _c) = pool().await;
    emit_poison(&pool).await;
    emit(&pool, 1).await;
    let h = Arc::new(Recording {
        pool: pool.clone(),
        fail: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        poison_is_permanent: false,
    });
    let r = relay(&pool, &h).with_max_attempts(3);
    assert_eq!(r.tick().await.unwrap(), 0);
    assert_eq!(r.tick().await.unwrap(), 0);
    assert_eq!((published(&pool).await, parked(&pool).await), (0, 0));
    assert_eq!(
        r.tick().await.unwrap(),
        1,
        "third failure parks the poison event"
    );
    assert_eq!(
        (
            published(&pool).await,
            parked(&pool).await,
            effects(&pool).await
        ),
        (2, 1, 1)
    );
}
