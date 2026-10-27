use async_trait::async_trait;
use platform::pagination::UpdatedCursor;
use presentations_contracts::{OwnerId, PresentationId};
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;

use crate::domain::Presentation;

/// Persistence failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum RepositoryError {
    /// Infrastructure failure (details are logged, not exposed).
    #[error("storage failure")]
    Storage,
    /// Content version did not match (a concurrent write won).
    #[error("version conflict")]
    Conflict,
}

/// Listing projection: no deck payload.
#[derive(Debug)]
pub(crate) struct PresentationSummary {
    pub(crate) id: PresentationId,
    pub(crate) title: crate::domain::PresentationTitle,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) updated_at: OffsetDateTime,
}

/// A page of results with a cursor for the next page.
#[derive(Debug)]
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) next_cursor: Option<UpdatedCursor>,
}

/// Persistence port. Inserts also write domain events to the outbox in the same transaction.
#[async_trait]
pub(crate) trait PresentationRepository: Send + Sync {
    /// Inserts a new presentation and its pending events atomically.
    async fn insert(&self, presentation: &mut Presentation) -> Result<(), RepositoryError>;
    /// Loads a non-deleted presentation, including its deck.
    async fn find(&self, id: PresentationId) -> Result<Option<Presentation>, RepositoryError>;
    /// Persists a renamed title and its pending events atomically. Does not change `version`.
    async fn update_title(&self, presentation: &mut Presentation) -> Result<(), RepositoryError>;
    /// Replaces the deck when `expected_version` still matches. Returns the new version.
    async fn save_content(
        &self,
        id: PresentationId,
        content: &Value,
        expected_version: i64,
        now: OffsetDateTime,
    ) -> Result<i64, RepositoryError>;
    /// Soft-deletes a presentation and writes its pending events atomically.
    async fn delete(&self, presentation: &mut Presentation) -> Result<(), RepositoryError>;
    /// Lists an owner's presentations, newest first, after `cursor` (exclusive).
    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<PresentationSummary>, RepositoryError>;
}

/// What the caller wants to do with a presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Read,
    Write,
    Delete,
}

/// Authorization port, backed by the workspace module's `PermissionChecker` (CLAUDE.md §5.4).
#[async_trait]
pub(crate) trait AccessPolicy: Send + Sync {
    /// Whether `actor` may perform `action` on `presentation`.
    async fn allows(&self, actor: OwnerId, presentation: &Presentation, action: Action) -> bool;
}
