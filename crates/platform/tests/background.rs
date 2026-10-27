//! Two event runtimes (two instances) on one Postgres and one Redis.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use platform::{
    background::{EventRuntime, EventRuntimeConfig},
    broadcast::BroadcastHandler,
    event_bus::{EventEnvelope, EventHandler, HandlerError, Subscriptions},
    outbox::claim,
};
use secrecy::SecretString;
use sqlx::PgPool;
use testcontainers_modules::{
    postgres::Postgres, redis::Redis, testcontainers::runners::AsyncRunner,
};
use uuid::Uuid;

struct Count(Arc<AtomicUsize>, PgPool);

#[async_trait]
impl EventHandler for Count {
    fn consumer(&self) -> &'static str {
        "count"
    }
    fn event_type(&self) -> &'static str {
        "Thing"
    }
    async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError> {
        let io = |e: sqlx::Error| HandlerError::Transient(e.to_string());
        let mut tx = self.1.begin().await.map_err(io)?;
        let first = claim(&mut tx, "count", event.id).await.map_err(io)?;
        tx.commit().await.map_err(io)?;
        if first {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

struct Seen(Arc<AtomicUsize>);

#[async_trait]
impl BroadcastHandler for Seen {
    fn event_type(&self) -> &'static str {
        "Thing"
    }
    async fn handle(&self, _event: &EventEnvelope) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn two_instances_apply_once_and_broadcast_to_both() {
    let pg = Postgres::default().start().await.unwrap();
    let rd = Redis::default().start().await.unwrap();
    let pg_port = pg.get_host_port_ipv4(5432).await.unwrap();
    let rd_port = rd.get_host_port_ipv4(6379).await.unwrap();
    let pool = platform::db::connect(&SecretString::from(format!(
        "postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres"
    )))
    .await
    .unwrap();
    platform::db::migrate(&pool, &platform::migrator())
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE SCHEMA testmod;
         CREATE TABLE testmod.outbox (
             id uuid PRIMARY KEY, event_type text NOT NULL, payload jsonb NOT NULL,
             created_at timestamptz NOT NULL, published_at timestamptz);",
    )
    .execute(&pool)
    .await
    .unwrap();

    let applied = Arc::new(AtomicUsize::new(0));
    let broadcasts = Arc::new(AtomicUsize::new(0));
    let mut runtimes = Vec::new();
    for _ in 0..2 {
        let mut subscriptions = Subscriptions::new();
        subscriptions.add(Arc::new(Count(applied.clone(), pool.clone())));
        runtimes.push(
            EventRuntime::start(EventRuntimeConfig {
                pool: pool.clone(),
                redis_url: SecretString::from(format!("redis://127.0.0.1:{rd_port}")),
                outbox_schemas: vec!["testmod"],
                subscriptions,
                broadcast_handlers: vec![Arc::new(Seen(broadcasts.clone()))],
                poll_interval: Duration::from_millis(50),
            })
            .await
            .unwrap(),
        );
    }

    sqlx::query("INSERT INTO testmod.outbox VALUES ($1, 'Thing', '{}'::jsonb, now())")
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await
        .unwrap();

    for _ in 0..100 {
        let done: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM testmod.outbox WHERE published_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if done == 1 && broadcasts.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        applied.load(Ordering::SeqCst),
        1,
        "durable handler runs once cluster-wide"
    );
    assert_eq!(
        broadcasts.load(Ordering::SeqCst),
        2,
        "broadcast handler runs on every instance"
    );
    for runtime in runtimes {
        tokio::time::timeout(Duration::from_secs(5), runtime.stop())
            .await
            .unwrap();
    }
}
