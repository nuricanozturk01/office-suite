//! Raw per-channel Redis pub/sub with dynamic subscriptions (used for realtime fan-out).
//!
//! Delivery is best-effort: messages published while a connection is down are lost, so callers
//! must be able to heal from the durable store.

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};

use futures_util::StreamExt;
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, PubSubSink, PubSubStream},
};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::{Mutex, broadcast};

use crate::outbox::BroadcastError;

const CHANNEL_CAPACITY: usize = 256;
const MIN_BACKOFF: Duration = Duration::from_millis(200);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

fn err(e: &redis::RedisError) -> BroadcastError {
    BroadcastError(e.to_string())
}

/// Why [`Subscription::recv`] returned no message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvError {
    /// The subscriber fell behind and missed this many messages.
    Lagged(u64),
    /// The bus was dropped.
    Closed,
}

struct Inner {
    sink: PubSubSink,
    routes: HashMap<String, broadcast::Sender<Arc<[u8]>>>,
}

/// Publishes to and subscribes from Redis channels; shared by all rooms of an instance.
pub struct DocBus {
    client: redis::Client,
    publisher: ConnectionManager,
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for DocBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocBus").finish_non_exhaustive()
    }
}

impl DocBus {
    /// Connects the publisher and the subscriber connection and starts the reader task.
    ///
    /// # Errors
    /// [`BroadcastError`] when Redis is unreachable.
    pub async fn connect(url: &SecretString) -> Result<Arc<Self>, BroadcastError> {
        let client = redis::Client::open(url.expose_secret()).map_err(|e| err(&e))?;
        let publisher = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| err(&e))?;
        let (sink, stream) = client
            .get_async_pubsub()
            .await
            .map_err(|e| err(&e))?
            .split();
        let bus = Arc::new(Self {
            client,
            publisher,
            inner: Mutex::new(Inner {
                sink,
                routes: HashMap::new(),
            }),
        });
        tokio::spawn(read_loop(Arc::downgrade(&bus), stream));
        Ok(bus)
    }

    /// Publishes `payload` to `channel`.
    ///
    /// # Errors
    /// [`BroadcastError`] when Redis rejects or cannot be reached.
    pub async fn publish(&self, channel: &str, payload: Vec<u8>) -> Result<(), BroadcastError> {
        let mut conn = self.publisher.clone();
        conn.publish::<_, _, ()>(channel, payload)
            .await
            .map_err(|e| err(&e))
    }

    /// Subscribes to `channel`; the Redis subscription lives while any [`Subscription`] does.
    ///
    /// # Errors
    /// [`BroadcastError`] when Redis rejects the subscription.
    pub async fn subscribe(
        self: &Arc<Self>,
        channel: &str,
    ) -> Result<Subscription, BroadcastError> {
        let mut inner = self.inner.lock().await;
        let rx = if let Some(tx) = inner.routes.get(channel) {
            tx.subscribe()
        } else {
            inner.sink.subscribe(channel).await.map_err(|e| err(&e))?;
            let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
            inner.routes.insert(channel.to_owned(), tx);
            rx
        };
        drop(inner);
        Ok(Subscription {
            rx: Some(rx),
            channel: channel.to_owned(),
            bus: Arc::clone(self),
        })
    }

    /// Drops the Redis subscription of `channel` when nobody listens locally anymore.
    async fn release(&self, channel: &str) {
        let mut inner = self.inner.lock().await;
        if inner
            .routes
            .get(channel)
            .is_some_and(|tx| tx.receiver_count() == 0)
        {
            inner.routes.remove(channel);
            if let Err(e) = inner.sink.unsubscribe(channel).await {
                tracing::warn!(channel, error = %e, "redis unsubscribe failed");
            }
        }
    }
}

/// A live subscription to one channel.
pub struct Subscription {
    rx: Option<broadcast::Receiver<Arc<[u8]>>>,
    channel: String,
    bus: Arc<DocBus>,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("channel", &self.channel)
            .finish_non_exhaustive()
    }
}

impl Subscription {
    /// Waits for the next message.
    ///
    /// # Errors
    /// [`RecvError::Lagged`] when messages were missed, [`RecvError::Closed`] when the bus is gone.
    pub async fn recv(&mut self) -> Result<Arc<[u8]>, RecvError> {
        let Some(rx) = self.rx.as_mut() else {
            return Err(RecvError::Closed);
        };
        rx.recv().await.map_err(|e| match e {
            broadcast::error::RecvError::Lagged(n) => RecvError::Lagged(n),
            broadcast::error::RecvError::Closed => RecvError::Closed,
        })
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        drop(self.rx.take());
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let bus = Arc::clone(&self.bus);
            let channel = std::mem::take(&mut self.channel);
            handle.spawn(async move { bus.release(&channel).await });
        }
    }
}

/// Routes incoming messages to local subscribers and re-establishes the connection on loss.
async fn read_loop(bus: Weak<DocBus>, mut stream: PubSubStream) {
    let mut backoff = MIN_BACKOFF;
    loop {
        while let Some(msg) = stream.next().await {
            let Some(bus) = bus.upgrade() else {
                return;
            };
            let inner = bus.inner.lock().await;
            if let Some(tx) = inner.routes.get(msg.get_channel_name()) {
                let _ = tx.send(Arc::from(msg.get_payload_bytes()));
            }
        }
        tracing::warn!("redis doc bus connection lost; reconnecting");
        loop {
            tokio::time::sleep(backoff).await;
            let Some(strong) = bus.upgrade() else {
                return;
            };
            match resubscribe(&strong).await {
                Ok(new_stream) => {
                    stream = new_stream;
                    backoff = MIN_BACKOFF;
                    break;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "redis doc bus reconnect failed");
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
    }
}

async fn resubscribe(bus: &DocBus) -> redis::RedisResult<PubSubStream> {
    let (mut sink, stream) = bus.client.get_async_pubsub().await?.split();
    let mut inner = bus.inner.lock().await;
    for channel in inner.routes.keys() {
        sink.subscribe(channel).await?;
    }
    inner.sink = sink;
    drop(inner);
    Ok(stream)
}
