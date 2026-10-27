use async_trait::async_trait;
use documents_contracts::{DocumentId, OwnerId};
use platform::pagination::UpdatedCursor;
use sqlx::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    application::{DocumentRepository, Page, RepositoryError},
    domain::{Document, DocumentEvent, DocumentTitle},
};

/// Postgres-backed repository (schema `documents`).
#[derive(Debug)]
pub(crate) struct PgDocumentRepository {
    pool: PgPool,
}

impl PgDocumentRepository {
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
    tracing::error!(error = %err, "documents storage failure");
    RepositoryError::Storage
}

fn to_document(row: &Row) -> Result<Document, RepositoryError> {
    let title = DocumentTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
    Ok(Document::restore(
        DocumentId::from_uuid(row.id),
        OwnerId::from_uuid(row.owner_id),
        title,
        row.created_at,
        row.updated_at,
    ))
}

/// Writes the aggregate's pending events to the outbox, inside the caller's transaction.
async fn write_events(tx: &mut PgConnection, doc: &mut Document) -> Result<(), RepositoryError> {
    for event in doc.take_events() {
        let (event_type, payload, at) = match &event {
            DocumentEvent::Created(e) => ("DocumentCreated", serde_json::to_value(e), e.created_at),
            DocumentEvent::Renamed(e) => ("DocumentRenamed", serde_json::to_value(e), e.at),
            DocumentEvent::Deleted(e) => ("DocumentDeleted", serde_json::to_value(e), e.at),
        };
        let payload = payload.map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO documents.outbox (id, event_type, payload, created_at)
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
impl DocumentRepository for PgDocumentRepository {
    async fn insert(&self, doc: &mut Document) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "INSERT INTO documents.documents (id, owner_id, title, created_at, updated_at)
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

    async fn find(&self, id: DocumentId) -> Result<Option<Document>, RepositoryError> {
        sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, created_at, updated_at
             FROM documents.documents WHERE id = $1 AND deleted_at IS NULL",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(|row| to_document(&row))
        .transpose()
    }

    async fn update(&self, doc: &mut Document) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE documents.documents
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

    async fn delete(&self, doc: &mut Document) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE documents.documents SET deleted_at = $2, updated_at = $2
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
    ) -> Result<Page<Document>, RepositoryError> {
        // Fetch one extra row to know whether another page exists.
        let fetch = i64::from(limit) + 1;
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, created_at, updated_at
             FROM documents.documents
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
            .map(|row| to_document(&row))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items
                .last()
                .map(|doc| UpdatedCursor::new(doc.updated_at(), doc.id().as_uuid()))
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
}
