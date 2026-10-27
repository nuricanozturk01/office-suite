//! API tests against a real Postgres (testcontainers).

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use identity::Config;
use identity_contracts::{TokenVerifier, UserId};
use platform::http::BearerVerifier;
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

struct Adapter(Arc<dyn TokenVerifier>);

impl BearerVerifier for Adapter {
    fn verify(&self, token: &str) -> Option<uuid::Uuid> {
        self.0.verify(token).map(|id| id.as_uuid())
    }
}

fn test_config() -> Config {
    Config {
        jwt_secret: SecretString::from("0123456789abcdef0123456789abcdef".to_owned()),
        access_token_ttl_secs: 60,
        refresh_token_ttl_secs: 3600,
        cookie_secure: false,
    }
}

async fn app() -> (Router, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &identity::migrator())
        .await
        .unwrap();
    let module = identity::module(pool, &test_config(), Arc::new(NoopExport)).unwrap();
    let verifier: Arc<dyn BearerVerifier> = Arc::new(Adapter(module.token_verifier));
    (module.router.layer(Extension(verifier)), container)
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn post(path: &str, body: &Value) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn register_login_me_and_failure_modes() {
    let (app, _db) = app().await;
    let creds = json!({"email": "Ada@Example.com", "display_name": "Ada", "password": "correct horse battery"});

    let (status, body) = send(&app, post("/register", &creds)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["email"], "ada@example.com");
    assert!(body.get("password_hash").is_none());

    let (status, body) = send(&app, post("/register", &creds)).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::CONFLICT, Some("email_taken"))
    );

    let weak = json!({"email": "b@example.com", "display_name": "B", "password": "short"});
    let (status, _) = send(&app, post("/register", &weak)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let bad = json!({"email": "ada@example.com", "password": "wrong password!!"});
    let (status, _) = send(&app, post("/login", &bad)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let unknown = json!({"email": "nobody@example.com", "password": "wrong password!!"});
    let (status, _) = send(&app, post("/login", &unknown)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let ok = json!({"email": "ADA@example.com", "password": "correct horse battery"});
    let (status, body) = send(&app, post("/login", &ok)).await;
    assert_eq!(status, StatusCode::OK);
    let token = body["access_token"].as_str().unwrap().to_owned();

    let me = Request::get("/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, me).await;
    assert_eq!(
        (status, body["display_name"].as_str()),
        (StatusCode::OK, Some("Ada"))
    );

    let (status, _) = send(&app, Request::get("/me").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

fn post_with(path: &str, body: Option<&Value>, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::post(path);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

/// Status, `Set-Cookie` header values and JSON body.
async fn send_full(app: &Router, req: Request<Body>) -> (StatusCode, Vec<String>, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookies = res
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        cookies,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// `name=value` part of the refresh cookie in `Set-Cookie` headers.
fn refresh_cookie(cookies: &[String]) -> String {
    cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .and_then(|c| c.split(';').next())
        .unwrap()
        .to_owned()
}

const CSRF: (&str, &str) = ("x-requested-with", "office");

#[tokio::test]
async fn refresh_tokens_rotate_detect_reuse_and_log_out() {
    let (app, _db) = app().await;
    let creds = json!({"email": "grace@example.com", "display_name": "Grace", "password": "correct horse battery"});
    send(&app, post("/register", &creds)).await;
    let login = json!({"email": "grace@example.com", "password": "correct horse battery"});

    // Login hands out an HttpOnly, strict, path-scoped refresh cookie.
    let (status, cookies, _) = send_full(&app, post("/login", &login)).await;
    assert_eq!(status, StatusCode::OK);
    let raw = cookies
        .iter()
        .find(|c| c.starts_with("refresh_token="))
        .unwrap();
    for attribute in [
        "HttpOnly",
        "SameSite=Strict",
        "Path=/api/v1/auth",
        "Max-Age=",
    ] {
        assert!(raw.contains(attribute), "{attribute} missing in {raw}");
    }
    let first = refresh_cookie(&cookies);

    // Refresh rotates the cookie and returns a usable access token.
    let (status, cookies, body) = send_full(
        &app,
        post_with("/refresh", None, &[CSRF, ("cookie", &first)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let second = refresh_cookie(&cookies);
    assert_ne!(first, second);
    let token = body["access_token"].as_str().unwrap();
    let me = Request::get("/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(&app, me).await.0, StatusCode::OK);

    // Replaying the old cookie is theft: it fails and kills the whole family.
    let (status, _, body) = send_full(
        &app,
        post_with("/refresh", None, &[CSRF, ("cookie", &first)]),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("invalid_refresh_token"))
    );
    let (status, _, _) = send_full(
        &app,
        post_with("/refresh", None, &[CSRF, ("cookie", &second)]),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the rotated token died with its family"
    );

    // Cookie-authenticated endpoints demand the custom header (CSRF defence).
    let (_, cookies, _) = send_full(&app, post("/login", &login)).await;
    let third = refresh_cookie(&cookies);
    let (status, _, _) = send_full(&app, post_with("/refresh", None, &[("cookie", &third)])).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send_full(&app, post_with("/refresh", None, &[CSRF])).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "no cookie, no session");

    // Logout revokes the family and expires the cookie.
    let (status, cookies, _) = send_full(
        &app,
        post_with("/logout", None, &[CSRF, ("cookie", &third)]),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        cookies
            .iter()
            .any(|c| c.starts_with("refresh_token=;") && c.contains("Max-Age=0"))
    );
    let (status, _, _) = send_full(
        &app,
        post_with("/refresh", None, &[CSRF, ("cookie", &third)]),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_refresh_tokens_are_rejected() {
    // A zero lifetime makes every refresh token expire immediately.
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
        refresh_token_ttl_secs: 0,
        ..test_config()
    };
    let app = identity::module(pool, &config, Arc::new(NoopExport))
        .unwrap()
        .router;
    let creds = json!({"email": "old@example.com", "display_name": "Old", "password": "correct horse battery"});
    send(&app, post("/register", &creds)).await;
    let login = json!({"email": "old@example.com", "password": "correct horse battery"});
    let (_, cookies, _) = send_full(&app, post("/login", &login)).await;
    let cookie = refresh_cookie(&cookies);
    let (status, _, _) = send_full(
        &app,
        post_with("/refresh", None, &[CSRF, ("cookie", &cookie)]),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
