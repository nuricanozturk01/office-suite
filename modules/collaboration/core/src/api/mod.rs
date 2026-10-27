//! Inbound adapter: WebSocket endpoint.

mod events;
mod ws;

pub(crate) use events::broadcast_handlers;
pub(crate) use ws::router;
