//! API tests against a real Postgres (testcontainers), including IDOR checks.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
    Extension, Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use identity_contracts::{UserId, UserProfiles};
use platform::http::BearerVerifier;
use storage_contracts::{MergeError, PackageMerger};
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use tower::ServiceExt;
use uuid::Uuid;
use workspace_contracts::{PermissionChecker, ResourceId, Role};

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

#[derive(Debug)]
struct FakeProfiles;

#[async_trait]
impl UserProfiles for FakeProfiles {
    async fn display_name(&self, user: UserId) -> Option<String> {
        Some(format!("user-{}", &user.as_uuid().to_string()[..4]))
    }
}

struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

async fn app(checker: Arc<FakeChecker>) -> (Router, impl Sized) {
    let (router, _pool, container) = app_with_pool(checker).await;
    (router, container)
}

async fn app_with_pool(checker: Arc<FakeChecker>) -> (Router, sqlx::PgPool, impl Sized) {
    app_with_mergers(checker, Vec::new()).await
}

async fn app_with_mergers(
    checker: Arc<FakeChecker>,
    mergers: Vec<Arc<dyn PackageMerger>>,
) -> (Router, sqlx::PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &storage::migrator())
        .await
        .unwrap();
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    (
        storage::module(pool.clone(), checker, Arc::new(FakeProfiles), mergers)
            .router
            .layer(Extension(verifier)),
        pool,
        container,
    )
}

