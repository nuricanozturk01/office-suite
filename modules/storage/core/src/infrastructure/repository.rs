use async_trait::async_trait;
use identity_contracts::UserId;
use shared_kernel::Uuid;
use sqlx::PgPool;
use storage_contracts::PackageSaved;
use time::OffsetDateTime;
use workspace_contracts::ResourceId;

use crate::application::{HistoryEntry, PackageRepository, RepositoryError, StoredPackage};

/// Versions kept per resource.
const KEPT_VERSIONS: i64 = 50;

/// Postgres-backed repository (schema `storage`).
#[derive(Debug)]
pub(crate) struct PgPackageRepository {
    pool: PgPool,
}

impl PgPackageRepository {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "storage failure");
    RepositoryError::Storage
}

#[async_trait]
impl PackageRepository for PgPackageRepository {
    async fn get(&self, resource: Uuid) -> Result<Option<StoredPackage>, RepositoryError> {
        let row = sqlx::query!(
            "SELECT name, bytes, version FROM storage.packages WHERE resource_id = $1",
            resource
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(row.map(|row| StoredPackage {
            name: row.name,
            bytes: row.bytes,
            version: row.version,
        }))
    }

    async fn put(
        &self,
        resource: Uuid,
        name: &str,
        bytes: &[u8],
        base_version: Option<i64>,
        actor: Uuid,
        now: OffsetDateTime,
    ) -> Result<i64, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let version = sqlx::query_scalar!(
            "INSERT INTO storage.packages
                 (resource_id, name, bytes, updated_by, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $5)
             ON CONFLICT (resource_id) DO UPDATE
                 SET name = $2, bytes = $3, version = storage.packages.version + 1,
                     updated_by = $4, updated_at = $5
                 WHERE $6::bigint IS NULL OR storage.packages.version = $6
             RETURNING version",
            resource,
            name,
            bytes,
            actor,
            now,
            base_version,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let Some(version) = version else {
            return Err(RepositoryError::Conflict);
        };
        sqlx::query!(
            "INSERT INTO storage.package_history
                 (resource_id, version, name, bytes, saved_by, saved_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            resource,
            version,
            name,
            bytes,
            actor,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        sqlx::query!(
            "DELETE FROM storage.package_history WHERE resource_id = $1 AND version <= $2",
            resource,
            version - KEPT_VERSIONS,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let event = PackageSaved {
            resource_id: ResourceId::from_uuid(resource),
            version,
            by: Some(UserId::from_uuid(actor)),
            at: now,
        };
        let payload = serde_json::to_value(&event).map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO storage.outbox (id, event_type, payload, created_at)
             VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(),
            "PackageSaved",
            payload,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        tx.commit().await.map_err(|e| storage(&e))?;
        Ok(version)
    }

    async fn version(
        &self,
        resource: Uuid,
        version: i64,
    ) -> Result<Option<StoredPackage>, RepositoryError> {
        let row = sqlx::query!(
            "SELECT name, bytes, version FROM storage.package_history
             WHERE resource_id = $1 AND version = $2",
            resource,
            version
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(row.map(|row| StoredPackage {
            name: row.name,
            bytes: row.bytes,
            version: row.version,
        }))
    }

    async fn history(&self, resource: Uuid) -> Result<Vec<HistoryEntry>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT version, name, saved_by, saved_at,
                      octet_length(bytes)::bigint AS "size!"
               FROM storage.package_history
               WHERE resource_id = $1 ORDER BY version DESC"#,
            resource
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(rows
            .into_iter()
            .map(|row| HistoryEntry {
                version: row.version,
                name: row.name,
                size: row.size,
                saved_by: row.saved_by,
                saved_at: row.saved_at,
            })
            .collect())
    }
}
