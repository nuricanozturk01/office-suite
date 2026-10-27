//! API tests against a real Postgres (testcontainers), including IDOR checks.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use identity_contracts::UserId;
use workspace_contracts::{PermissionChecker, ResourceId, Role};

use axum::{
    Extension, Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use platform::http::BearerVerifier;
use serde_json::{Value, json};
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use tower::ServiceExt;
use uuid::Uuid;

/// Roles granted in a test, keyed by `(user, resource)`.
#[derive(Debug, Default)]
struct FakeChecker(Mutex<HashMap<(Uuid, Uuid), Role>>);

#[async_trait]
impl PermissionChecker for FakeChecker {
    async fn role(&self, user: UserId, resource: ResourceId) -> Option<Role> {
        self.0
            .lock()
            .unwrap()
            .get(&(user.as_uuid(), resource.as_uuid()))
            .copied()
    }
}

/// Test verifier: the bearer token is the user's UUID.
struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

async fn app_with_checker(
    checker: Arc<dyn PermissionChecker>,
) -> (Router, sqlx::PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &pdfs::migrator())
        .await
        .unwrap();
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    (
        pdfs::module(pool.clone(), checker)
            .router
            .layer(Extension(verifier)),
        pool,
        container,
    )
}

async fn app_with_pool() -> (Router, sqlx::PgPool, impl Sized) {
    app_with_checker(Arc::new(FakeChecker::default())).await
}

async fn app() -> (Router, impl Sized) {
    let (router, _pool, container) = app_with_pool().await;
    (router, container)
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    user: Option<Uuid>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(user) = user {
        req = req.header(header::AUTHORIZATION, format!("Bearer {user}"));
    }
    let body = body.map_or_else(Body::empty, |b| {
        req = std::mem::take(&mut req).header(header::CONTENT_TYPE, "application/json");
        Body::from(b.to_string())
    });
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn crud_pagination_and_isolation() {
    let (app, _db) = app().await;
    let (alice, mallory) = (Uuid::now_v7(), Uuid::now_v7());

    let (status, _) = send(&app, Method::GET, "/", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = send(
        &app,
        Method::POST,
        "/",
        Some(alice),
        Some(json!({"title": "  "})),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("validation_failed"))
    );

    let mut ids = Vec::new();
    for n in 0..3 {
        let (status, body) = send(
            &app,
            Method::POST,
            "/",
            Some(alice),
            Some(json!({"title": format!("Doc {n}")})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        ids.push(body["id"].as_str().unwrap().to_owned());
    }

    // Newest first, two pages of size 2 then 1.
    let (_, page1) = send(&app, Method::GET, "/?limit=2", Some(alice), None).await;
    let titles: Vec<_> = page1["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["title"].clone())
        .collect();
    assert_eq!(titles, [json!("Doc 2"), json!("Doc 1")]);
    let cursor = page1["next_cursor"].as_str().unwrap();
    let (_, page2) = send(
        &app,
        Method::GET,
        &format!("/?limit=2&cursor={cursor}"),
        Some(alice),
        None,
    )
    .await;
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    assert!(page2["next_cursor"].is_null());

    // Another user cannot see, read, change or delete (indistinguishable from missing).
    let target = format!("/{}", ids[0]);
    let (_, theirs) = send(&app, Method::GET, "/", Some(mallory), None).await;
    assert!(theirs["items"].as_array().unwrap().is_empty());
    for method in [Method::GET, Method::DELETE] {
        let (status, _) = send(&app, method, &target, Some(mallory), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, _) = send(
        &app,
        Method::PATCH,
        &target,
        Some(mallory),
        Some(json!({"title": "pwned"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Owner can rename then delete.
    let (status, body) = send(
        &app,
        Method::PATCH,
        &target,
        Some(alice),
        Some(json!({"title": "Renamed"})),
    )
    .await;
    assert_eq!(
        (status, body["title"].as_str()),
        (StatusCode::OK, Some("Renamed"))
    );
    let (status, _) = send(&app, Method::DELETE, &target, Some(alice), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, Method::GET, &target, Some(alice), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rename_and_delete_write_outbox_rows() {
    let (app, pool, _db) = app_with_pool().await;
    let alice = Uuid::now_v7();
    let (_, created) = send(
        &app,
        Method::POST,
        "/",
        Some(alice),
        Some(json!({"title": "Old"})),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    let (status, _) = send(
        &app,
        Method::PATCH,
        &format!("/{id}"),
        Some(alice),
        Some(json!({"title": "New"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, Method::DELETE, &format!("/{id}"), Some(alice), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let rows: Vec<(String, Value)> =
        sqlx::query_as("SELECT event_type, payload FROM pdfs.outbox ORDER BY created_at, id")
            .fetch_all(&pool)
            .await
            .unwrap();
    let types: Vec<_> = rows.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(types, ["PdfCreated", "PdfRenamed", "PdfDeleted"]);
    assert_eq!(rows[1].1["pdf_id"], json!(id));
    assert_eq!(rows[1].1["title"], "New");
    assert_eq!(rows[2].1["pdf_id"], json!(id));
}

#[tokio::test]
async fn shared_roles_map_to_pdf_permissions() {
    let checker = Arc::new(FakeChecker::default());
    let (app, _pool, _db) = app_with_checker(checker.clone()).await;
    let (alice, bob, mallory) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    let (_, created) = send(
        &app,
        Method::POST,
        "/",
        Some(alice),
        Some(json!({"title": "Plan"})),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let doc: Uuid = id.parse().unwrap();
    let uri = format!("/{id}");
    let rename = || Some(json!({"title": "Renamed"}));

    // Viewer: may read, may not change or delete. Strangers see nothing.
    checker.0.lock().unwrap().insert((bob, doc), Role::Viewer);
    assert_eq!(
        send(&app, Method::GET, &uri, Some(bob), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, Method::PATCH, &uri, Some(bob), rename()).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, Method::DELETE, &uri, Some(bob), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, Method::GET, &uri, Some(mallory), None).await.0,
        StatusCode::NOT_FOUND
    );

    // Editor: may also rename, but never delete.
    checker.0.lock().unwrap().insert((bob, doc), Role::Editor);
    assert_eq!(
        send(&app, Method::PATCH, &uri, Some(bob), rename()).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, Method::DELETE, &uri, Some(bob), None).await.0,
        StatusCode::NOT_FOUND
    );

    // Revoked: next request is denied.
    checker.0.lock().unwrap().remove(&(bob, doc));
    assert_eq!(
        send(&app, Method::GET, &uri, Some(bob), None).await.0,
        StatusCode::NOT_FOUND
    );
}
