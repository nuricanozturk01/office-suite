//! Storage module: the native file (`.docx`, `.xlsx`, `.pptx`, `.pdf`) behind a resource.

#![forbid(unsafe_code)]

mod api;
mod application;
mod domain;
mod infrastructure;

use std::sync::Arc;

use axum::Router;
use identity_contracts::UserProfiles;
use sqlx::{PgPool, migrate::Migrator};
use storage_contracts::PackageMerger;
use workspace_contracts::PermissionChecker;

use crate::{application::StorageService, infrastructure::PgPackageRepository};

/// Embedded migrations of this module, tracked in the `storage` schema.
#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    platform::db::scope_to_schema(&mut migrator, "storage");
    migrator
}

/// `OpenAPI` description of [`Module::router`], with paths relative to its mount point.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    <api::ApiDoc as utoipa::OpenApi>::openapi()
}

/// Constructed storage module.
#[derive(Debug)]
pub struct Module {
    /// Routes mounted under `/api/v1/packages`; requires the bearer verifier extension.
    pub router: Router,
}

/// Builds the module (composition happens in the caller).
///
/// `mergers` combine concurrent edits of the formats they handle; a format without one refuses
/// a save based on an old version.
#[must_use]
pub fn module(
    pool: PgPool,
    checker: Arc<dyn PermissionChecker>,
    profiles: Arc<dyn UserProfiles>,
    mergers: Vec<Arc<dyn PackageMerger>>,
) -> Module {
    let service = Arc::new(StorageService::new(
        Arc::new(PgPackageRepository::new(pool)),
        checker,
        Arc::new(shared_kernel::SystemClock),
        profiles,
        mergers,
    ));
    Module {
        router: api::router(service),
    }
}
