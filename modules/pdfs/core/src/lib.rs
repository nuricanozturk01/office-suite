//! PDF module: domain, application, infrastructure and HTTP adapter.

#![forbid(unsafe_code)]

pub(crate) mod api;
pub(crate) mod application;
pub mod domain;
pub(crate) mod infrastructure;

use std::sync::Arc;

use axum::Router;
use platform::event_bus::EventHandler;
use sqlx::{PgPool, migrate::Migrator};
use workspace_contracts::PermissionChecker;

use crate::{
    application::{OwnerOrShared, PdfService},
    infrastructure::PgPdfRepository,
};

/// Embedded migrations of this module, tracked in the `pdfs` schema.
#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    platform::db::scope_to_schema(&mut migrator, "pdfs");
    migrator
}

/// `OpenAPI` description of [`Module::router`], with paths relative to its mount point.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    <api::ApiDoc as utoipa::OpenApi>::openapi()
}

/// Constructed pdfs module.
pub struct Module {
    /// Routes mounted under `/api/v1/pdfs`; requires the bearer verifier extension.
    pub router: Router,
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
pub fn module(pool: PgPool, checker: Arc<dyn PermissionChecker>) -> Module {
    let service = Arc::new(PdfService::new(
        Arc::new(PgPdfRepository::new(pool.clone())),
        Arc::new(OwnerOrShared::new(checker)),
        Arc::new(shared_kernel::SystemClock),
        Arc::new(shared_kernel::UuidV7Generator),
    ));
    Module {
        router: api::router(service),
        event_handlers: api::handlers(pool),
    }
}
