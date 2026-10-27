use async_trait::async_trait;
use documents_contracts::{DocumentId, OwnerId};
use platform::pagination::UpdatedCursor;
use thiserror::Error;

use crate::domain::Document;

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
    pub(crate) next_cursor: Option<UpdatedCursor>,
}

/// Persistence port for the document aggregate. Saving also writes domain events to the
/// outbox in the same transaction.
#[async_trait]
pub(crate) trait DocumentRepository: Send + Sync {
    /// Inserts a new document and its pending events atomically.
    async fn insert(&self, doc: &mut Document) -> Result<(), RepositoryError>;
    /// Loads a non-deleted document.
    async fn find(&self, id: DocumentId) -> Result<Option<Document>, RepositoryError>;
    /// Persists title/timestamps of an existing document and its pending events atomically.
    async fn update(&self, doc: &mut Document) -> Result<(), RepositoryError>;
    /// Soft-deletes a document and writes its pending events atomically.
    async fn delete(&self, doc: &mut Document) -> Result<(), RepositoryError>;
    /// Lists an owner's documents, most recently updated first, after `cursor` (exclusive).
    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<Document>, RepositoryError>;
}

/// What the caller wants to do with a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Read,
    Write,
    Delete,
}

/// Authorization port, backed by the workspace module's `PermissionChecker` (CLAUDE.md §5.4).
#[async_trait]
pub(crate) trait AccessPolicy: Send + Sync {
    /// Whether `actor` may perform `action` on `doc`.
    async fn allows(&self, actor: OwnerId, doc: &Document, action: Action) -> bool;
}
