//! Sharing API and projections against a real Postgres, including IDOR checks.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use axum::{
    Extension, Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use identity_contracts::{UserDirectory, UserId};
use platform::{
    event_bus::{EventEnvelope, EventHandler},
    http::BearerVerifier,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tower::ServiceExt;
use uuid::Uuid;
use workspace_contracts::{PermissionChecker, ResourceId, Role};

/// Test verifier: the bearer token is the user's UUID.
struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

#[derive(Debug)]
struct Directory(HashMap<&'static str, Uuid>);

#[async_trait]
impl UserDirectory for Directory {
    async fn find_by_email(&self, email: &str) -> Option<UserId> {
        self.0.get(email).copied().map(UserId::from_uuid)
    }
}

struct Env {
    app: Router,
    pool: PgPool,
    handlers: Vec<Arc<dyn EventHandler>>,
    checker: Arc<dyn PermissionChecker>,
    alice: Uuid,
    bob: Uuid,
    mallory: Uuid,
    _db: Box<dyn Send + Sync>,
}

async fn env() -> Env {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &platform::migrator())
        .await
        .unwrap();
    platform::db::migrate(&pool, &workspace::migrator())
        .await
        .unwrap();
    let (alice, bob, mallory) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    let directory = Directory(HashMap::from([
        ("alice@x.com", alice),
        ("bob@x.com", bob),
        ("mallory@x.com", mallory),
    ]));
    let module = workspace::module(pool.clone(), Arc::new(directory));
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    Env {
        app: module.router.layer(Extension(verifier)),
        pool,
        handlers: module.event_handlers,
        checker: module.permission_checker,
        alice,
        bob,
        mallory,
        _db: Box::new(container),
    }
}

fn ts(secs: i64) -> String {
    (OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
        .format(&Rfc3339)
        .unwrap()
}

impl Env {
    async fn project(&self, event_id: Uuid, event_type: &str, payload: Value) {
        let handler = self
            .handlers
            .iter()
            .find(|h| h.event_type() == event_type)
            .unwrap();
        let envelope = EventEnvelope {
            id: event_id,
            event_type: event_type.to_owned(),
            payload,
        };
        handler.handle(&envelope).await.unwrap();
    }

    async fn seed_document(&self, id: Uuid, owner: Uuid, title: &str) {
        self.project(
            Uuid::now_v7(),
            "DocumentCreated",
            json!({"document_id": id, "owner_id": owner, "title": title, "created_at": ts(10)}),
        )
        .await;
    }

    async fn send(
        &self,
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
        let res = self
            .app
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn role(&self, user: Uuid, resource: Uuid) -> Option<Role> {
        self.checker
            .role(UserId::from_uuid(user), ResourceId::from_uuid(resource))
            .await
    }
}

#[tokio::test]
async fn owner_shares_grantee_sees_it_and_revoke_removes_access() {
    let e = env().await;
    let doc = Uuid::now_v7();
    e.seed_document(doc, e.alice, "Plan").await;

    let (status, body) = e
        .send(
            Method::POST,
            &format!("/{doc}/shares"),
            Some(e.alice),
            Some(json!({"email": "bob@x.com", "role": "editor"})),
        )
        .await;
    assert_eq!(
        (status, body["role"].as_str(), body["email"].as_str()),
        (StatusCode::OK, Some("editor"), Some("bob@x.com"))
    );

    let (_, listed) = e
        .send(Method::GET, "/shared-with-me", Some(e.bob), None)
        .await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    assert_eq!(listed["items"][0]["id"], json!(doc));
    assert_eq!(
        (
            listed["items"][0]["role"].as_str(),
            listed["items"][0]["kind"].as_str()
        ),
        (Some("editor"), Some("document"))
    );
    assert_eq!(listed["items"][0]["title"], "Plan");

    let (_, shares) = e
        .send(Method::GET, &format!("/{doc}/shares"), Some(e.alice), None)
        .await;
    assert_eq!(shares["items"].as_array().unwrap().len(), 1);
    assert_eq!(shares["items"][0]["email"], "bob@x.com");

    assert_eq!(e.role(e.bob, doc).await, Some(Role::Editor));
    assert_eq!(e.role(e.alice, doc).await, Some(Role::Owner));
    assert_eq!(e.role(e.mallory, doc).await, None);

    let (status, _) = e
        .send(
            Method::DELETE,
            &format!("/{doc}/shares/{}", e.bob),
            Some(e.alice),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        e.role(e.bob, doc).await,
        None,
        "revoke applies on the next check"
    );
    let (_, after) = e
        .send(Method::GET, "/shared-with-me", Some(e.bob), None)
        .await;
    assert!(after["items"].as_array().unwrap().is_empty());

    let types: Vec<String> =
        sqlx::query_scalar("SELECT event_type FROM workspace.outbox ORDER BY created_at, id")
            .fetch_all(&e.pool)
            .await
            .unwrap();
    assert_eq!(types, ["ResourceShared", "PermissionRevoked"]);
}

#[tokio::test]
async fn strangers_and_viewers_cannot_manage_shares_and_cannot_probe_ids() {
    let e = env().await;
    let doc = Uuid::now_v7();
    e.seed_document(doc, e.alice, "Plan").await;
    let share = json!({"email": "mallory@x.com", "role": "viewer"});
    e.send(
        Method::POST,
        &format!("/{doc}/shares"),
        Some(e.alice),
        Some(json!({"email": "bob@x.com", "role": "viewer"})),
    )
    .await;

    let missing = Uuid::now_v7();
    for caller in [e.mallory, e.bob] {
        for target in [doc, missing] {
            let (s, b) = e
                .send(
                    Method::GET,
                    &format!("/{target}/shares"),
                    Some(caller),
                    None,
                )
                .await;
            assert_eq!(
                (s, b["code"].as_str()),
                (StatusCode::NOT_FOUND, Some("not_found"))
            );
            let (s, _) = e
                .send(
                    Method::POST,
                    &format!("/{target}/shares"),
                    Some(caller),
                    Some(share.clone()),
                )
                .await;
            assert_eq!(s, StatusCode::NOT_FOUND);
            let (s, _) = e
                .send(
                    Method::DELETE,
                    &format!("/{target}/shares/{}", e.bob),
                    Some(caller),
                    None,
                )
                .await;
            assert_eq!(s, StatusCode::NOT_FOUND);
        }
    }
    assert_eq!(
        e.role(e.bob, doc).await,
        Some(Role::Viewer),
        "failed attempts changed nothing"
    );

    let (s, _) = e
        .send(Method::GET, &format!("/{doc}/shares"), None, None)
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_shares_are_rejected_with_specific_codes() {
    let e = env().await;
    let doc = Uuid::now_v7();
    e.seed_document(doc, e.alice, "Plan").await;
    let path = format!("/{doc}/shares");
    let post = |body: Value| e.send(Method::POST, &path, Some(e.alice), Some(body));

    let (s, b) = post(json!({"email": "ghost@x.com", "role": "viewer"})).await;
    assert_eq!(
        (s, b["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("user_not_found"))
    );
    let (s, b) = post(json!({"email": "alice@x.com", "role": "editor"})).await;
    assert_eq!(
        (s, b["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("cannot_share_with_owner")
        )
    );
    let (s, _) = post(json!({"email": "bob@x.com", "role": "owner"})).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn projections_tolerate_replay_reordering_and_deletion() {
    let e = env().await;
    let doc = Uuid::now_v7();
    let event_id = Uuid::now_v7();
    let created =
        json!({"document_id": doc, "owner_id": e.alice, "title": "First", "created_at": ts(10)});

    // Same event delivered twice leaves exactly one row.
    e.project(event_id, "DocumentCreated", created.clone())
        .await;
    e.project(event_id, "DocumentCreated", created.clone())
        .await;
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM workspace.resources")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);

    // A rename that arrives before the (older) created event wins.
    let other = Uuid::now_v7();
    e.project(
        Uuid::now_v7(),
        "DocumentRenamed",
        json!({"document_id": other, "title": "Renamed", "at": ts(20)}),
    )
    .await;
    e.project(
        Uuid::now_v7(),
        "DocumentCreated",
        json!({"document_id": other, "owner_id": e.alice, "title": "Old", "created_at": ts(10)}),
    )
    .await;
    let title: String = sqlx::query_scalar("SELECT title FROM workspace.resources WHERE id = $1")
        .bind(other)
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(title, "Renamed");

    // Deleted resources lose all access and disappear from "shared with me".
    e.send(
        Method::POST,
        &format!("/{doc}/shares"),
        Some(e.alice),
        Some(json!({"email": "bob@x.com", "role": "viewer"})),
    )
    .await;
    assert_eq!(e.role(e.bob, doc).await, Some(Role::Viewer));
    e.project(
        Uuid::now_v7(),
        "DocumentDeleted",
        json!({"document_id": doc, "at": ts(30)}),
    )
    .await;
    assert_eq!(e.role(e.bob, doc).await, None);
    assert_eq!(e.role(e.alice, doc).await, None);
    let (_, listed) = e
        .send(Method::GET, "/shared-with-me", Some(e.bob), None)
        .await;
    assert!(listed["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn shared_with_me_paginates_with_a_stable_cursor() {
    let e = env().await;
    let mut ids = Vec::new();
    for n in 0..3 {
        let id = Uuid::now_v7();
        e.seed_document(id, e.alice, &format!("Doc {n}")).await;
        e.send(
            Method::POST,
            &format!("/{id}/shares"),
            Some(e.alice),
            Some(json!({"email": "bob@x.com", "role": "commenter"})),
        )
        .await;
        ids.push(id);
    }
    let (_, page1) = e
        .send(Method::GET, "/shared-with-me?limit=2", Some(e.bob), None)
        .await;
    let first: Vec<_> = page1["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].clone())
        .collect();
    assert_eq!(first, [json!(ids[2]), json!(ids[1])]);
    let cursor = page1["next_cursor"].as_str().unwrap();
    let (_, page2) = e
        .send(
            Method::GET,
            &format!("/shared-with-me?limit=2&cursor={cursor}"),
            Some(e.bob),
            None,
        )
        .await;
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    assert_eq!(page2["items"][0]["id"], json!(ids[0]));
    assert!(page2["next_cursor"].is_null());
}
