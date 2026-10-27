use async_trait::async_trait;
use identity_contracts::UserId;
use serde_json::Value;
use shared_kernel::Uuid;
use thiserror::Error;
use time::OffsetDateTime;
use workspace_contracts::{ResourceId, ResourceKind, Role};

use crate::domain::Share;

/// Persistence failure.
#[derive(Debug, Error)]
pub(crate) enum RepositoryError {
    /// Infrastructure failure (details are logged, not exposed).
    #[error("storage failure")]
    Storage,
}

/// A page of results with a cursor for the next page.
#[derive(Debug)]
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) next_cursor: Option<ResourceId>,
}

/// A known, non-deleted resource as mirrored from its owning module.
#[derive(Debug, Clone)]
pub(crate) struct ResourceInfo {
    pub(crate) owner_id: UserId,
}

/// One grant on a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShareRecord {
    pub(crate) grantee: UserId,
    /// Normalized email the owner shared with (display label).
    pub(crate) email: String,
    pub(crate) role: Role,
    pub(crate) shared_by: UserId,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

/// A resource shared with a user.
#[derive(Debug, Clone)]
pub(crate) struct SharedItem {
    pub(crate) id: ResourceId,
    pub(crate) kind: ResourceKind,
    pub(crate) title: String,
    pub(crate) role: Role,
    pub(crate) owner_id: UserId,
    pub(crate) shared_at: OffsetDateTime,
}

/// An integration event to write to the outbox in the same transaction as the change.
#[derive(Debug, Clone)]
pub(crate) struct OutboxEvent {
    pub(crate) event_type: &'static str,
    pub(crate) payload: Value,
    pub(crate) at: OffsetDateTime,
}

/// Persistence port for shares. Writes also append `events` to the outbox atomically.
#[async_trait]
pub(crate) trait WorkspaceRepository: Send + Sync {
    /// The resource, unless unknown or deleted.
    async fn find_resource(&self, id: ResourceId) -> Result<Option<ResourceInfo>, RepositoryError>;
    /// Inserts or updates a share in one transaction. The previous role is read under a row
    /// lock and handed to `events_for`, whose events are appended to the outbox; returns the
    /// stored row.
    async fn upsert_share(
        &self,
        share: &Share,
        email: &str,
        now: OffsetDateTime,
        events_for: &(
             dyn Fn(Option<Role>) -> Result<Vec<OutboxEvent>, RepositoryError> + Send + Sync
         ),
    ) -> Result<ShareRecord, RepositoryError>;
    /// Removes a share and appends `events` when it existed; returns whether it existed.
    async fn delete_share(
        &self,
        resource: ResourceId,
        grantee: UserId,
        events: &[OutboxEvent],
    ) -> Result<bool, RepositoryError>;
    /// All shares of a resource, oldest first.
    async fn list_shares(&self, resource: ResourceId) -> Result<Vec<ShareRecord>, RepositoryError>;
    /// Resources shared with `user`, newest resource first, after `cursor` (exclusive).
    async fn shared_with_me(
        &self,
        user: UserId,
        cursor: Option<ResourceId>,
        limit: u32,
    ) -> Result<Page<SharedItem>, RepositoryError>;
}

/// A change to the resource read-model derived from another module's event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResourceChange {
    Created {
        id: ResourceId,
        kind: ResourceKind,
        owner: UserId,
        title: String,
        at: OffsetDateTime,
    },
    Renamed {
        id: ResourceId,
        title: String,
        at: OffsetDateTime,
    },
    Deleted {
        id: ResourceId,
        at: OffsetDateTime,
    },
}

/// Applies read-model changes idempotently: `event_id` is claimed per `consumer` in the same
/// transaction as the change, and replays or reordering must leave the model correct.
#[async_trait]
pub(crate) trait ResourceStore: Send + Sync {
    /// Applies `change` once per `(consumer, event_id)`.
    async fn apply(
        &self,
        consumer: &'static str,
        event_id: Uuid,
        change: ResourceChange,
    ) -> Result<(), RepositoryError>;
}
