use async_trait::async_trait;
use identity_contracts::UserId;
use shared_kernel::Uuid;
use time::OffsetDateTime;

use super::RepositoryError;

/// A refresh token to persist (only its hash is stored).
#[derive(Debug, Clone)]
pub(crate) struct NewRefreshToken {
    pub(crate) id: Uuid,
    /// All tokens issued from one login share a family.
    pub(crate) family_id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) token_hash: Vec<u8>,
    pub(crate) expires_at: OffsetDateTime,
}

/// The token that replaces a rotated one; family and user come from the consumed token.
#[derive(Debug, Clone)]
pub(crate) struct RefreshSuccessor {
    pub(crate) id: Uuid,
    pub(crate) token_hash: Vec<u8>,
    pub(crate) expires_at: OffsetDateTime,
}

/// Result of presenting a refresh token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rotation {
    /// The token was valid and has been replaced by its successor.
    Rotated { user_id: UserId },
    /// The token had been used before: the family is revoked (likely theft).
    Reused,
    /// Unknown, expired or revoked.
    Unknown,
}

/// Persistence port for refresh tokens.
#[async_trait]
pub(crate) trait RefreshStore: Send + Sync {
    /// Stores the first token of a new family.
    async fn insert(
        &self,
        token: &NewRefreshToken,
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
    /// Atomically consumes `presented` and stores `successor` in the same family.
    async fn rotate(
        &self,
        presented: &[u8],
        successor: &RefreshSuccessor,
        now: OffsetDateTime,
    ) -> Result<Rotation, RepositoryError>;
    /// Revokes the whole family of `presented` (logout). Unknown tokens are ignored.
    async fn revoke_family(
        &self,
        presented: &[u8],
        now: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
}

/// Generates and hashes refresh tokens (separate port so tests are deterministic).
pub(crate) trait RefreshCodec: Send + Sync {
    /// A fresh, unguessable token.
    fn generate(&self) -> String;
    /// Stable hash of a token, the only form that is stored.
    fn hash(&self, token: &str) -> Vec<u8>;
}

/// A refresh token handed to the client.
#[derive(Debug, Clone)]
pub(crate) struct RefreshGrant {
    pub(crate) token: String,
    pub(crate) max_age_secs: u64,
}
