//! Inbound HTTP adapter.

mod events;
mod routes;

pub(crate) use events::handlers;
pub(crate) use routes::{ApiDoc, router};
