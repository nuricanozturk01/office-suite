use std::sync::Arc;

use identity_contracts::UserId;
use serde::Serialize;
use shared_kernel::{Clock, IdGenerator};
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;
use workspace_contracts::{FileExportRow, WorkspaceExportPort};

use super::{
    AccessToken, NewRefreshToken, PasswordHasher, RefreshCodec, RefreshGrant, RefreshStore,
    RefreshSuccessor, RepositoryError, Rotation, TokenIssuer, UserRepository,
};
use crate::domain::{AvatarUrl, Bio, DisplayName, Email, Password, User, ValueError};

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum IdentityError {
    /// Input failed validation.
    #[error(transparent)]
    Invalid(ValueError),
    /// Email already registered.
    #[error("email already registered")]
    EmailTaken,
    /// Wrong email or password (deliberately indistinguishable).
    #[error("invalid credentials")]
    InvalidCredentials,
    /// The refresh token is unknown, expired, revoked or was replayed.
    #[error("invalid refresh token")]
    InvalidRefreshToken,
    /// User does not exist.
    #[error("user not found")]
    NotFound,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
    /// Password change: old password did not match.
    #[error("wrong current password")]
    WrongPassword,
    /// Account deletion is already scheduled.
    #[error("deletion already scheduled")]
    DeletionAlreadyScheduled,
    /// Cancel deletion when none is pending.
    #[error("no pending deletion")]
    NoPendingDeletion,
    /// Profile update: at least one field must be provided (reserved for future optional-all-fields variant).
    #[error("at least one field is required")]
    #[expect(
        dead_code,
        reason = "reserved error code; unused until the API gains fully-optional profile updates"
    )]
    AtLeastOneFieldRequired,
}

/// Account section of a GDPR export.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GdprAccount {
    /// User id.
    pub(crate) id: String,
    /// Email address.
    pub(crate) email: String,
    /// Display name.
    pub(crate) display_name: String,
    /// Bio, if set.
    pub(crate) bio: Option<String>,
    /// Avatar URL, if set.
    pub(crate) avatar_url: Option<String>,
    /// Registration time (RFC3339).
    pub(crate) created_at: String,
}

/// Full GDPR export payload.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct GdprExport {
    /// Account information.
    pub(crate) account: GdprAccount,
    /// All owned files.
    pub(crate) files: Vec<FileExportRow>,
}

impl From<RepositoryError> for IdentityError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::EmailTaken => Self::EmailTaken,
            RepositoryError::Storage => Self::Internal,
        }
    }
}

impl From<ValueError> for IdentityError {
    fn from(err: ValueError) -> Self {
        match err {
            ValueError::WrongPassword => Self::WrongPassword,
            ValueError::DeletionAlreadyScheduled => Self::DeletionAlreadyScheduled,
            ValueError::NoPendingDeletion => Self::NoPendingDeletion,
            other => Self::Invalid(other),
        }
    }
}

/// Identity use cases.
pub(crate) struct IdentityService {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    refresh: RefreshPolicy,
}

/// How refresh tokens are stored, generated and for how long they live.
pub(crate) struct RefreshPolicy {
    pub(crate) store: Arc<dyn RefreshStore>,
    pub(crate) codec: Arc<dyn RefreshCodec>,
    pub(crate) ttl_secs: u64,
}

