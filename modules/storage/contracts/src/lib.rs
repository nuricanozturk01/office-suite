//! Public surface of the `storage` module: limits and integration events.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use identity_contracts::UserId;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use workspace_contracts::ResourceId;

/// Largest file package (`.docx`, `.xlsx`, `.pptx`, `.pdf`) the module stores, in bytes.
pub const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;

/// Emitted (via outbox) after the file behind a resource is saved.
///
/// The resource modules use it to move the resource to the top of "last edited" lists, and
/// `livesync` tells everyone who has the file open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSaved {
    /// The document, spreadsheet, presentation or PDF whose file was saved.
    pub resource_id: ResourceId,
    /// Package version after the save.
    pub version: i64,
    /// Who saved it; `None` on events written before this field existed.
    #[serde(default)]
    pub by: Option<UserId>,
    /// Time of the save (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Why two edits of one file could not be combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MergeError {
    /// Both sides changed the same part of the file.
    #[error("both sides changed the same part")]
    Conflict,
    /// One of the three files is not a valid file of this format.
    #[error("file could not be read")]
    Unreadable,
}

/// Combines two concurrent edits of a file with the version both were based on.
///
/// Implemented per file format by the resource modules' engine crates and injected into
/// `storage` at the composition root. Pure CPU work: callers run it off the async executor.
pub trait PackageMerger: Send + Sync + std::fmt::Debug {
    /// Whether this merger understands files with the given name (by extension).
    fn handles(&self, name: &str) -> bool;

    /// Returns `ours` plus the changes `theirs` made to `base`, as file bytes.
    ///
    /// # Errors
    /// [`MergeError::Conflict`] when both sides changed the same part;
    /// [`MergeError::Unreadable`] when a file cannot be parsed.
    fn merge(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>, MergeError>;
}
