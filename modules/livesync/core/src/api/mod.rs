//! Inbound adapters: the WebSocket endpoint and broadcast event handlers.

mod events;
mod ws;

pub(crate) use events::broadcast_handlers;
pub(crate) use ws::router;
