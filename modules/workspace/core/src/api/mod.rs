//! Inbound adapters: HTTP routes and event projections.

mod events;
mod routes;

pub(crate) use events::projections;
pub(crate) use routes::{ApiDoc, router};
