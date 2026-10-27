//! Live sync module: who has a resource open, and a push when somebody saves a new version.
//!
//! It knows resources only by id, so it serves every editor the same way. A client connects a
//! WebSocket per open resource (`/{resource_id}`, authenticated like the collaboration channel)
//! and receives [`livesync_contracts::ServerMessage`]s. The module reads two things from other
//! modules, both through contracts: access (`workspace_contracts::PermissionChecker`) and the
//! `PackageSaved` event of `storage`.

#![forbid(unsafe_code)]

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;

use std::{sync::Arc, time::Duration};

use axum::Router;
use identity_contracts::UserProfiles;
use platform::broadcast::BroadcastHandler;
use workspace_contracts::PermissionChecker;

use crate::{
    application::{LiveBus, LiveSyncService},
    infrastructure::{NoBus, RedisBus},
};

/// How instances of this module reach each other.
#[derive(Clone)]
pub struct BusHandle(Arc<dyn LiveBus>);

impl BusHandle {
    /// Single instance: presence is local, saves still reach everybody.
    #[must_use]
    pub fn none() -> Self {
        Self(Arc::new(NoBus))
    }

    /// Redis pub/sub shared with the other instances.
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
    /// How often an open connection re-checks that its user may still read the resource.
    pub recheck_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            recheck_interval: Duration::from_secs(30),
        }
    }
}

/// Constructed live sync module.
pub struct Module {
    /// Routes mounted under `/api/v1/livesync`; requires the bearer verifier extension.
    pub router: Router,
    /// Handlers to run on every instance for broadcast events (saves, revoked access).
    pub broadcast_handlers: Vec<Arc<dyn BroadcastHandler>>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("broadcast_handlers", &self.broadcast_handlers.len())
            .finish_non_exhaustive()
    }
}

/// Builds the module with default options; ports are injected by the composition root.
#[must_use]
pub fn module(
    checker: Arc<dyn PermissionChecker>,
    profiles: Arc<dyn UserProfiles>,
    bus: BusHandle,
) -> Module {
    module_with(checker, profiles, bus, Options::default())
}

/// Builds the module with explicit options.
#[must_use]
pub fn module_with(
    checker: Arc<dyn PermissionChecker>,
    profiles: Arc<dyn UserProfiles>,
    bus: BusHandle,
    options: Options,
) -> Module {
    let service = Arc::new(LiveSyncService::new(
        checker,
        profiles,
        bus.0,
        options.recheck_interval,
    ));
    Module {
        broadcast_handlers: api::broadcast_handlers(&service),
        router: api::router(service),
    }
}
