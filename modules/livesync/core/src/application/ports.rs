use async_trait::async_trait;
use livesync_contracts::Participant;
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;
use thiserror::Error;
use workspace_contracts::ResourceId;

/// The bus could not be reached.
#[derive(Debug, Error)]
#[error("bus unavailable")]
pub(crate) struct BusError;

/// What instances tell each other about one room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BusEvent {
    /// A connection opened on the sending instance.
    Joined { participant: Participant },
    /// A connection closed on the sending instance.
    Left { session: Uuid },
    /// A connection of the sending instance started or stopped editing.
    Activity { session: Uuid, editing: bool },
    /// Everybody connected to the sending instance; repeated so that late instances catch up and
    /// lost messages heal.
    Presence { participants: Vec<Participant> },
}

/// A bus message with its sender, so an instance can ignore its own echoes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BusMessage {
    pub(crate) instance: Uuid,
    pub(crate) event: BusEvent,
}

/// Messages of other instances for one room.
#[async_trait]
pub(crate) trait BusStream: Send {
    /// Next message, or `None` when the bus is gone (presence then relies on the heartbeat).
    async fn next(&mut self) -> Option<BusMessage>;
}

/// Best-effort fan-out between instances; correctness never depends on it.
#[async_trait]
pub(crate) trait LiveBus: Send + Sync {
    /// Publishes `message` for `resource`; failures are logged, never surfaced to clients.
    async fn publish(&self, resource: ResourceId, message: BusMessage);
    /// Subscribes to `resource`; messages published after this returns are not missed.
    async fn subscribe(&self, resource: ResourceId) -> Result<Box<dyn BusStream>, BusError>;
}
