//! Infrastructure helpers shared by modules and binaries: settings, database, HTTP plumbing.

#![forbid(unsafe_code)]

pub mod background;
pub mod broadcast;
pub mod db;
pub mod docbus;
pub mod event_bus;
pub mod http;
pub mod outbox;
pub mod pagination;
pub mod settings;
pub mod touch;

/// Embedded migrations of the platform schema (idempotency ledger).
#[must_use]
pub fn migrator() -> sqlx::migrate::Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    db::scope_to_schema(&mut migrator, "platform");
    migrator
}
