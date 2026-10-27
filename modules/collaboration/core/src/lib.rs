//! Collaboration module: real-time Yjs sync over WebSocket with durable update storage.

#![forbid(unsafe_code)]

pub(crate) mod api;
pub(crate) mod application;
pub mod domain;
pub(crate) mod infrastructure;

use std::{sync::Arc, time::Duration};

use axum::Router;
use documents_contracts::DocumentAccess;
use platform::broadcast::BroadcastHandler;
use sqlx::{PgPool, migrate::Migrator};

use crate::{
    application::{CollaborationService, RoomEnv, UpdateBus},
    infrastructure::{NoBus, PgUpdateStore, RedisBus},
};

/// Embedded migrations of this module, tracked in the `collaboration` schema.
#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    platform::db::scope_to_schema(&mut migrator, "collaboration");
    migrator
}

/// How instances of this module reach each other.
#[derive(Clone)]
pub struct BusHandle(Arc<dyn UpdateBus>);

impl BusHandle {
    /// Single instance: nothing is shared, the database catch-up still applies.
    #[must_use]
    pub fn none() -> Self {
        Self(Arc::new(NoBus))
    }

    /// Redis pub/sub fan-out shared with the other instances.
    #[must_use]
    pub fn redis(bus: Arc<platform::docbus::DocBus>) -> Self {
        Self(Arc::new(RedisBus::new(bus)))
    }
}

impl std::fmt::Debug for BusHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusHandle").finish_non_exhaustive()
    }
}

/// Tunables of the module.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// How often a room re-reads the durable log for updates other instances appended.
    pub catch_up_interval: Duration,
    /// How often an open session re-checks that its user still has the access they joined with.
    pub recheck_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            catch_up_interval: Duration::from_secs(5),
            recheck_interval: Duration::from_secs(30),
        }
    }
}

/// Constructed collaboration module.
pub struct Module {
    /// Routes mounted under `/api/v1/collab`; requires the bearer verifier extension.
    pub router: Router,
    /// Handlers to run on every instance for broadcast events (closing revoked sessions).
    pub broadcast_handlers: Vec<Arc<dyn BroadcastHandler>>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("broadcast_handlers", &self.broadcast_handlers.len())
            .finish_non_exhaustive()
    }
}

/// Builds the module with default options; document access is injected by the composition root.
#[must_use]
pub fn module(pool: PgPool, access: Arc<dyn DocumentAccess>, bus: BusHandle) -> Module {
    module_with(pool, access, bus, Options::default())
}

/// Builds the module with explicit options.
#[must_use]
pub fn module_with(
    pool: PgPool,
    access: Arc<dyn DocumentAccess>,
    bus: BusHandle,
    options: Options,
) -> Module {
    let env = RoomEnv {
        store: Arc::new(PgUpdateStore::new(pool)),
        bus: bus.0,
        instance: shared_kernel::Uuid::now_v7(),
        catch_up: options.catch_up_interval,
    };
    let service = Arc::new(CollaborationService::new(
        env,
        access,
        options.recheck_interval,
    ));
    Module {
        broadcast_handlers: api::broadcast_handlers(&service),
        router: api::router(service),
    }
}
