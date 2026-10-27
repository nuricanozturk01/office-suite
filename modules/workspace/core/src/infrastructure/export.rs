use async_trait::async_trait;
use identity_contracts::UserId;
use sqlx::PgPool;
use workspace_contracts::{ExportError, FileExportRow, WorkspaceExportPort};

/// Postgres-backed GDPR export adapter.
#[derive(Debug)]
pub struct PgWorkspaceExportAdapter {
    pool: PgPool,
}

impl PgWorkspaceExportAdapter {
    /// Creates the adapter.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct ResourceRow {
    id: sqlx::types::Uuid,
    kind: Option<String>,
    title: String,
    created_at: Option<time::OffsetDateTime>,
    updated_at: Option<time::OffsetDateTime>,
}

#[async_trait]
impl WorkspaceExportPort for PgWorkspaceExportAdapter {
    async fn list_owned_files(&self, owner: UserId) -> Result<Vec<FileExportRow>, ExportError> {
        let rows = sqlx::query_as!(
            ResourceRow,
            r#"SELECT id,
                      kind,
                      title,
                      created_at,
                      COALESCE(title_at, created_at) AS "updated_at: _"
               FROM workspace.resources
               WHERE owner_id = $1 AND deleted_at IS NULL
               ORDER BY created_at DESC"#,
            owner.as_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "workspace export storage failure");
            ExportError::Storage
        })?;

        let epoch = time::OffsetDateTime::UNIX_EPOCH;
        Ok(rows
            .into_iter()
            .map(|r| FileExportRow {
                id: r.id.to_string(),
                name: r.title,
                kind: r.kind.unwrap_or_else(|| "document".to_owned()),
                created_at: r.created_at.unwrap_or(epoch),
                updated_at: r.updated_at.unwrap_or(epoch),
            })
            .collect())
    }
}
