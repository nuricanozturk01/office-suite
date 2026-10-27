use async_trait::async_trait;
use platform::pagination::UpdatedCursor;
use serde_json::Value;
use spreadsheets_contracts::{OwnerId, SpreadsheetId};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    application::{Page, RepositoryError, SpreadsheetRepository, SpreadsheetSummary},
    domain::{Spreadsheet, SpreadsheetEvent, SpreadsheetTitle, WorkbookContent},
};

/// Postgres-backed repository (schema `spreadsheets`).
#[derive(Debug)]
pub(crate) struct PgSpreadsheetRepository {
    pool: PgPool,
}

impl PgSpreadsheetRepository {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct Row {
    id: sqlx::types::Uuid,
    owner_id: sqlx::types::Uuid,
    title: String,
    content: Value,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

struct SummaryRow {
    id: sqlx::types::Uuid,
    title: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "spreadsheets storage failure");
    RepositoryError::Storage
}

fn to_spreadsheet(row: &Row) -> Result<Spreadsheet, RepositoryError> {
    let title = SpreadsheetTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
    let content =
        WorkbookContent::parse(row.content.clone()).map_err(|_| RepositoryError::Storage)?;
    Ok(Spreadsheet::restore(
        SpreadsheetId::from_uuid(row.id),
        OwnerId::from_uuid(row.owner_id),
        title,
        content,
        row.version,
        row.created_at,
        row.updated_at,
    ))
}

/// Writes the aggregate's pending events to the outbox, inside the caller's transaction.
async fn write_events(
    tx: &mut sqlx::PgConnection,
    spreadsheet: &mut Spreadsheet,
) -> Result<(), RepositoryError> {
    for event in spreadsheet.take_events() {
        let (event_type, payload, at) = match &event {
            SpreadsheetEvent::Created(e) => {
                ("SpreadsheetCreated", serde_json::to_value(e), e.created_at)
            }
            SpreadsheetEvent::Renamed(e) => ("SpreadsheetRenamed", serde_json::to_value(e), e.at),
            SpreadsheetEvent::Deleted(e) => ("SpreadsheetDeleted", serde_json::to_value(e), e.at),
        };
        let payload = payload.map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO spreadsheets.outbox (id, event_type, payload, created_at)
             VALUES ($1, $2, $3, $4)",
            shared_kernel::Uuid::now_v7(),
            event_type,
            payload,
            at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
    }
    Ok(())
}

#[async_trait]
impl SpreadsheetRepository for PgSpreadsheetRepository {
    async fn insert(&self, spreadsheet: &mut Spreadsheet) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "INSERT INTO spreadsheets.spreadsheets
                (id, owner_id, title, content, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            spreadsheet.id().as_uuid(),
            spreadsheet.owner_id().as_uuid(),
            spreadsheet.title().as_str(),
            spreadsheet.content_value(),
            spreadsheet.created_at(),
            spreadsheet.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        write_events(&mut tx, spreadsheet).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn find(&self, id: SpreadsheetId) -> Result<Option<Spreadsheet>, RepositoryError> {
        sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, content, version, created_at, updated_at
             FROM spreadsheets.spreadsheets WHERE id = $1 AND deleted_at IS NULL",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(|row| to_spreadsheet(&row))
        .transpose()
    }

    async fn update_title(&self, spreadsheet: &mut Spreadsheet) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE spreadsheets.spreadsheets
             SET title = $2, updated_at = $3
             WHERE id = $1 AND deleted_at IS NULL",
            spreadsheet.id().as_uuid(),
            spreadsheet.title().as_str(),
            spreadsheet.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            spreadsheet.take_events();
            return Ok(());
        }
        write_events(&mut tx, spreadsheet).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn save_content(
        &self,
        id: SpreadsheetId,
        content: &Value,
        expected_version: i64,
        now: OffsetDateTime,
    ) -> Result<i64, RepositoryError> {
        sqlx::query_scalar!(
            "UPDATE spreadsheets.spreadsheets
             SET content = $2, version = version + 1, updated_at = $3
             WHERE id = $1 AND version = $4 AND deleted_at IS NULL
             RETURNING version",
            id.as_uuid(),
            content,
            now,
            expected_version,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .ok_or(RepositoryError::Conflict)
    }

    async fn delete(&self, spreadsheet: &mut Spreadsheet) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE spreadsheets.spreadsheets SET deleted_at = $2, updated_at = $2
             WHERE id = $1 AND deleted_at IS NULL",
            spreadsheet.id().as_uuid(),
            spreadsheet.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            spreadsheet.take_events();
            return Ok(());
        }
        write_events(&mut tx, spreadsheet).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<SpreadsheetSummary>, RepositoryError> {
        let fetch = i64::from(limit) + 1;
        let rows = sqlx::query_as!(
            SummaryRow,
            "SELECT id, title, created_at, updated_at
             FROM spreadsheets.spreadsheets
             WHERE owner_id = $1 AND deleted_at IS NULL
               AND ($2::timestamptz IS NULL OR (updated_at, id) < ($2::timestamptz, $3::uuid))
             ORDER BY updated_at DESC, id DESC
             LIMIT $4",
            owner.as_uuid(),
            cursor.map(|c| c.updated_at),
            cursor.map(|c| c.id),
            fetch,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?;

        let mut items = rows
            .into_iter()
            .map(|row| {
                let title =
                    SpreadsheetTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
                Ok(SpreadsheetSummary {
                    id: SpreadsheetId::from_uuid(row.id),
                    title,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items
                .last()
                .map(|item| UpdatedCursor::new(item.updated_at, item.id.as_uuid()))
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
}
