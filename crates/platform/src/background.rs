//! Event runtime: outbox relay plus Redis listener, started by each instance.

use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretString;
use sqlx::PgPool;
use thiserror::Error;
use tokio::{sync::watch, task::JoinSet};

use crate::broadcast::{BroadcastHandler, Listener, RedisBroadcaster};
use crate::event_bus::Subscriptions;
use crate::outbox::{BroadcastError, OutboxError, OutboxTable, Relay};

/// Startup failure of the event runtime.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Bad outbox schema name.
    #[error(transparent)]
    Outbox(#[from] OutboxError),
    /// Redis unreachable at startup (fail-fast; multi-instance needs it).
    #[error(transparent)]
    Redis(#[from] BroadcastError),
}

/// Inputs of [`EventRuntime::start`].
pub struct EventRuntimeConfig {
    /// Shared Postgres pool.
    pub pool: PgPool,
    /// Redis connection URL.
    pub redis_url: SecretString,
    /// Module schemas whose `<schema>.outbox` tables are relayed.
    pub outbox_schemas: Vec<&'static str>,
    /// Durable (once cluster-wide) handlers.
    pub subscriptions: Subscriptions,
    /// Broadcast (every instance) handlers.
    pub broadcast_handlers: Vec<Arc<dyn BroadcastHandler>>,
    /// Outbox polling interval.
    pub poll_interval: Duration,
}

impl std::fmt::Debug for EventRuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventRuntimeConfig")
            .field("outbox_schemas", &self.outbox_schemas)
            .field("poll_interval", &self.poll_interval)
            .finish_non_exhaustive()
    }
}

/// Running relay and listener tasks.
#[derive(Debug)]
pub struct EventRuntime {
    tasks: JoinSet<()>,
    shutdown: watch::Sender<bool>,
}

async fn wait(mut rx: watch::Receiver<bool>) {
    // A closed channel also means shutdown.
    let _ = rx.wait_for(|stop| *stop).await;
}

impl EventRuntime {
    /// Connects to Redis and spawns the relay and (when handlers exist) the listener.
    ///
    /// # Errors
    /// [`RuntimeError`] for invalid schema names or an unreachable Redis.
    pub async fn start(config: EventRuntimeConfig) -> Result<Self, RuntimeError> {
        let tables = config
            .outbox_schemas
            .iter()
            .map(|schema| OutboxTable::new(schema))
            .collect::<Result<Vec<_>, _>>()?;
        let broadcaster = RedisBroadcaster::connect(&config.redis_url).await?;
        let (shutdown, rx) = watch::channel(false);
        let mut tasks = JoinSet::new();
        let relay = Relay::new(
            config.pool,
            tables,
            Arc::new(config.subscriptions),
            Arc::new(broadcaster),
        );
        tasks.spawn(relay.run(config.poll_interval, wait(rx.clone())));
        if !config.broadcast_handlers.is_empty() {
            let listener = Listener::connect(&config.redis_url, config.broadcast_handlers).await?;
            tasks.spawn(listener.run(wait(rx)));
        }
        Ok(Self { tasks, shutdown })
    }

    /// Signals shutdown and waits for the tasks to finish.
    pub async fn stop(mut self) {
        let _ = self.shutdown.send(true);
        while self.tasks.join_next().await.is_some() {}
    }
}
