//! Outbox relay: delivers `<schema>.outbox` rows to durable handlers (at-least-once) and
//! broadcasts them; consumers dedupe through [`claim`] (CLAUDE.md §4.2.5).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sqlx::{AssertSqlSafe, PgConnection, PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

use crate::event_bus::{EventEnvelope, HandlerError, Subscriptions};

/// Advisory-lock namespace of the relay (first key of the two-key form).
const LOCK_NAMESPACE: i32 = 0x6F62;
/// Upper bound for one handler call, so a hung handler cannot hold the table lock forever.
const HANDLER_TIMEOUT: Duration = Duration::from_secs(30);
/// Upper bound for the best-effort broadcast.
const BROADCAST_TIMEOUT: Duration = Duration::from_secs(2);
/// Default number of failed attempts after which an event is parked as a dead letter.
const DEFAULT_MAX_ATTEMPTS: i32 = 20;

/// Invalid outbox configuration.
#[derive(Debug, Error)]
pub enum OutboxError {
    /// Schema name is not a plain lowercase identifier.
    #[error("invalid schema identifier `{0}`")]
    InvalidSchema(String),
}

/// Best-effort publication of an event to every running instance.
#[derive(Debug, Error)]
#[error("broadcast failed: {0}")]
pub struct BroadcastError(pub String);

/// Publishes events to all instances (e.g. Redis pub/sub). Failures never block delivery.
#[async_trait]
pub trait Broadcaster: Send + Sync {
    /// Publishes `event`.
    ///
    /// # Errors
    /// Returns [`BroadcastError`] when the transport is unavailable.
    async fn publish(&self, event: &EventEnvelope) -> Result<(), BroadcastError>;
}

/// Broadcaster that drops everything (single-instance setups and tests).
#[derive(Debug, Clone, Copy)]
pub struct NoBroadcast;

#[async_trait]
impl Broadcaster for NoBroadcast {
    async fn publish(&self, _event: &EventEnvelope) -> Result<(), BroadcastError> {
        Ok(())
    }
}

/// Records that `consumer` handled `event_id`; returns `false` when it already had.
///
/// Call inside the same transaction as the handler's effect.
///
/// # Errors
/// Returns the underlying [`sqlx::Error`].
pub async fn claim(
    tx: &mut PgConnection,
    consumer: &str,
    event_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query!(
        "INSERT INTO platform.processed_events (consumer, event_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        consumer,
        event_id,
    )
    .execute(tx)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}

/// A validated `<schema>.outbox` table.
#[derive(Debug, Clone)]
pub struct OutboxTable {
    schema: String,
    select_sql: String,
    mark_sql: String,
}

impl OutboxTable {
    /// Accepts `^[a-z][a-z0-9_]{0,62}$` only.
    ///
    /// # Errors
    /// [`OutboxError::InvalidSchema`] for anything else.
    pub fn new(schema: &str) -> Result<Self, OutboxError> {
        let valid = schema.len() <= 63
            && schema.starts_with(|c: char| c.is_ascii_lowercase())
            && schema
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !valid {
            return Err(OutboxError::InvalidSchema(schema.to_owned()));
        }
        // Identifiers cannot be bound as parameters; `schema` is validated above and is never
        // user input (it comes from the composition root).
        Ok(Self {
            schema: schema.to_owned(),
            select_sql: format!(
                "SELECT id, event_type, payload FROM {schema}.outbox
                 WHERE published_at IS NULL ORDER BY created_at, id LIMIT 1"
            ),
            mark_sql: format!("UPDATE {schema}.outbox SET published_at = now() WHERE id = $1"),
        })
    }

    fn lock_key(&self) -> String {
        format!("outbox:{}", self.schema)
    }
}

/// Polls outbox tables and delivers their events.
pub struct Relay {
    pool: PgPool,
    tables: Vec<OutboxTable>,
    subscriptions: Arc<Subscriptions>,
    broadcaster: Arc<dyn Broadcaster>,
    max_attempts: i32,
}

impl std::fmt::Debug for Relay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relay")
            .field("tables", &self.tables)
            .finish_non_exhaustive()
    }
}

