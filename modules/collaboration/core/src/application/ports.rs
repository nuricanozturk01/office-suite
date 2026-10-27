use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::DocumentId;
use shared_kernel::Uuid;
use thiserror::Error;

/// Persistence failure (details are logged, not exposed).
#[derive(Debug, Error)]
#[error("storage failure")]
pub(crate) struct StoreError;

/// Persisted state of a document: optional snapshot plus updates after it, in order.
#[derive(Debug, Default)]
pub(crate) struct Loaded {
    pub(crate) snapshot: Option<Vec<u8>>,
    /// Highest sequence number folded into `snapshot` (0 without one).
    pub(crate) snapshot_seq: i64,
    pub(crate) updates: Vec<(i64, Vec<u8>)>,
}

/// Durable, append-only update log with snapshot compaction.
#[async_trait]
pub(crate) trait UpdateStore: Send + Sync {
    /// Loads the snapshot and all later updates.
    async fn load(&self, document: DocumentId) -> Result<Loaded, StoreError>;
    /// Appends an update and returns its sequence number.
    async fn append(&self, document: DocumentId, payload: &[u8]) -> Result<i64, StoreError>;
    /// Updates of `document` with `seq > after`, in order.
    async fn updates_after(
        &self,
        document: DocumentId,
        after: i64,
    ) -> Result<Vec<(i64, Vec<u8>)>, StoreError>;
    /// The snapshot, when it covers more than `after` (i.e. rows up to its `upto_seq` may no longer
    /// exist in the log).
    async fn snapshot_after(
        &self,
        document: DocumentId,
        after: i64,
    ) -> Result<Option<(i64, Vec<u8>)>, StoreError>;
    /// Folds the durable log into the snapshot and drops the folded rows. The result is derived
    /// from the stored log under a per-document lock (never from any instance's memory), so
    /// concurrent instances cannot lose each other's updates. Returns `Ok(())` without work when
    /// another instance is already compacting this document.
    async fn compact(&self, document: DocumentId) -> Result<(), StoreError>;
}

/// What a bus message carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BusKind {
    /// A document update that was appended to the log as `seq`; `bytes` is the raw update.
    Update { seq: i64 },
    /// A complete awareness frame, relayed verbatim.
    Awareness,
}

/// A message exchanged between instances for one document.
#[derive(Debug, Clone)]
pub(crate) struct BusMessage {
    /// Instance that published the message (used to ignore own echoes).
    pub(crate) instance: Uuid,
    pub(crate) kind: BusKind,
    pub(crate) bytes: Arc<[u8]>,
}

/// Messages from other instances for one document.
#[async_trait]
pub(crate) trait BusStream: Send {
    /// Next message, or `None` when the bus is gone (callers then rely on the DB catch-up).
    async fn next(&mut self) -> Option<BusMessage>;
}

/// Best-effort fan-out between instances; correctness never depends on it.
#[async_trait]
pub(crate) trait UpdateBus: Send + Sync {
    /// Publishes `message` for `document`; failures are logged, never surfaced to editors.
    async fn publish(&self, document: DocumentId, message: BusMessage);
    /// Subscribes to `document` (messages published after this returns are not missed).
    async fn subscribe(&self, document: DocumentId) -> Result<Box<dyn BusStream>, StoreError>;
}
