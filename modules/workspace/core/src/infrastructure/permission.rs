use async_trait::async_trait;
use identity_contracts::UserId;
use sqlx::PgPool;
use workspace_contracts::{PermissionChecker, ResourceId, Role};

use super::repository::parse_role;

/// Resolves roles live from the database, so a revoked share applies on the very next check.
#[derive(Debug)]
pub(crate) struct PgPermissionChecker {
    pool: PgPool,
}

impl PgPermissionChecker {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PermissionChecker for PgPermissionChecker {
    async fn role(&self, user: UserId, resource: ResourceId) -> Option<Role> {
        let role = sqlx::query_scalar!(
            r#"SELECT CASE WHEN r.owner_id = $1 THEN 'owner' ELSE s.role END AS "role?"
               FROM workspace.resources r
               LEFT JOIN workspace.shares s ON s.resource_id = r.id AND s.grantee_id = $1
               WHERE r.id = $2 AND r.deleted_at IS NULL AND r.owner_id IS NOT NULL"#,
            user.as_uuid(),
            resource.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| tracing::error!(error = %e, "permission lookup failed"))
        .ok()
        .flatten()
        .flatten()?;
        parse_role(&role)
    }
}