impl Relay {
    /// Creates a relay.
    #[must_use]
    pub fn new(
        pool: PgPool,
        tables: Vec<OutboxTable>,
        subscriptions: Arc<Subscriptions>,
        broadcaster: Arc<dyn Broadcaster>,
    ) -> Self {
        Self {
            pool,
            tables,
            subscriptions,
            broadcaster,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }

    /// Overrides after how many failed attempts an event is parked as a dead letter.
    #[must_use]
    pub const fn with_max_attempts(mut self, max_attempts: i32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Delivers pending events; returns how many were published.
    ///
    /// Per table, only one instance works at a time (advisory lock), which keeps delivery
    /// roughly ordered (producer timestamps) and makes concurrent relays safe; the others return
    /// immediately. A failing handler is retried; after `max_attempts` (or on a permanent error)
    /// the event is parked in `platform.outbox_failures` so later events keep flowing.
    ///
    /// # Errors
    /// Returns the underlying [`sqlx::Error`].
    pub async fn tick(&self) -> Result<usize, sqlx::Error> {
        let mut published = 0;
        for table in &self.tables {
            loop {
                match self.publish_next(table).await? {
                    Step::Published => published += 1,
                    Step::Parked => {}
                    Step::Idle | Step::Blocked => break,
                }
            }
        }
        Ok(published)
    }

    async fn publish_next(&self, table: &OutboxTable) -> Result<Step, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1, hashtext($2))")
            .bind(LOCK_NAMESPACE)
            .bind(table.lock_key())
            .fetch_one(&mut *tx)
            .await?;
        if !locked {
            return Ok(Step::Idle);
        }
        let Some(row) = sqlx::query(AssertSqlSafe(table.select_sql.as_str()))
            .fetch_optional(&mut *tx)
            .await?
        else {
            return Ok(Step::Idle);
        };
        let event = EventEnvelope {
            id: row.try_get("id")?,
            event_type: row.try_get("event_type")?,
            payload: row.try_get("payload")?,
        };
        let mut step = Step::Published;
        for handler in self.subscriptions.for_type(&event.event_type) {
            let outcome = tokio::time::timeout(HANDLER_TIMEOUT, handler.handle(&event))
                .await
                .unwrap_or_else(|_| Err(HandlerError::Transient("handler timed out".to_owned())));
            if let Err(err) = outcome {
                tracing::error!(
                    event_id = %event.id,
                    event_type = %event.event_type,
                    consumer = handler.consumer(),
                    error = %err,
                    "handler failed"
                );
                let permanent = matches!(err, HandlerError::Permanent(_));
                let attempts = record_failure(
                    &mut tx,
                    &table.schema,
                    event.id,
                    handler.consumer(),
                    &err.to_string(),
                )
                .await?;
                if !permanent && attempts < self.max_attempts {
                    // Keep the row at the head and retry on the next tick; handler effects live
                    // in their own transactions, so nothing needs rolling back here.
                    tx.commit().await?;
                    return Ok(Step::Blocked);
                }
                park(&mut tx, &table.schema, event.id).await?;
                step = Step::Parked;
                break;
            }
        }
        sqlx::query(AssertSqlSafe(table.mark_sql.as_str()))
            .bind(event.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        // Best effort and outside the lock: a slow or absent Redis must not stall delivery.
        if step == Step::Published {
            match tokio::time::timeout(BROADCAST_TIMEOUT, self.broadcaster.publish(&event)).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    tracing::warn!(event_id = %event.id, error = %err, "broadcast failed");
                }
                Err(_) => tracing::warn!(event_id = %event.id, "broadcast timed out"),
            }
        }
        Ok(step)
    }

    /// Ticks every `interval` until `shutdown` resolves.
    pub async fn run(self, interval: Duration, shutdown: impl Future<Output = ()>) {
        tokio::pin!(shutdown);
        loop {
            if let Err(err) = self.tick().await {
                tracing::error!(error = %err, "outbox tick failed");
            }
            tokio::select! {
                () = &mut shutdown => return,
                () = tokio::time::sleep(interval) => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    Published,
    Parked,
    Idle,
    Blocked,
}

async fn record_failure(
    tx: &mut PgConnection,
    outbox: &str,
    event_id: Uuid,
    consumer: &str,
    error: &str,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query!(
        "INSERT INTO platform.outbox_failures (outbox, event_id, consumer, attempts, last_error)
         VALUES ($1, $2, $3, 1, $4)
         ON CONFLICT (outbox, event_id) DO UPDATE
         SET attempts = platform.outbox_failures.attempts + 1,
             consumer = EXCLUDED.consumer,
             last_error = EXCLUDED.last_error,
             updated_at = now()
         RETURNING attempts",
        outbox,
        event_id,
        consumer,
        error,
    )
    .fetch_one(tx)
    .await?;
    Ok(row.attempts)
}

async fn park(tx: &mut PgConnection, outbox: &str, event_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE platform.outbox_failures SET parked_at = now()
         WHERE outbox = $1 AND event_id = $2",
        outbox,
        event_id,
    )
    .execute(tx)
    .await?;
    tracing::error!(%event_id, outbox, "event parked as dead letter");
    Ok(())
}
