//! API tests against a real Postgres (testcontainers), including version conflicts and IDOR.

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

struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

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

async fn app_with_checker(
    checker: Arc<dyn PermissionChecker>,
) -> (Router, sqlx::PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &presentations::migrator())
        .await
        .unwrap();
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    (
        presentations::module(pool.clone(), checker)
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
    let body = body.map_or_else(Body::empty, |value| {
        req = std::mem::take(&mut req).header(header::CONTENT_TYPE, "application/json");
        Body::from(value.to_string())
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
async fn saves_content_and_hides_other_owners() {
    let (app, _db) = app().await;
    let (alice, mallory) = (Uuid::now_v7(), Uuid::now_v7());
    let (status, created) = send(
        &app,
        Method::POST,
        "/",
        Some(alice),
        Some(json!({ "title": "Budget" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["version"], 1);
    let id = created["id"].as_str().unwrap();

    let content = json!({
        "width": 960, "height": 540,
        "slides": [{ "id": "s1", "elements": [
            { "id": "e1", "kind": "text", "x": 1, "y": 2, "w": 300, "h": 50, "text": "Merhaba" }
        ] }]
    });
    let (status, saved) = send(
        &app,
        Method::PUT,
        &format!("/{id}/content"),
        Some(alice),
        Some(json!({ "version": 1, "content": content })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["version"], 2);

    let (status, _) = send(
        &app,
        Method::PUT,
        &format!("/{id}/content"),
        Some(alice),
        Some(json!({ "version": 1, "content": content })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = send(&app, Method::GET, &format!("/{id}"), Some(mallory), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = send(&app, Method::GET, &format!("/{id}"), Some(alice), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["content"]["slides"][0]["elements"][0]["text"],
        "Merhaba"
    );
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

    let rows: Vec<(String, Value)> = sqlx::query_as(
        "SELECT event_type, payload FROM presentations.outbox ORDER BY created_at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let types: Vec<_> = rows.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        types,
        [
            "PresentationCreated",
            "PresentationRenamed",
            "PresentationDeleted"
        ]
    );
    assert_eq!(rows[1].1["presentation_id"], json!(id));
    assert_eq!(rows[1].1["title"], "New");
    assert_eq!(rows[2].1["presentation_id"], json!(id));
}

#[tokio::test]
async fn shared_roles_map_to_presentation_permissions() {
    let checker = Arc::new(FakeChecker::default());
    let (app, _pool, _db) = app_with_checker(checker.clone()).await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let (_, created) = send(
        &app,
        Method::POST,
        "/",
        Some(alice),
        Some(json!({"title": "Budget"})),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let sheet: Uuid = id.parse().unwrap();
    let content = json!({"width": 960, "height": 540, "slides": [{"id": "s1", "elements": []}]});
    let save = |version: i64| Some(json!({"version": version, "content": content}));

    checker.0.lock().unwrap().insert((bob, sheet), Role::Viewer);
    assert_eq!(
        send(&app, Method::GET, &format!("/{id}"), Some(bob), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            Method::PUT,
            &format!("/{id}/content"),
            Some(bob),
            save(1)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    checker.0.lock().unwrap().insert((bob, sheet), Role::Editor);
    assert_eq!(
        send(
            &app,
            Method::PUT,
            &format!("/{id}/content"),
            Some(bob),
            save(1)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, Method::DELETE, &format!("/{id}"), Some(bob), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );

    checker.0.lock().unwrap().remove(&(bob, sheet));
    assert_eq!(
        send(&app, Method::GET, &format!("/{id}"), Some(bob), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

async fn send_raw(
    app: &Router,
    method: Method,
    uri: &str,
    user: Option<Uuid>,
    body: Vec<u8>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(user) = user {
        req = req.header(header::AUTHORIZATION, format!("Bearer {user}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

#[tokio::test]
async fn imports_and_exports_pptx_with_owner_checks() {
    let (app, _db) = app().await;
    let (alice, mallory) = (Uuid::now_v7(), Uuid::now_v7());
    let deck = json!({
        "width": 960, "height": 540,
        "slides": [{ "id": "s1", "elements": [
            { "id": "e1", "kind": "text", "x": 1, "y": 2, "w": 300, "h": 50, "text": "Kira" }
        ] }]
    });
    let file = presentations_engine::export_pptx(&deck).unwrap();

    let (status, _, _) = send_raw(&app, Method::POST, "/import", None, file.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, body) = send_raw(
        &app,
        Method::POST,
        "/import?name=B%C3%BCt%C3%A7e",
        Some(alice),
        file,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let imported: Value = serde_json::from_slice(&body).unwrap();
    let sheet = &imported["presentation"];
    assert_eq!(sheet["title"], "Bütçe");
    assert_eq!(sheet["version"], 1);
    assert_eq!(sheet["content"]["slides"][0]["elements"][0]["text"], "Kira");
    let id = sheet["id"].as_str().unwrap();

    let uri = format!("/{id}/export.pptx");
    let (status, headers, bytes) = send_raw(&app, Method::GET, &uri, Some(alice), vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("presentationml.presentation")
    );
    let disposition = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(disposition.contains("filename*=UTF-8''B%C3%BCt%C3%A7e.pptx"));
    assert!(presentations_engine::import_pptx(&bytes).is_ok());

    let (status, _, _) = send_raw(&app, Method::GET, &uri, Some(mallory), vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send_raw(&app, Method::GET, &uri, None, vec![]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_garbage_and_oversized_uploads() {
    let (app, _db) = app().await;
    let alice = Uuid::now_v7();
    let (status, _, _) = send_raw(
        &app,
        Method::POST,
        "/import?name=x",
        Some(alice),
        b"not a zip".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let huge = vec![0_u8; presentations_engine::MAX_PPTX_BYTES + 1];
    let (status, _, _) = send_raw(&app, Method::POST, "/import?name=x", Some(alice), huge).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}
