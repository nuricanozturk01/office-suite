use async_trait::async_trait;
use identity_contracts::UserId;
use thiserror::Error;
use time::OffsetDateTime;

use crate::domain::{AvatarUrl, Bio, DisplayName, Email, User};

/// Persistence failure.
#[derive(Debug, Error)]
pub(crate) enum RepositoryError {
    /// Email is already registered.
    #[error("email already registered")]
    EmailTaken,
    /// Infrastructure failure (details are logged, not exposed).
    #[error("storage failure")]
    Storage,
}

/// Persistence port for the user aggregate. Saving also writes domain events to the outbox
/// in the same transaction.
#[async_trait]
pub(crate) trait UserRepository: Send + Sync {
    /// Inserts a new user and its pending events atomically.
    async fn insert(&self, user: &mut User) -> Result<(), RepositoryError>;
    /// Finds a user by email.
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    /// Finds a user by id.
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;
    /// Updates `display_name`, `bio`, and `avatar_url` atomically, writes `ProfileUpdated` outbox event.
    async fn update_profile(
        &self,
        id: UserId,
        display_name: &DisplayName,
        bio: Option<&Bio>,
        avatar_url: Option<&AvatarUrl>,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
    /// Updates the password hash (caller has already verified old password).
    async fn update_password(&self, id: UserId, new_hash: &str) -> Result<(), RepositoryError>;
    /// Revokes all active refresh tokens for the user (called on password change).
    async fn revoke_all_refresh_tokens(
        &self,
        id: UserId,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
    /// Persists the `deletion_scheduled_at` date and writes `UserDeletionScheduled` outbox event.
    async fn schedule_deletion(
        &self,
        id: UserId,
        execute_after: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
    /// Clears `deletion_scheduled_at`.
    async fn cancel_deletion(&self, id: UserId) -> Result<(), RepositoryError>;
}

/// Password hashing port (CPU-bound; implementations must not block the executor).
#[async_trait]
pub(crate) trait PasswordHasher: Send + Sync {
    /// Hashes a plaintext password to a PHC string.
    async fn hash(&self, plaintext: &str) -> Result<String, RepositoryError>;
    /// Verifies a plaintext against a PHC string in constant time.
    async fn verify(&self, plaintext: &str, phc: &str) -> bool;
    /// A valid hash of a random secret, used to equalize login timing for unknown emails.
    fn dummy_hash(&self) -> &str;
}

/// An issued access token.
#[derive(Debug)]
pub(crate) struct AccessToken {
    /// Opaque bearer value.
    pub(crate) value: String,
    /// Lifetime in seconds.
    pub(crate) expires_in_secs: u64,
}

/// Token issuing port.
pub(crate) trait TokenIssuer: Send + Sync {
    /// Issues an access token for a user.
    ///
    /// # Errors
    /// [`RepositoryError::Storage`] when signing fails.
    fn issue(&self, user: UserId) -> Result<AccessToken, RepositoryError>;
}
