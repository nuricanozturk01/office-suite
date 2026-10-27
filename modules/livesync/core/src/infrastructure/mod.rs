//! Adapters for the ports of the application layer.

mod bus;

pub(crate) use bus::{NoBus, RedisBus};
