//! Public surface of the `presentations` module: IDs and integration events only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;

typed_id!(
    /// Identifier of a presentation.
    PresentationId
);
typed_id!(
    /// Identifier of a user, as referenced by presentations.
    OwnerId
);

/// Emitted (via outbox) after a presentation is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationCreated {
    /// Created presentation.
    pub presentation_id: PresentationId,
    /// Owning user.
    pub owner_id: OwnerId,
    /// Title at creation time.
    pub title: String,
    /// Creation time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Emitted (via outbox) after a presentation is renamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationRenamed {
    /// Renamed presentation.
    pub presentation_id: PresentationId,
    /// New title.
    pub title: String,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Emitted (via outbox) after a presentation is deleted (soft delete).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationDeleted {
    /// Deleted presentation.
    pub presentation_id: PresentationId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}
