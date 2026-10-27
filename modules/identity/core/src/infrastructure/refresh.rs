use async_trait::async_trait;
use identity_contracts::UserId;
use sha2::{Digest, Sha256};
use shared_kernel::Uuid;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;

use crate::application::{
    NewRefreshToken, RefreshCodec, RefreshStore, RefreshSuccessor, RepositoryError, Rotation,
};

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "identity storage failure");
    RepositoryError::Storage
}

/// 244 random bits (two v4 UUIDs, CSPRNG-backed), SHA-256 at rest.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RandomRefreshCodec;

impl RefreshCodec for RandomRefreshCodec {
    fn generate(&self) -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    fn hash(&self, token: &str) -> Vec<u8> {
        Sha256::digest(token.as_bytes()).to_vec()
    }
}

/// Postgres-backed refresh token store (schema `identity`).
#[derive(Debug)]
pub(crate) struct PgRefreshStore {
    pool: PgPool,
}

impl PgRefreshStore {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn insert_token(
    conn: &mut PgConnection,
    token: &NewRefreshToken,
    now: OffsetDateTime,
) -> Result<(), RepositoryError> {
    sqlx::query!(
        "INSERT INTO identity.refresh_tokens (id, family_id, user_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
        token.id,
        token.family_id,
        token.user_id.as_uuid(),
        token.token_hash,
        token.expires_at,
        now,
    )
    .execute(conn)
    .await
    .map_err(|e| storage(&e))?;
    Ok(())
}

#[async_trait]
impl RefreshStore for PgRefreshStore {
    async fn insert(
        &self,
        token: &NewRefreshToken,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut conn = self.pool.acquire().await.map_err(|e| storage(&e))?;
        insert_token(&mut conn, token, now).await
    }

    async fn rotate(
        &self,
        presented: &[u8],
        successor: &RefreshSuccessor,
        now: OffsetDateTime,
    ) -> Result<Rotation, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        // The row lock serializes two requests presenting the same token.
        let row = sqlx::query!(
            "SELECT id, family_id, user_id, expires_at, used_at, revoked_at
             FROM identity.refresh_tokens WHERE token_hash = $1 FOR UPDATE",
            presented,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let Some(row) = row else {
            return Ok(Rotation::Unknown);
        };
        if row.revoked_at.is_some() || row.expires_at <= now {
            return Ok(Rotation::Unknown);
        }
        if row.used_at.is_some() {
            sqlx::query!(
                "UPDATE identity.refresh_tokens SET revoked_at = $2
                 WHERE family_id = $1 AND revoked_at IS NULL",
                row.family_id,
                now,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
            tx.commit().await.map_err(|e| storage(&e))?;
            return Ok(Rotation::Reused);
        }
        sqlx::query!(
            "UPDATE identity.refresh_tokens SET used_at = $2 WHERE id = $1",
            row.id,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let successor = NewRefreshToken {
            id: successor.id,
            family_id: row.family_id,
            user_id: UserId::from_uuid(row.user_id),
            token_hash: successor.token_hash.clone(),
            expires_at: successor.expires_at,
        };
        insert_token(&mut tx, &successor, now).await?;
        tx.commit().await.map_err(|e| storage(&e))?;
        Ok(Rotation::Rotated {
            user_id: UserId::from_uuid(row.user_id),
        })
    }

    async fn revoke_family(
        &self,
        presented: &[u8],
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE identity.refresh_tokens SET revoked_at = $2
             WHERE revoked_at IS NULL AND family_id =
                (SELECT family_id FROM identity.refresh_tokens WHERE token_hash = $1)",
            presented,
            now,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(())
    }
}
