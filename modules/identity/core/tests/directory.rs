//! `UserDirectory` port against a real Postgres.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt;
use identity::Config;
use identity_contracts::UserId;
use secrecy::SecretString;
use serde_json::{Value, json};
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use tower::ServiceExt;
use workspace_contracts::{ExportError, FileExportRow, WorkspaceExportPort};

#[derive(Debug)]
struct NoopExport;

#[async_trait]
impl WorkspaceExportPort for NoopExport {
    async fn list_owned_files(&self, _owner: UserId) -> Result<Vec<FileExportRow>, ExportError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn directory_resolves_registered_emails_only() {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &identity::migrator())
        .await
        .unwrap();
    let config = Config {
        jwt_secret: SecretString::from("0123456789abcdef0123456789abcdef".to_owned()),
        access_token_ttl_secs: 60,
        refresh_token_ttl_secs: 3600,
        cookie_secure: false,
    };
    let module = identity::module(pool, &config, Arc::new(NoopExport)).unwrap();

    let body = json!({"email": "ada@example.com", "display_name": "Ada", "password": "correct horse battery"});
    let req = Request::post("/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = module.router.clone().oneshot(req).await.unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&bytes).unwrap();
    let id: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();

    let dir = &module.user_directory;
    assert_eq!(
        dir.find_by_email("ada@example.com")
            .await
            .map(|u| u.as_uuid()),
        Some(id)
    );
    assert_eq!(
        dir.find_by_email("  ADA@Example.com ")
            .await
            .map(|u| u.as_uuid()),
        Some(id)
    );
    assert_eq!(dir.find_by_email("nobody@example.com").await, None);
    assert_eq!(dir.find_by_email("not an email").await, None);
}
