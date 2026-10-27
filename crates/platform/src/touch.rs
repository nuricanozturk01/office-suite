//! Idempotent "last edited" bump for resource tables, driven by integration events.

use sqlx::{AssertSqlSafe, PgPool};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::outbox::claim;

/// Invalid table identifier.
#[derive(Debug, Error)]
#[error("invalid table identifier `{0}`")]
pub struct InvalidTable(String);

/// A `<schema>.<table>` with `id`, `updated_at` and `deleted_at` columns.
#[derive(Debug, Clone)]
pub struct TouchTable {
    sql: String,
}

fn plain(name: &str) -> bool {
    name.len() <= 63
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl TouchTable {
    /// Accepts plain lowercase identifiers (`^[a-z][a-z0-9_]{0,62}$`) only.
    ///
    /// # Errors
    /// [`InvalidTable`] for anything else.
    pub fn new(schema: &str, table: &str) -> Result<Self, InvalidTable> {
        if !plain(schema) || !plain(table) {
            return Err(InvalidTable(format!("{schema}.{table}")));
        }
        // Identifiers cannot be bound as parameters; both are validated above and come from the
        // module that owns the table, never from user input.
        Ok(Self {
            sql: format!(
                "UPDATE {schema}.{table} SET updated_at = GREATEST(updated_at, $2)
                 WHERE id = $1 AND deleted_at IS NULL"
            ),
        })
    }
}

/// Moves `resource` to `at` in its table unless it is newer already, once per `event_id`.
///
/// The ledger entry and the update share one transaction, so a redelivered event does nothing.
///
/// # Errors
/// Returns the underlying [`sqlx::Error`]; the caller retries.
pub async fn touch_updated_at(
    pool: &PgPool,
    table: &TouchTable,
    consumer: &str,
    event_id: Uuid,
    resource: Uuid,
    at: OffsetDateTime,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    if claim(&mut tx, consumer, event_id).await? {
        sqlx::query(AssertSqlSafe(table.sql.as_str()))
            .bind(resource)
            .bind(at)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}
