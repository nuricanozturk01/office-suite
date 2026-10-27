use async_trait::async_trait;
use identity_contracts::{ProfileUpdated, UserDeletionScheduled, UserId};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    application::{RepositoryError, UserRepository},
    domain::{AvatarUrl, Bio, DisplayName, Email, User},
};

/// Postgres-backed user repository (schema `identity`).
#[derive(Debug)]
pub(crate) struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// Creates the repository.
    #[must_use]
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

struct UserRow {
    id: sqlx::types::Uuid,
    email: String,
    display_name: String,
    password_hash: String,
    avatar_url: Option<String>,
    bio: Option<String>,
    deletion_scheduled_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

fn storage(err: &sqlx::Error) -> RepositoryError {
    tracing::error!(error = %err, "identity storage failure");
    RepositoryError::Storage
}

fn to_user(row: UserRow) -> Result<User, RepositoryError> {
    let email = Email::parse(&row.email).map_err(|_| RepositoryError::Storage)?;
    let name = DisplayName::parse(&row.display_name).map_err(|_| RepositoryError::Storage)?;
    let avatar_url = row
        .avatar_url
        .as_deref()
        .map(AvatarUrl::parse)
        .transpose()
        .map_err(|_| RepositoryError::Storage)?;
    let bio = row
        .bio
        .as_deref()
        .map(Bio::new)
        .transpose()
        .map_err(|_| RepositoryError::Storage)?;
    Ok(User::restore(
        UserId::from_uuid(row.id),
        email,
        name,
        row.password_hash,
        avatar_url,
        bio,
        row.deletion_scheduled_at,
        row.created_at,
    ))
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn insert(&self, user: &mut User) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        let inserted = sqlx::query!(
            "INSERT INTO identity.users (id, email, display_name, password_hash, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $5)",
            user.id().as_uuid(),
            user.email().as_str(),
            user.display_name().as_str(),
            user.password_hash(),
            user.created_at(),
        )
        .execute(&mut *tx)
        .await;
        if let Err(err) = inserted {
            return Err(match &err {
                sqlx::Error::Database(db) if db.is_unique_violation() => {
                    RepositoryError::EmailTaken
                }
                _ => storage(&err),
            });
        }
        for event in user.take_events() {
            let payload = serde_json::to_value(&event).map_err(|_| RepositoryError::Storage)?;
            sqlx::query!(
                "INSERT INTO identity.outbox (id, event_type, payload, created_at)
                 VALUES ($1, 'UserRegistered', $2, $3)",
                shared_kernel::Uuid::now_v7(),
                payload,
                event.registered_at,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| storage(&e))?;
        }
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        sqlx::query_as!(
            UserRow,
            "SELECT id, email, display_name, password_hash,
                    avatar_url, bio, deletion_scheduled_at, created_at
             FROM identity.users WHERE email = $1",
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(to_user)
        .transpose()
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        sqlx::query_as!(
            UserRow,
            "SELECT id, email, display_name, password_hash,
                    avatar_url, bio, deletion_scheduled_at, created_at
             FROM identity.users WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| storage(&e))?
        .map(to_user)
        .transpose()
    }

    async fn update_profile(
        &self,
        id: UserId,
        display_name: &DisplayName,
        bio: Option<&Bio>,
        avatar_url: Option<&AvatarUrl>,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "UPDATE identity.users
             SET display_name = $2, bio = $3, avatar_url = $4, updated_at = $5
             WHERE id = $1",
            id.as_uuid(),
            display_name.as_str(),
            bio.map(Bio::as_str),
            avatar_url.map(AvatarUrl::as_str),
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let event = ProfileUpdated {
            user_id: id,
            display_name: display_name.as_str().to_owned(),
            updated_at: now,
        };
        let payload = serde_json::to_value(&event).map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO identity.outbox (id, event_type, payload, created_at)
             VALUES ($1, 'ProfileUpdated', $2, $3)",
            shared_kernel::Uuid::now_v7(),
            payload,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn update_password(&self, id: UserId, new_hash: &str) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE identity.users SET password_hash = $2, updated_at = now() WHERE id = $1",
            id.as_uuid(),
            new_hash,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(())
    }

    async fn revoke_all_refresh_tokens(
        &self,
        id: UserId,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE identity.refresh_tokens
             SET revoked_at = $2
             WHERE user_id = $1 AND revoked_at IS NULL AND used_at IS NULL",
            id.as_uuid(),
            now,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(())
    }

    async fn schedule_deletion(
        &self,
        id: UserId,
        execute_after: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(|e| storage(&e))?;
        sqlx::query!(
            "UPDATE identity.users
             SET deletion_scheduled_at = $2, updated_at = $3
             WHERE id = $1",
            id.as_uuid(),
            execute_after,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        let event = UserDeletionScheduled {
            user_id: id,
            scheduled_at: now,
            execute_after,
        };
        let payload = serde_json::to_value(&event).map_err(|_| RepositoryError::Storage)?;
        sqlx::query!(
            "INSERT INTO identity.outbox (id, event_type, payload, created_at)
             VALUES ($1, 'UserDeletionScheduled', $2, $3)",
            shared_kernel::Uuid::now_v7(),
            payload,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| storage(&e))?;
        tx.commit().await.map_err(|e| storage(&e))
    }

    async fn cancel_deletion(&self, id: UserId) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE identity.users
             SET deletion_scheduled_at = NULL, updated_at = now()
             WHERE id = $1",
            id.as_uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| storage(&e))?;
        Ok(())
    }
}
