//! Workspace module: sharing, roles and the resource read-model.

#![forbid(unsafe_code)]

pub(crate) mod api;
pub(crate) mod application;
pub mod domain;
pub(crate) mod infrastructure;

use std::sync::Arc;

use axum::Router;
use identity_contracts::UserDirectory;
use platform::event_bus::EventHandler;
use sqlx::{PgPool, migrate::Migrator};
use workspace_contracts::PermissionChecker;

pub use infrastructure::PgWorkspaceExportAdapter;

use crate::{
    application::{ResourceStore, WorkspaceService},
    infrastructure::{PgPermissionChecker, PgWorkspaceRepository},
};

/// Embedded migrations of this module, tracked in the `workspace` schema.
#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    platform::db::scope_to_schema(&mut migrator, "workspace");
    migrator
}

/// `OpenAPI` description of [`Module::router`], with paths relative to its mount point.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    <api::ApiDoc as utoipa::OpenApi>::openapi()
}

/// Constructed workspace module.
pub struct Module {
    /// Routes mounted under `/api/v1/files`; requires the bearer verifier extension.
    pub router: Router,
    /// Port other modules use to resolve a user's role on a resource.
    pub permission_checker: Arc<dyn PermissionChecker>,
    /// Durable event handlers to register with the outbox relay.
    pub event_handlers: Vec<Arc<dyn EventHandler>>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("event_handlers", &self.event_handlers.len())
            .finish_non_exhaustive()
    }
}

/// Builds the module (composition happens in the caller).
#[must_use]
pub fn module(pool: PgPool, user_directory: Arc<dyn UserDirectory>) -> Module {
    let repository = Arc::new(PgWorkspaceRepository::new(pool.clone()));
    let service = Arc::new(WorkspaceService::new(
        repository.clone(),
        user_directory,
        Arc::new(shared_kernel::SystemClock),
    ));
    let store: Arc<dyn ResourceStore> = repository;
    Module {
        router: api::router(service),
        permission_checker: Arc::new(PgPermissionChecker::new(pool)),
        event_handlers: api::projections(&store),
    }
}
