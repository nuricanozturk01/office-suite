//! Public surface of the `identity` module: IDs, events and the token verification port.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;

typed_id!(
    /// Identifier of a user.
    UserId
);

/// Emitted (via outbox) after a user registers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRegistered {
    /// Registered user.
    pub user_id: UserId,
    /// Display name at registration time.
    pub display_name: String,
    /// Registration time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub registered_at: OffsetDateTime,
}

/// Synchronous port: resolve an access token to a user.
pub trait TokenVerifier: Send + Sync + std::fmt::Debug {
    /// Returns the user a valid access token was issued for.
    fn verify(&self, token: &str) -> Option<UserId>;
}

/// Port: resolve an email address to a user (used for sharing). Implemented by `identity`.
#[async_trait::async_trait]
pub trait UserDirectory: Send + Sync + std::fmt::Debug {
    /// The user registered under `email`; `None` for unknown or malformed addresses.
    /// Matching is case-insensitive and ignores surrounding whitespace.
    async fn find_by_email(&self, email: &str) -> Option<UserId>;
}

/// Port: public profile data of users, used to label people in shared sessions.
/// Implemented by `identity`.
#[async_trait::async_trait]
pub trait UserProfiles: Send + Sync + std::fmt::Debug {
    /// Display name of `user`; `None` when the user is unknown.
    async fn display_name(&self, user: UserId) -> Option<String>;
}

/// Emitted (via outbox) after a user updates their profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileUpdated {
    /// The user whose profile changed.
    pub user_id: UserId,
    /// New display name.
    pub display_name: String,
    /// When the update occurred (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Emitted (via outbox) when a user schedules their account for deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserDeletionScheduled {
    /// The user who requested deletion.
    pub user_id: UserId,
    /// When the request was made (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_at: OffsetDateTime,
    /// When the account will actually be erased (UTC) — `scheduled_at` + 30 days.
    #[serde(with = "time::serde::rfc3339")]
    pub execute_after: OffsetDateTime,
}
