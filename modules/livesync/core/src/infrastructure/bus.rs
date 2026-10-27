//! Cross-instance bus implementations.

use std::sync::Arc;

use async_trait::async_trait;
use platform::docbus::{DocBus, RecvError, Subscription};
use workspace_contracts::ResourceId;

use crate::application::{BusError, BusMessage, BusStream, LiveBus};

/// Single-instance setups and tests: nothing is published, nothing arrives.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoBus;

struct Silent;

#[async_trait]
impl BusStream for Silent {
    async fn next(&mut self) -> Option<BusMessage> {
        std::future::pending().await
    }
}

#[async_trait]
impl LiveBus for NoBus {
    async fn publish(&self, _resource: ResourceId, _message: BusMessage) {}

    async fn subscribe(&self, _resource: ResourceId) -> Result<Box<dyn BusStream>, BusError> {
        Ok(Box::new(Silent))
    }
}

/// Redis pub/sub bus, channel `livesync.<resource_id>`, JSON bodies.
pub(crate) struct RedisBus {
    bus: Arc<DocBus>,
}

impl RedisBus {
    pub(crate) const fn new(bus: Arc<DocBus>) -> Self {
        Self { bus }
    }
}

fn channel(resource: ResourceId) -> String {
    format!("livesync.{}", resource.as_uuid())
}

struct RedisStream(Subscription);

#[async_trait]
impl BusStream for RedisStream {
    async fn next(&mut self) -> Option<BusMessage> {
        loop {
            match self.0.recv().await {
                Ok(raw) => {
                    if let Ok(message) = serde_json::from_slice(&raw) {
                        return Some(message);
                    }
                    tracing::warn!("ignoring malformed live bus message");
                }
                // Missed messages heal with the next heartbeat.
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    }
}

#[async_trait]
impl LiveBus for RedisBus {
    async fn publish(&self, resource: ResourceId, message: BusMessage) {
        let Ok(body) = serde_json::to_vec(&message) else {
            return;
        };
        if let Err(err) = self.bus.publish(&channel(resource), body).await {
            tracing::warn!(%resource, error = %err, "live bus publish failed");
        }
    }

    async fn subscribe(&self, resource: ResourceId) -> Result<Box<dyn BusStream>, BusError> {
        let subscription = self
            .bus
            .subscribe(&channel(resource))
            .await
            .map_err(|err| {
                tracing::error!(%resource, error = %err, "live bus subscribe failed");
                BusError
            })?;
        Ok(Box::new(RedisStream(subscription)))
    }
}
