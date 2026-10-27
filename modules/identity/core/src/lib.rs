//! Identity module: users, registration, login and access tokens.

#![forbid(unsafe_code)]

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod directory;
pub mod domain;
pub(crate) mod infrastructure;

use std::sync::Arc;

use axum::Router;
use identity_contracts::{TokenVerifier, UserDirectory, UserProfiles};
use sqlx::{PgPool, migrate::Migrator};

use crate::{
    application::{IdentityService, RefreshPolicy},
    infrastructure::{
        Argon2Hasher, JwtTokens, PgRefreshStore, PgUserRepository, RandomRefreshCodec,
    },
};

pub use workspace_contracts::WorkspaceExportPort;

/// Embedded migrations of this module, tracked in the `identity` schema.
#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    platform::db::scope_to_schema(&mut migrator, "identity");
    migrator
}

/// `OpenAPI` description of [`Module::router`], with paths relative to its mount point.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    <api::ApiDoc as utoipa::OpenApi>::openapi()
}

/// The module could not be initialized.
#[derive(Debug, thiserror::Error)]
#[error("failed to initialize identity module")]
pub struct InitError;

/// Constructed identity module: HTTP router plus the token verification port.
#[derive(Debug)]
pub struct Module {
    /// Routes mounted under `/api/v1/auth`.
    pub router: Router,
    /// Port other modules use to authenticate requests.
    pub token_verifier: Arc<dyn TokenVerifier>,
    /// Port other modules use to look users up by email.
    pub user_directory: Arc<dyn UserDirectory>,
    /// Port other modules use to label users by display name.
    pub user_profiles: Arc<dyn UserProfiles>,
}

/// Module wiring settings.
#[derive(Debug)]
pub struct Config {
    /// HMAC secret for access tokens.
    pub jwt_secret: secrecy::SecretString,
    /// Access token lifetime in seconds.
    pub access_token_ttl_secs: u64,
    /// Refresh token (and refresh cookie) lifetime in seconds.
    pub refresh_token_ttl_secs: u64,
    /// Marks the refresh cookie `Secure`; must be on wherever the API is served over HTTPS.
    pub cookie_secure: bool,
}

/// Builds the module (composition happens in the caller).
///
/// # Errors
/// When password hashing cannot be initialized.
pub fn module(
    pool: PgPool,
    config: &Config,
    workspace_export: Arc<dyn WorkspaceExportPort>,
) -> Result<Module, InitError> {
    let tokens = Arc::new(JwtTokens::new(
        &config.jwt_secret,
        config.access_token_ttl_secs,
    ));
    let users = Arc::new(PgUserRepository::new(pool.clone()));
    let directory = Arc::new(directory::RepositoryDirectory(users.clone()));
    let service = Arc::new(IdentityService::new(
        users,
        Arc::new(Argon2Hasher::new().map_err(|_| InitError)?),
        tokens.clone(),
        Arc::new(shared_kernel::SystemClock),
        Arc::new(shared_kernel::UuidV7Generator),
        RefreshPolicy {
            store: Arc::new(PgRefreshStore::new(pool)),
            codec: Arc::new(RandomRefreshCodec),
            ttl_secs: config.refresh_token_ttl_secs,
        },
    ));
    Ok(Module {
        router: api::router(api::ApiState {
            service,
            cookie_secure: config.cookie_secure,
            workspace_export,
        }),
        token_verifier: tokens,
        user_directory: directory.clone(),
        user_profiles: directory,
    })
}
