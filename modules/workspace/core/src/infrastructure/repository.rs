use async_trait::async_trait;
use identity_contracts::UserId;
use shared_kernel::Uuid;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use workspace_contracts::{ResourceId, ResourceKind, Role};

use crate::{
    application::{
        OutboxEvent, Page, RepositoryError, ResourceChange, ResourceInfo, ResourceStore,
        ShareRecord, SharedItem, WorkspaceRepository,
    },
    domain::Share,
};

/// Postgres-backed repository (schema `workspace`).
#[derive(Debug)]
pub(crate) struct PgWorkspaceRepository {
    pool: PgPool,
}

impl PgWorkspaceRepository {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "workspace storage failure");
    RepositoryError::Storage
}

pub(super) const fn role_str(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Commenter => "commenter",
        Role::Editor => "editor",
        Role::Owner => "owner",
    }
}

pub(super) fn parse_role(value: &str) -> Option<Role> {
    match value {
        "viewer" => Some(Role::Viewer),
        "commenter" => Some(Role::Commenter),
        "editor" => Some(Role::Editor),
        "owner" => Some(Role::Owner),
        _ => None,
    }
}

const fn kind_str(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Document => "document",
        ResourceKind::Spreadsheet => "spreadsheet",
        ResourceKind::Presentation => "presentation",
        ResourceKind::Pdf => "pdf",
    }
}

fn parse_kind(value: &str) -> Option<ResourceKind> {
    match value {
        "document" => Some(ResourceKind::Document),
        "spreadsheet" => Some(ResourceKind::Spreadsheet),
        "presentation" => Some(ResourceKind::Presentation),
        "pdf" => Some(ResourceKind::Pdf),
        _ => None,
    }
}

