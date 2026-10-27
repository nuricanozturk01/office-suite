//! Redis pub/sub fan-out of events to every running instance (best-effort, not durable).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use redis::{AsyncCommands, aio::ConnectionManager, aio::PubSub};
use secrecy::{ExposeSecret, SecretString};

use crate::event_bus::EventEnvelope;
use crate::outbox::{BroadcastError, Broadcaster};

const MAX_BACKOFF: Duration = Duration::from_secs(5);

fn channel(event_type: &str) -> String {
    format!("events.{event_type}")
}

fn err(e: &redis::RedisError) -> BroadcastError {
    BroadcastError(e.to_string())
}

/// Publishes envelopes to the Redis channel `events.<event_type>`.
#[derive(Clone)]
pub struct RedisBroadcaster {
    conn: ConnectionManager,
}

impl std::fmt::Debug for RedisBroadcaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisBroadcaster").finish_non_exhaustive()
    }
}

impl RedisBroadcaster {
    /// Connects (and transparently reconnects later).
    ///
    /// # Errors
    /// [`BroadcastError`] when the initial connection fails.
    pub async fn connect(url: &SecretString) -> Result<Self, BroadcastError> {
        let client = redis::Client::open(url.expose_secret()).map_err(|e| err(&e))?;
        let conn = ConnectionManager::new(client).await.map_err(|e| err(&e))?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Broadcaster for RedisBroadcaster {
    async fn publish(&self, event: &EventEnvelope) -> Result<(), BroadcastError> {
        let body = serde_json::to_string(event).map_err(|e| BroadcastError(e.to_string()))?;
        let mut conn = self.conn.clone();
        conn.publish::<_, _, ()>(channel(&event.event_type), body)
            .await
            .map_err(|e| err(&e))
    }
}

/// Runs on every instance for broadcast events (e.g. dropping local WebSocket sessions).
#[async_trait]
pub trait BroadcastHandler: Send + Sync {
    /// Event type this handler reacts to.
    fn event_type(&self) -> &'static str;
    /// Handles one event; must be cheap and idempotent (delivery is best-effort).
    async fn handle(&self, event: &EventEnvelope);
}

/// An established subscription; call [`Listener::run`] to start dispatching.
pub struct Listener {
    client: redis::Client,
    handlers: Vec<Arc<dyn BroadcastHandler>>,
    pubsub: PubSub,
}

impl std::fmt::Debug for Listener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener")
            .field("handlers", &self.handlers.len())
            .finish_non_exhaustive()
    }
}

impl Listener {
    /// Connects and subscribes; messages published after this returns are not missed.
    ///
    /// # Errors
    /// [`BroadcastError`] when Redis is unreachable.
    pub async fn connect(
        url: &SecretString,
        handlers: Vec<Arc<dyn BroadcastHandler>>,
    ) -> Result<Self, BroadcastError> {
        let client = redis::Client::open(url.expose_secret()).map_err(|e| err(&e))?;
        let pubsub = open(&client, &handlers).await.map_err(|e| err(&e))?;
        Ok(Self {
            client,
            handlers,
            pubsub,
        })
    }

    /// Dispatches messages until `shutdown` resolves, reconnecting with capped backoff.
    pub async fn run(self, shutdown: impl Future<Output = ()>) {
        let Self {
            client,
            handlers,
            mut pubsub,
        } = self;
        tokio::pin!(shutdown);
        let mut backoff = Duration::from_millis(200);
        loop {
            {
                let mut stream = pubsub.on_message();
                loop {
                    tokio::select! {
                        () = &mut shutdown => return,
                        msg = stream.next() => match msg {
                            Some(msg) => dispatch(&handlers, msg.get_payload::<String>().ok()).await,
                            None => break,
                        },
                    }
                }
            }
            tracing::warn!("redis subscription lost; reconnecting");
            pubsub = loop {
                tokio::select! {
                    () = &mut shutdown => return,
                    () = tokio::time::sleep(backoff) => {}
                }
                match open(&client, &handlers).await {
                    Ok(ps) => {
                        backoff = Duration::from_millis(200);
                        break ps;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "redis reconnect failed");
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                }
            };
        }
    }
}

async fn open(
    client: &redis::Client,
    handlers: &[Arc<dyn BroadcastHandler>],
) -> redis::RedisResult<PubSub> {
    let mut pubsub = client.get_async_pubsub().await?;
    let mut seen = Vec::new();
    for handler in handlers {
        if !seen.contains(&handler.event_type()) {
            seen.push(handler.event_type());
            pubsub.subscribe(channel(handler.event_type())).await?;
        }
    }
    Ok(pubsub)
}

async fn dispatch(handlers: &[Arc<dyn BroadcastHandler>], payload: Option<String>) {
    let Some(event) = payload.and_then(|p| serde_json::from_str::<EventEnvelope>(&p).ok()) else {
        tracing::warn!("ignoring malformed broadcast message");
        return;
    };
    for handler in handlers
        .iter()
        .filter(|h| h.event_type() == event.event_type)
    {
        handler.handle(&event).await;
    }
}
