//! Database pool and per-module migration helpers.

use secrecy::{ExposeSecret, SecretString};
use sqlx::{migrate::Migrator, postgres::PgPoolOptions};

pub use sqlx::PgPool;

/// Connects a Postgres pool.
///
/// # Errors
/// Returns the underlying [`sqlx::Error`] when the connection fails.
pub async fn connect(url: &SecretString) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(url.expose_secret())
        .await
}

/// Applies a module's migrations, tracked in the module's own schema.
///
/// # Errors
/// Returns the underlying [`sqlx::migrate::MigrateError`].
pub async fn migrate(
    pool: &PgPool,
    migrator: &Migrator,
) -> Result<(), sqlx::migrate::MigrateError> {
    migrator.run(pool).await
}

/// Adjusts a migrator so its history lives in `<schema>._sqlx_migrations`.
pub fn scope_to_schema(migrator: &mut Migrator, schema: &'static str) {
    migrator.create_schema(schema);
    migrator.dangerous_set_table_name(format!("{schema}._sqlx_migrations"));
}