impl IdentityService {
    /// Creates the service from its ports.
    #[must_use]
    pub(crate) fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        refresh: RefreshPolicy,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
            clock,
            ids,
            refresh,
        }
    }

    /// A fresh refresh token: the secret for the client and the record data to store.
    fn mint(&self) -> (RefreshGrant, RefreshSuccessor) {
        let token = self.refresh.codec.generate();
        let ttl = i64::try_from(self.refresh.ttl_secs).unwrap_or(i64::MAX);
        let record = RefreshSuccessor {
            id: self.ids.next(),
            token_hash: self.refresh.codec.hash(&token),
            expires_at: self
                .clock
                .now()
                .saturating_add(time::Duration::seconds(ttl)),
        };
        (
            RefreshGrant {
                token,
                max_age_secs: self.refresh.ttl_secs,
            },
            record,
        )
    }

    /// Registers a user.
    ///
    /// # Errors
    /// Validation, duplicate email or infrastructure errors.
    pub(crate) async fn register(
        &self,
        email: &str,
        display_name: &str,
        password: &str,
    ) -> Result<User, IdentityError> {
        let email = Email::parse(email)?;
        let display_name = DisplayName::parse(display_name)?;
        let password = Password::new(password)?;
        let hash = self.hasher.hash(password.expose()).await?;
        let mut user = User::register(
            UserId::from_uuid(self.ids.next()),
            email,
            display_name,
            hash,
            self.clock.now(),
        );
        self.users.insert(&mut user).await?;
        Ok(user)
    }

    /// Verifies credentials and issues an access token plus a refresh token.
    ///
    /// # Errors
    /// [`IdentityError::InvalidCredentials`] for any mismatch.
    pub(crate) async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(User, AccessToken, RefreshGrant), IdentityError> {
        let email = Email::parse(email).map_err(|_| IdentityError::InvalidCredentials)?;
        let password = Password::attempt(password);
        let user = self.users.find_by_email(&email).await?;
        let hash = user
            .as_ref()
            .map_or_else(|| self.hasher.dummy_hash(), User::password_hash);
        let ok = self.hasher.verify(password.expose(), hash).await;
        match user {
            Some(user) if ok => {
                let token = self.tokens.issue(user.id())?;
                let (grant, minted) = self.mint();
                let record = NewRefreshToken {
                    id: minted.id,
                    family_id: self.ids.next(),
                    user_id: user.id(),
                    token_hash: minted.token_hash,
                    expires_at: minted.expires_at,
                };
                self.refresh.store.insert(&record, self.clock.now()).await?;
                Ok((user, token, grant))
            }
            _ => Err(IdentityError::InvalidCredentials),
        }
    }

    /// Exchanges a refresh token for a new access token and a rotated refresh token.
    ///
    /// # Errors
    /// [`IdentityError::InvalidRefreshToken`] when the token is unknown, expired, revoked or was
    /// already used (which also revokes its family).
    pub(crate) async fn refresh(
        &self,
        presented: &str,
    ) -> Result<(User, AccessToken, RefreshGrant), IdentityError> {
        let hash = self.refresh.codec.hash(presented);
        let (grant, successor) = self.mint();
        let rotation = self
            .refresh
            .store
            .rotate(&hash, &successor, self.clock.now())
            .await?;
        let Rotation::Rotated { user_id } = rotation else {
            return Err(IdentityError::InvalidRefreshToken);
        };
        let user = self.get(user_id).await?;
        let token = self.tokens.issue(user.id())?;
        Ok((user, token, grant))
    }

    /// Revokes the session behind a refresh token. Unknown tokens are ignored.
    ///
    /// # Errors
    /// Infrastructure failures only.
    pub(crate) async fn logout(&self, presented: &str) -> Result<(), IdentityError> {
        let hash = self.refresh.codec.hash(presented);
        self.refresh
            .store
            .revoke_family(&hash, self.clock.now())
            .await?;
        Ok(())
    }

    /// Loads a user by id.
    ///
    /// # Errors
    /// [`IdentityError::NotFound`] when missing.
    pub(crate) async fn get(&self, id: UserId) -> Result<User, IdentityError> {
        self.users
            .find_by_id(id)
            .await?
            .ok_or(IdentityError::NotFound)
    }

    /// Updates the user's profile (`display_name` required; `bio` and `avatar_url` optional).
    ///
    /// # Errors
    /// Validation or infrastructure errors.
    pub(crate) async fn update_profile(
        &self,
        id: UserId,
        display_name: &str,
        bio: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<User, IdentityError> {
        let display_name = DisplayName::parse(display_name)?;
        let bio = bio.map(Bio::new).transpose()?;
        let avatar_url = avatar_url.map(AvatarUrl::parse).transpose()?;
        let now = self.clock.now();
        self.users
            .update_profile(id, &display_name, bio.as_ref(), avatar_url.as_ref(), now)
            .await?;
        self.get(id).await
    }

    /// Changes the user's password. Revokes all existing refresh tokens on success.
    ///
    /// # Errors
    /// [`IdentityError::WrongPassword`] when the old password does not match.
    pub(crate) async fn change_password(
        &self,
        id: UserId,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), IdentityError> {
        let user = self.get(id).await?;
        let old_matches = self.hasher.verify(old_password, user.password_hash()).await;
        let new_pw = Password::new(new_password)?;
        if !old_matches {
            return Err(IdentityError::WrongPassword);
        }
        let new_hash = self.hasher.hash(new_pw.expose()).await?;
        self.users.update_password(id, &new_hash).await?;
        self.users
            .revoke_all_refresh_tokens(id, self.clock.now())
            .await?;
        Ok(())
    }

    /// Schedules account deletion 30 days out; returns the execution date.
    ///
    /// # Errors
    /// [`IdentityError::DeletionAlreadyScheduled`] when already pending.
    pub(crate) async fn schedule_deletion(
        &self,
        id: UserId,
    ) -> Result<OffsetDateTime, IdentityError> {
        let mut user = self.get(id).await?;
        let event = user.schedule_deletion(self.clock.now())?;
        self.users
            .schedule_deletion(id, event.execute_after, event.scheduled_at)
            .await?;
        Ok(event.execute_after)
    }

    /// Cancels a pending deletion.
    ///
    /// # Errors
    /// [`IdentityError::NoPendingDeletion`] when nothing is pending.
    pub(crate) async fn cancel_deletion(&self, id: UserId) -> Result<(), IdentityError> {
        let mut user = self.get(id).await?;
        user.cancel_deletion()?;
        self.users.cancel_deletion(id).await?;
        Ok(())
    }

    /// Exports all personal data for the authenticated user.
    ///
    /// # Errors
    /// Infrastructure or not-found errors.
    pub(crate) async fn gdpr_export(
        &self,
        id: UserId,
        workspace: &dyn WorkspaceExportPort,
    ) -> Result<GdprExport, IdentityError> {
        let user = self.get(id).await?;
        let files = workspace
            .list_owned_files(id)
            .await
            .map_err(|_| IdentityError::Internal)?;
        let account = GdprAccount {
            id: user.id().as_uuid().to_string(),
            email: user.email().as_str().to_owned(),
            display_name: user.display_name().as_str().to_owned(),
            bio: user.bio().map(|b| b.as_str().to_owned()),
            avatar_url: user.avatar_url().map(|a| a.as_str().to_owned()),
            created_at: user.created_at().to_string(),
        };
        Ok(GdprExport { account, files })
    }
}

impl std::fmt::Debug for IdentityService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityService").finish_non_exhaustive()
    }
}
