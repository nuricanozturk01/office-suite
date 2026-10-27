//! Public surface of the `pdfs` module: IDs and integration events only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;

typed_id!(
    /// Identifier of a pdf.
    PdfId
);
typed_id!(
    /// Identifier of a user, as referenced by pdfs.
    OwnerId
);

/// Emitted (via outbox) after a pdf is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfCreated {
    /// Created pdf.
    pub pdf_id: PdfId,
    /// Owning user.
    pub owner_id: OwnerId,
    /// Title at creation time.
    pub title: String,
    /// Creation time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Emitted (via outbox) after a pdf is renamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfRenamed {
    /// Renamed pdf.
    pub pdf_id: PdfId,
    /// New title.
    pub title: String,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Emitted (via outbox) after a pdf is deleted (soft delete).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfDeleted {
    /// Deleted pdf.
    pub pdf_id: PdfId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}
