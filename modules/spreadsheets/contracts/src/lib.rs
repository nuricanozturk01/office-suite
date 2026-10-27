//! Public surface of the `spreadsheets` module: IDs and integration events only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;

typed_id!(
    /// Identifier of a spreadsheet.
    SpreadsheetId
);
typed_id!(
    /// Identifier of a user, as referenced by spreadsheets.
    OwnerId
);

/// Emitted (via outbox) after a spreadsheet is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetCreated {
    /// Created spreadsheet.
    pub spreadsheet_id: SpreadsheetId,
    /// Owning user.
    pub owner_id: OwnerId,
    /// Title at creation time.
    pub title: String,
    /// Creation time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Emitted (via outbox) after a spreadsheet is renamed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetRenamed {
    /// Renamed spreadsheet.
    pub spreadsheet_id: SpreadsheetId,
    /// New title.
    pub title: String,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Emitted (via outbox) after a spreadsheet is deleted (soft delete).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetDeleted {
    /// Deleted spreadsheet.
    pub spreadsheet_id: SpreadsheetId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}
