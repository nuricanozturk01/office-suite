//! Public surface of the `documents` module: IDs and integration events only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;

typed_id!(
    /// Identifier of a document.
    DocumentId
);
typed_id!(
    /// Identifier of a user, as referenced by documents.
    OwnerId
);

/// Emitted (via outbox) after a document is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCreated {
    /// Created document.
    pub document_id: DocumentId,
    /// Owning user.
    pub owner_id: OwnerId,
    /// Title at creation time.
    pub title: String,
    /// Creation time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Emitted (via outbox) after a document is renamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRenamed {
    /// Renamed document.
    pub document_id: DocumentId,
    /// New title.
    pub title: String,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Emitted (via outbox) after a document is deleted (soft delete).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDeleted {
    /// Deleted document.
    pub document_id: DocumentId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// How much a user may do with a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessLevel {
    /// No access (or the document does not exist).
    None,
    /// May read content.
    Read,
    /// May read and change content.
    Write,
}

/// Synchronous-style port (async call): resolves a user's access to a document.
///
/// Implemented by the `documents` module; consumed by modules that act on document
/// content (e.g. `collaboration`).
#[async_trait::async_trait]
pub trait DocumentAccess: Send + Sync + std::fmt::Debug {
    /// Access level of `user` on `document`; [`AccessLevel::None`] when missing or denied.
    async fn access_level(&self, user: OwnerId, document: DocumentId) -> AccessLevel;
}
