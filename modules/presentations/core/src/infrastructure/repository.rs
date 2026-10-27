use async_trait::async_trait;
use platform::pagination::UpdatedCursor;
use presentations_contracts::{OwnerId, PresentationId};
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    application::{Page, PresentationRepository, PresentationSummary, RepositoryError},
    domain::{DeckContent, Presentation, PresentationEvent, PresentationTitle},
};

/// Postgres-backed repository (schema `presentations`).
#[derive(Debug)]
pub(crate) struct PgPresentationRepository {
    pool: PgPool,
}

impl PgPresentationRepository {
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
    tracing::error!(error = %err, "presentations storage failure");
    RepositoryError::Storage
}

fn to_presentation(row: &Row) -> Result<Presentation, RepositoryError> {
    let title = PresentationTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
    let content = DeckContent::parse(row.content.clone()).map_err(|_| RepositoryError::Storage)?;
    Ok(Presentation::restore(
        PresentationId::from_uuid(row.id),
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
    presentation: &mut Presentation,
) -> Result<(), RepositoryError> {
    for event in presentation.take_events() {
        let (event_type, payload, at) = match &event {
            PresentationEvent::Created(e) => {
                ("PresentationCreated", serde_json::to_value(e), e.created_at)
            }
            PresentationEvent::Renamed(e) => ("PresentationRenamed", serde_json::to_value(e), e.at),
            PresentationEvent::Deleted(e) => ("PresentationDeleted", serde_json::to_value(e), e.at),
        };
        let payload = payload.map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO presentations.outbox (id, event_type, payload, created_at)
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
impl PresentationRepository for PgPresentationRepository {
    async fn insert(&self, presentation: &mut Presentation) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "INSERT INTO presentations.presentations
                (id, owner_id, title, content, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            presentation.id().as_uuid(),
            presentation.owner_id().as_uuid(),
            presentation.title().as_str(),
            presentation.content_value(),
            presentation.created_at(),
            presentation.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        write_events(&mut tx, presentation).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn find(&self, id: PresentationId) -> Result<Option<Presentation>, RepositoryError> {
        sqlx::query_as!(
            Row,
            "SELECT id, owner_id, title, content, version, created_at, updated_at
             FROM presentations.presentations WHERE id = $1 AND deleted_at IS NULL",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(|row| to_presentation(&row))
        .transpose()
    }

    async fn update_title(&self, presentation: &mut Presentation) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE presentations.presentations
             SET title = $2, updated_at = $3
             WHERE id = $1 AND deleted_at IS NULL",
            presentation.id().as_uuid(),
            presentation.title().as_str(),
            presentation.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            presentation.take_events();
            return Ok(());
        }
        write_events(&mut tx, presentation).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn save_content(
        &self,
        id: PresentationId,
        content: &Value,
        expected_version: i64,
        now: OffsetDateTime,
    ) -> Result<i64, RepositoryError> {
        sqlx::query_scalar!(
            "UPDATE presentations.presentations
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

    async fn delete(&self, presentation: &mut Presentation) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let changed = sqlx::query!(
            "UPDATE presentations.presentations SET deleted_at = $2, updated_at = $2
             WHERE id = $1 AND deleted_at IS NULL",
            presentation.id().as_uuid(),
            presentation.updated_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected();
        if changed == 0 {
            // Concurrently deleted: emit no events for a change that did not happen.
            presentation.take_events();
            return Ok(());
        }
        write_events(&mut tx, presentation).await?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn list_by_owner(
        &self,
        owner: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: u32,
    ) -> Result<Page<PresentationSummary>, RepositoryError> {
        let fetch = i64::from(limit) + 1;
        let rows = sqlx::query_as!(
            SummaryRow,
            "SELECT id, title, created_at, updated_at
             FROM presentations.presentations
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
                    PresentationTitle::new(&row.title).map_err(|_| RepositoryError::Storage)?;
                Ok(PresentationSummary {
                    id: PresentationId::from_uuid(row.id),
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