async fn send(
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
async fn stores_versions_and_guards_by_role() {
    let checker = Arc::new(FakeChecker::default());
    let (app, _db) = app(checker.clone()).await;
    let (owner, viewer, stranger, resource) = (
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    );
    {
        let mut roles = checker.0.lock().unwrap();
        roles.insert((owner, resource), Role::Owner);
        roles.insert((viewer, resource), Role::Viewer);
    }
    let uri = format!("/{resource}");
    let put = |name: &str, base: Option<i64>| {
        let base = base.map_or(String::new(), |v| format!("&base_version={v}"));
        format!("{uri}?name={name}{base}")
    };

    let (status, _, _) = send(&app, Method::GET, &uri, None, vec![]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(&app, Method::GET, &uri, Some(owner), vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, body) = send(
        &app,
        Method::PUT,
        &put("Rapor.docx", None),
        Some(owner),
        b"v1".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["version"],
        1
    );

    let (status, headers, body) = send(&app, Method::GET, &uri, Some(viewer), vec![]).await;
    assert_eq!((status, body.as_slice()), (StatusCode::OK, &b"v1"[..]));
    assert_eq!(headers["x-package-name"], "Rapor.docx");
    assert_eq!(headers["x-package-version"], "1");

    let (status, _, _) = send(
        &app,
        Method::PUT,
        &put("x.docx", None),
        Some(viewer),
        b"no".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(&app, Method::GET, &uri, Some(stranger), vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(
        &app,
        Method::PUT,
        &put("Rapor.docx", Some(1)),
        Some(owner),
        b"v2".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &app,
        Method::PUT,
        &put("Rapor.docx", Some(1)),
        Some(owner),
        b"v3".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _, _) = send(
        &app,
        Method::PUT,
        &put("..%2Fevil", None),
        Some(owner),
        b"x".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = send(&app, Method::PUT, &put("a.docx", None), Some(owner), vec![]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn every_save_emits_package_saved_with_the_new_version() {
    let checker = Arc::new(FakeChecker::default());
    let (app, pool, _db) = app_with_pool(checker.clone()).await;
    let (owner, resource) = (Uuid::now_v7(), Uuid::now_v7());
    checker
        .0
        .lock()
        .unwrap()
        .insert((owner, resource), Role::Owner);

    for _ in 0..2 {
        let (status, ..) = send(
            &app,
            Method::PUT,
            &format!("/{resource}?name=a.docx"),
            Some(owner),
            b"PK".to_vec(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT event_type, payload FROM storage.outbox ORDER BY created_at, id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 2);
    for (index, (event_type, payload)) in rows.iter().enumerate() {
        assert_eq!(event_type, "PackageSaved");
        assert_eq!(payload["resource_id"], resource.to_string());
        assert_eq!(payload["version"], index + 1);
    }

    // A rejected save (stale base version) emits nothing.
    let (status, ..) = send(
        &app,
        Method::PUT,
        &format!("/{resource}?name=a.docx&base_version=1"),
        Some(owner),
        b"PK".to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM storage.outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

/// Joins the two sides with `+`; refuses when ours says it conflicts.
#[derive(Debug)]
struct JoiningMerger;

impl PackageMerger for JoiningMerger {
    fn handles(&self, name: &str) -> bool {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("docx"))
    }

    fn merge(&self, base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Vec<u8>, MergeError> {
        if ours.starts_with(b"clash") {
            return Err(MergeError::Conflict);
        }
        Ok([base, b"/", ours, b"+", theirs].concat())
    }
}

fn version_of(body: &[u8]) -> (i64, bool) {
    let json: serde_json::Value = serde_json::from_slice(body).unwrap();
    (
        json["version"].as_i64().unwrap(),
        json["merged"].as_bool().unwrap(),
    )
}

#[tokio::test]
async fn history_lists_restores_and_merges_stale_saves() {
    let checker = Arc::new(FakeChecker::default());
    let (app, _pool, _db) = app_with_mergers(checker.clone(), vec![Arc::new(JoiningMerger)]).await;
    let (owner, viewer, resource) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    {
        let mut roles = checker.0.lock().unwrap();
        roles.insert((owner, resource), Role::Owner);
        roles.insert((viewer, resource), Role::Viewer);
    }
    let save = |query: &str, body: &[u8]| (format!("/{resource}?{query}"), body.to_vec());
    for (query, body) in [
        save("name=a.docx", b"one"),
        save("name=a.docx&base_version=1", b"two"),
    ] {
        let (status, ..) = send(&app, Method::PUT, &query, Some(owner), body).await;
        assert_eq!(status, StatusCode::OK);
    }

    // Stale save without `merge` is refused; with it, base(1) and theirs(2) are combined.
    let (q, b) = save("name=a.docx&base_version=1", b"mine");
    let (status, ..) = send(&app, Method::PUT, &q, Some(owner), b.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (q, b) = save("name=a.docx&base_version=1&merge=true", b"mine");
    let (status, _, body) = send(&app, Method::PUT, &q, Some(owner), b).await;
    assert_eq!((status, version_of(&body)), (StatusCode::OK, (3, true)));
    let (_, _, current) = send(
        &app,
        Method::GET,
        &format!("/{resource}"),
        Some(owner),
        vec![],
    )
    .await;
    assert_eq!(current, b"one/mine+two");

    // A real clash and a format without a merger stay conflicts.
    let (q, b) = save("name=a.docx&base_version=1&merge=true", b"clash");
    let (status, ..) = send(&app, Method::PUT, &q, Some(owner), b).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (q, b) = save("name=a.xlsx&base_version=1&merge=true", b"x");
    let (status, ..) = send(&app, Method::PUT, &q, Some(owner), b).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Anyone who can read lists and downloads versions; only writers restore.
    let (status, _, body) = send(
        &app,
        Method::GET,
        &format!("/{resource}/versions"),
        Some(viewer),
        vec![],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list["can_restore"], false, "viewers cannot restore");
    assert!(
        list["versions"][0]["saved_by_name"]
            .as_str()
            .unwrap()
            .starts_with("user-")
    );
    let versions: Vec<i64> = list["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["version"].as_i64().unwrap())
        .collect();
    assert_eq!(versions, [3, 2, 1]);
    let (status, headers, body) = send(
        &app,
        Method::GET,
        &format!("/{resource}/versions/1"),
        Some(viewer),
        vec![],
    )
    .await;
    assert_eq!((status, body.as_slice()), (StatusCode::OK, &b"one"[..]));
    assert_eq!(headers["x-package-version"], "1");
    let restore = format!("/{resource}/versions/1/restore");
    let (status, ..) = send(&app, Method::POST, &restore, Some(viewer), vec![]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, body) = send(&app, Method::POST, &restore, Some(owner), vec![]).await;
    assert_eq!((status, version_of(&body)), (StatusCode::OK, (4, false)));
    let (_, _, current) = send(
        &app,
        Method::GET,
        &format!("/{resource}"),
        Some(owner),
        vec![],
    )
    .await;
    assert_eq!(current, b"one");
    let (status, ..) = send(
        &app,
        Method::GET,
        &format!("/{resource}/versions/99"),
        Some(owner),
        vec![],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