async fn write_events(
    tx: &mut PgConnection,
    events: &[OutboxEvent],
) -> Result<(), RepositoryError> {
    for event in events {
        sqlx::query!(
            "INSERT INTO workspace.outbox (id, event_type, payload, created_at)
             VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(),
            event.event_type,
            event.payload,
            event.at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
    }
    Ok(())
}

#[async_trait]
impl WorkspaceRepository for PgWorkspaceRepository {
    async fn find_resource(&self, id: ResourceId) -> Result<Option<ResourceInfo>, RepositoryError> {
        let row = sqlx::query!(
            "SELECT owner_id FROM workspace.resources
             WHERE id = $1 AND deleted_at IS NULL AND owner_id IS NOT NULL",
            id.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(row.and_then(|r| r.owner_id).map(|owner| ResourceInfo {
            owner_id: UserId::from_uuid(owner),
        }))
    }

    async fn upsert_share(
        &self,
        share: &Share,
        email: &str,
        now: OffsetDateTime,
        events_for: &(
             dyn Fn(Option<Role>) -> Result<Vec<OutboxEvent>, RepositoryError> + Send + Sync
         ),
    ) -> Result<ShareRecord, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        // Lock the current row so concurrent role changes see each other's result and the
        // downgrade detection below cannot be raced.
        let previous = sqlx::query_scalar!(
            "SELECT role FROM workspace.shares WHERE resource_id = $1 AND grantee_id = $2 FOR UPDATE",
            share.resource_id().as_uuid(),
            share.grantee().as_uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let events = events_for(previous.as_deref().and_then(parse_role))?;
        let row = sqlx::query!(
            "INSERT INTO workspace.shares (resource_id, grantee_id, grantee_email, role, shared_by, created_at, updated_at)
             VALUES ($1, $2, $6, $3, $4, $5, $5)
             ON CONFLICT (resource_id, grantee_id) DO UPDATE
             SET role = EXCLUDED.role, shared_by = EXCLUDED.shared_by,
                 grantee_email = EXCLUDED.grantee_email, updated_at = EXCLUDED.updated_at
             RETURNING created_at, updated_at",
            share.resource_id().as_uuid(),
            share.grantee().as_uuid(),
            role_str(share.role()),
            share.shared_by().as_uuid(),
            now,
            email,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        write_events(&mut tx, &events).await?;
        tx.commit().await.map_err(|e| storage(&e))?;
        Ok(ShareRecord {
            grantee: share.grantee(),
            email: email.to_owned(),
            role: share.role(),
            shared_by: share.shared_by(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn delete_share(
        &self,
        resource: ResourceId,
        grantee: UserId,
        events: &[OutboxEvent],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let removed = sqlx::query!(
            "DELETE FROM workspace.shares WHERE resource_id = $1 AND grantee_id = $2",
            resource.as_uuid(),
            grantee.as_uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?
        .rows_affected()
            > 0;
        if removed {
            write_events(&mut tx, events).await?;
        }
        tx.commit().await.map_err(|e| storage(&e))?;
        Ok(removed)
    }

    async fn list_shares(&self, resource: ResourceId) -> Result<Vec<ShareRecord>, RepositoryError> {
        let rows = sqlx::query!(
            "SELECT grantee_id, grantee_email, role, shared_by, created_at, updated_at
             FROM workspace.shares WHERE resource_id = $1 ORDER BY created_at, grantee_id",
            resource.as_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        rows.into_iter()
            .map(|r| {
                Ok(ShareRecord {
                    grantee: UserId::from_uuid(r.grantee_id),
                    email: r.grantee_email,
                    role: parse_role(&r.role).ok_or(RepositoryError::Storage)?,
                    shared_by: UserId::from_uuid(r.shared_by),
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect()
    }

    async fn shared_with_me(
        &self,
        user: UserId,
        cursor: Option<ResourceId>,
        limit: u32,
    ) -> Result<Page<SharedItem>, RepositoryError> {
        // Fetch one extra row to know whether another page exists.
        let fetch = i64::from(limit) + 1;
        let rows = sqlx::query!(
            "SELECT r.id, r.kind, r.title, r.owner_id, s.role, s.updated_at AS shared_at
             FROM workspace.shares s
             JOIN workspace.resources r ON r.id = s.resource_id
             WHERE s.grantee_id = $1 AND r.deleted_at IS NULL AND r.owner_id IS NOT NULL
               AND ($2::uuid IS NULL OR r.id < $2)
             ORDER BY r.id DESC
             LIMIT $3",
            user.as_uuid(),
            cursor.map(|c| c.as_uuid()),
            fetch,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        let mut items = rows
            .into_iter()
            .map(|r| {
                Ok(SharedItem {
                    id: ResourceId::from_uuid(r.id),
                    kind: r
                        .kind
                        .as_deref()
                        .and_then(parse_kind)
                        .ok_or(RepositoryError::Storage)?,
                    title: r.title,
                    role: parse_role(&r.role).ok_or(RepositoryError::Storage)?,
                    owner_id: UserId::from_uuid(r.owner_id.ok_or(RepositoryError::Storage)?),
                    shared_at: r.shared_at,
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        let next_cursor = if has_more {
            items.last().map(|i| i.id)
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
}

#[async_trait]
impl ResourceStore for PgWorkspaceRepository {
    async fn apply(
        &self,
        consumer: &'static str,
        event_id: Uuid,
        change: ResourceChange,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let first = platform::outbox::claim(&mut tx, consumer, event_id)
            .await
            .map_err(|e| storage(&e))?;
        if first {
            apply_change(&mut tx, change).await?;
        }
        tx.commit().await.map_err(|e| storage(&e))
    }
}

/// Upserts tolerate replays and reordering: titles carry the time of the event that set them and
/// a rename or delete that arrives first leaves a stub row that `Created` later completes.
async fn apply_change(
    tx: &mut PgConnection,
    change: ResourceChange,
) -> Result<(), RepositoryError> {
    match change {
        ResourceChange::Created {
            id,
            kind,
            owner,
            title,
            at,
        } => {
            sqlx::query!(
                "INSERT INTO workspace.resources (id, kind, owner_id, title, title_at, created_at)
                 VALUES ($1, $2, $3, $4, $5, $5)
                 ON CONFLICT (id) DO UPDATE SET
                    kind = EXCLUDED.kind,
                    owner_id = EXCLUDED.owner_id,
                    created_at = EXCLUDED.created_at,
                    title = CASE WHEN workspace.resources.title_at IS NULL
                                   OR workspace.resources.title_at < EXCLUDED.title_at
                                 THEN EXCLUDED.title ELSE workspace.resources.title END,
                    title_at = GREATEST(workspace.resources.title_at, EXCLUDED.title_at)",
                id.as_uuid(),
                kind_str(kind),
                owner.as_uuid(),
                title,
                at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
        }
        ResourceChange::Renamed { id, title, at } => {
            sqlx::query!(
                "INSERT INTO workspace.resources (id, title, title_at) VALUES ($1, $2, $3)
                 ON CONFLICT (id) DO UPDATE SET
                    title = CASE WHEN workspace.resources.title_at IS NULL
                                   OR workspace.resources.title_at < EXCLUDED.title_at
                                 THEN EXCLUDED.title ELSE workspace.resources.title END,
                    title_at = GREATEST(workspace.resources.title_at, EXCLUDED.title_at)",
                id.as_uuid(),
                title,
                at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
        }
        ResourceChange::Deleted { id, at } => {
            sqlx::query!(
                "INSERT INTO workspace.resources (id, deleted_at) VALUES ($1, $2)
                 ON CONFLICT (id) DO UPDATE SET
                    deleted_at = COALESCE(workspace.resources.deleted_at, EXCLUDED.deleted_at)",
                id.as_uuid(),
                at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
        }
    }
    Ok(())
}
