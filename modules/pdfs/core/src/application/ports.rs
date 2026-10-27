use async_trait::async_trait;
use pdfs_contracts::{OwnerId, PdfId};
use platform::pagination::UpdatedCursor;
use thiserror::Error;

use crate::domain::Pdf;

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

/// Persistence port for the pdf aggregate. Saving also writes domain events to the
/// outbox in the same transaction.
#[async_trait]
pub(crate) trait PdfRepository: Send + Sync {
    /// Inserts a new pdf and its pending events atomically.
    async fn insert(&self, doc: &mut Pdf) -> Result<(), RepositoryError>;
    /// Loads a non-deleted pdf.
    async fn find(&self, id: PdfId) -> Result<Option<Pdf>, RepositoryError>;
    /// Persists title/timestamps of an existing pdf and its pending events atomically.
    async fn update(&self, doc: &mut Pdf) -> Result<(), RepositoryError>;
    /// Soft-deletes a pdf and writes its pending events atomically.
    async fn delete(&self, doc: &mut Pdf) -> Result<(), RepositoryError>;
    /// Lists an owner's pdfs, newest first, after `cursor` (exclusive).
    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<Pdf>, RepositoryError>;
}

/// What the caller wants to do with a pdf.
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
    async fn allows(&self, actor: OwnerId, doc: &Pdf, action: Action) -> bool;
}
