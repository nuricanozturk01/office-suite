use async_trait::async_trait;
use pdfs_contracts::{OwnerId, PdfId};
use platform::pagination::UpdatedCursor;
use sqlx::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    application::{Page, PdfRepository, RepositoryError},
    domain::{Pdf, PdfEvent, PdfTitle},
};

/// Postgres-backed repository (schema `pdfs`).
#[derive(Debug)]
pub(crate) struct PgPdfRepository {
    pool: PgPool,
}

impl PgPdfRepository {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct Row {
    id: sqlx::types::Uuid,
    owner_id: sqlx::types::Uuid,
    title: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "pdfs storage failure");
    RepositoryError::Storage
}

fn to_pdf(row: &Row) -> Result<Pdf, RepositoryError> {
    let title = PdfTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
    Ok(Pdf::restore(
        PdfId::from_uuid(row.id),
        OwnerId::from_uuid(row.owner_id),
        title,
        row.created_at,
        row.updated_at,
    ))
}

/// Writes the aggregate's pending events to the outbox, inside the caller's transaction.
async fn write_events(tx: &mut PgConnection, doc: &mut Pdf) -> Result<(), RepositoryError> {
    for event in doc.take_events() {
        let (event_type, payload, at) = match &event {
            PdfEvent::Created(e) => ("PdfCreated", serde_json::to_value(e), e.created_at),
            PdfEvent::Renamed(e) => ("PdfRenamed", serde_json::to_value(e), e.at),
            PdfEvent::Deleted(e) => ("PdfDeleted", serde_json::to_value(e), e.at),
        };
        let payload = payload.map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO pdfs.outbox (id, event_type, payload, created_at)
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
impl PdfRepository for PgPdfRepository {
    async fn insert(&self, doc: &mut Pdf) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "INSERT INTO pdfs.pdfs (id, owner_id, title, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)",
            doc.id().as_uuid(),
            doc.owner_id().as_uuid(),
            doc.title().as_str(),
            doc.created_at(),
            doc.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        write_events(&mut tx, doc).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn find(&self, id: PdfId) -> Result<Option<Pdf>, RepositoryError> {
        sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, created_at, updated_at
             FROM pdfs.pdfs WHERE id = $1 AND deleted_at IS NULL",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(|row| to_pdf(&row))
        .transpose()
    }

    async fn update(&self, doc: &mut Pdf) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE pdfs.pdfs
             SET title = $2, updated_at = $3, version = version + 1
             WHERE id = $1 AND deleted_at IS NULL",
            doc.id().as_uuid(),
            doc.title().as_str(),
            doc.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            doc.take_events();
            return Ok(());
        }
        write_events(&mut tx, doc).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn delete(&self, doc: &mut Pdf) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE pdfs.pdfs SET deleted_at = $2, updated_at = $2
             WHERE id = $1 AND deleted_at IS NULL",
            doc.id().as_uuid(),
            doc.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            doc.take_events();
            return Ok(());
        }
        write_events(&mut tx, doc).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<Pdf>, RepositoryError> {
        // Fetch one extra row to know whether another page exists.
        let fetch = i64::from(limit) + 1;
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, created_at, updated_at
             FROM pdfs.pdfs
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
            .map(|row| to_pdf(&row))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items
                .last()
                .map(|pdf| UpdatedCursor::new(pdf.updated_at(), pdf.id().as_uuid()))
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
}
