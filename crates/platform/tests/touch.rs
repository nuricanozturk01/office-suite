//! "Last edited" bump against a real Postgres: idempotent, monotonic, ignores deleted rows.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use platform::touch::{TouchTable, touch_updated_at};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

async fn pool() -> (PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &platform::migrator())
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE SCHEMA res;
         CREATE TABLE res.items (
             id uuid PRIMARY KEY, updated_at timestamptz NOT NULL, deleted_at timestamptz);",
    )
    .execute(&pool)
    .await
    .unwrap();
    (pool, container)
}

async fn insert(pool: &PgPool, at: OffsetDateTime, deleted: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO res.items (id, updated_at, deleted_at) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(at)
        .bind(deleted.then_some(at))
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn updated_at(pool: &PgPool, id: Uuid) -> OffsetDateTime {
    sqlx::query_scalar("SELECT updated_at FROM res.items WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn moves_forward_once_and_never_backward() {
    let (pool, _db) = pool().await;
    let table = TouchTable::new("res", "items").unwrap();
    let base = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    let id = insert(&pool, base, false).await;

    let later = base + Duration::hours(1);
    let event = Uuid::now_v7();
    touch_updated_at(&pool, &table, "test.touch", event, id, later)
        .await
        .unwrap();
    assert_eq!(updated_at(&pool, id).await, later);

    // The same event again does nothing, even with a different time.
    touch_updated_at(
        &pool,
        &table,
        "test.touch",
        event,
        id,
        later + Duration::hours(5),
    )
    .await
    .unwrap();
    assert_eq!(updated_at(&pool, id).await, later);

    // An older event never moves the row backward.
    touch_updated_at(&pool, &table, "test.touch", Uuid::now_v7(), id, base)
        .await
        .unwrap();
    assert_eq!(updated_at(&pool, id).await, later);
}

#[tokio::test]
async fn leaves_deleted_and_unknown_rows_alone() {
    let (pool, _db) = pool().await;
    let table = TouchTable::new("res", "items").unwrap();
    let base = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    let deleted = insert(&pool, base, true).await;

    touch_updated_at(
        &pool,
        &table,
        "test.touch",
        Uuid::now_v7(),
        deleted,
        base + Duration::hours(1),
    )
    .await
    .unwrap();
    assert_eq!(updated_at(&pool, deleted).await, base);

    // A resource this module does not know is fine: the event is for another table.
    touch_updated_at(
        &pool,
        &table,
        "test.touch",
        Uuid::now_v7(),
        Uuid::now_v7(),
        base,
    )
    .await
    .unwrap();
}

#[test]
fn rejects_unsafe_identifiers() {
    assert!(TouchTable::new("res", "items; DROP TABLE x").is_err());
    assert!(TouchTable::new("Res", "items").is_err());
    assert!(TouchTable::new("res", "").is_err());
}
