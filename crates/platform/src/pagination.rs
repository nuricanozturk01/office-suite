//! Keyset cursor for owner listings ordered by last update.

use std::{fmt, str::FromStr};

use axum::http::StatusCode;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::http::ProblemDetails;

/// Position after the last row of a page. The next page holds rows that sort strictly after
/// `(updated_at, id)` in `updated_at DESC, id DESC` order.
///
/// Serialized as `<unix microseconds>.<uuid>`; microseconds match the precision Postgres stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdatedCursor {
    pub updated_at: OffsetDateTime,
    pub id: Uuid,
}

impl UpdatedCursor {
    /// Cursor positioned after the row with the given sort key.
    #[must_use]
    pub const fn new(updated_at: OffsetDateTime, id: Uuid) -> Self {
        Self { updated_at, id }
    }
}

impl fmt::Display for UpdatedCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.updated_at.unix_timestamp_nanos() / 1_000,
            self.id
        )
    }
}

/// A cursor string that does not decode.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid pagination cursor")]
pub struct InvalidCursor;

impl FromStr for UpdatedCursor {
    type Err = InvalidCursor;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let (micros, id) = raw.split_once('.').ok_or(InvalidCursor)?;
        let micros: i128 = micros.parse().map_err(|_| InvalidCursor)?;
        let nanos = micros.checked_mul(1_000).ok_or(InvalidCursor)?;
        let updated_at =
            OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|_| InvalidCursor)?;
        let id = Uuid::parse_str(id).map_err(|_| InvalidCursor)?;
        Ok(Self { updated_at, id })
    }
}

impl From<InvalidCursor> for ProblemDetails {
    fn from(_: InvalidCursor) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
        )
        .with_detail("invalid cursor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_at_postgres_precision() {
        let at = OffsetDateTime::from_unix_timestamp_nanos(1_700_000_000_123_456_000).unwrap();
        let id = Uuid::now_v7();
        let cursor = UpdatedCursor::new(at, id);
        let parsed: UpdatedCursor = cursor.to_string().parse().unwrap();
        assert_eq!(parsed, cursor);
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "",
            "nodot",
            "12.notauuid",
            "x.00000000-0000-0000-0000-000000000000",
        ] {
            assert_eq!(bad.parse::<UpdatedCursor>(), Err(InvalidCursor), "{bad}");
        }
        let overflow = format!("{}.{}", i128::MAX, Uuid::nil());
        assert_eq!(overflow.parse::<UpdatedCursor>(), Err(InvalidCursor));
    }
}
