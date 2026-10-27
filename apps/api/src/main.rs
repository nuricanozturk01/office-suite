//! Composition root for the HTTP API.

#![forbid(unsafe_code)]

mod bearer;
mod openapi;

use std::{io::Write, sync::Arc};

use anyhow::Context;
use axum::{
    Extension, Json, Router,
    http::{HeaderName, HeaderValue, Method, header},
    response::IntoResponse,
    routing::get,
};
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use platform::{
    background::{EventRuntime, EventRuntimeConfig},
    db,
    event_bus::Subscriptions,
    http::BearerVerifier,
    settings::Settings,
};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn openapi_json() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        openapi::to_json().unwrap_or_default(),
    )
}

/// Cross-origin access for the separately hosted web app. Credentials are allowed so the refresh
/// cookie travels with requests. `x-requested-with` must stay listed: the refresh endpoints
/// require it as their CSRF guard, and a non-simple header makes the browser send a preflight
/// that only allowed origins pass. An empty origin list allows nothing.
fn cors(origins: &[String]) -> anyhow::Result<CorsLayer> {
    let origins = origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin).with_context(|| format!("CORS origin {origin}"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-package-name"),
            HeaderName::from_static("x-package-version"),
        ])
        .expose_headers([
            header::CONTENT_DISPOSITION,
            HeaderName::from_static("x-package-name"),
            HeaderName::from_static("x-package-version"),
        ])
        .max_age(Duration::from_mins(10)))
}

/// Mounts every module's routes under its prefix and adds the cross-cutting layers.
fn app(
    verifier: Arc<dyn BearerVerifier>,
    routes: Vec<(&'static str, Router)>,
    cors: CorsLayer,
) -> Router {
    let base = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/openapi.json", get(openapi_json));
    routes
        .into_iter()
        .fold(base, |router, (prefix, nested)| router.nest(prefix, nested))
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(Extension(verifier))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

/// Applies every module's migrations, each in its own schema.
async fn migrate_all(pool: &db::PgPool) -> anyhow::Result<()> {
    let migrators = [
        ("platform", platform::migrator()),
        ("identity", identity::migrator()),
        ("documents", documents::migrator()),
        ("workspace", workspace::migrator()),
        ("spreadsheets", spreadsheets::migrator()),
        ("presentations", presentations::migrator()),
        ("pdfs", pdfs::migrator()),
        ("storage", storage::migrator()),
        ("collaboration", collaboration::migrator()),
    ];
    for (name, migrator) in migrators {
        db::migrate(pool, &migrator)
            .await
            .with_context(|| format!("migrate {name}"))?;
    }
    Ok(())
}

/// Loads a repo-root `.env` without overriding variables already set in the shell.
fn load_dotenv() -> anyhow::Result<()> {
    let mut dir = std::env::current_dir().context("current directory")?;
    loop {
        let path = dir.join(".env");
        if path.is_file() {
            dotenvy::from_path(&path).with_context(|| format!("read {}", path.display()))?;
            return Ok(());
        }
        if !dir.pop() {
            return Ok(());
        }
    }
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        tracing::warn!("failed to install ctrl-c handler");
    }
}

/// Formats that can combine two concurrent saves; PDF has no merger, so its stale saves are
/// refused and the user picks a side.
fn mergers() -> Vec<Arc<dyn storage_contracts::PackageMerger>> {
    vec![
        Arc::new(documents_engine::DocxMerger),
        Arc::new(spreadsheets_engine::XlsxMerger),
        Arc::new(presentations_engine::PptxMerger),
    ]
}

/// Handles the `openapi` subcommand, or loads settings and brings the database up to date.
/// `None` means there is nothing to serve.
async fn prepare() -> anyhow::Result<Option<(Settings, db::PgPool)>> {
    if std::env::args().nth(1).as_deref() == Some("openapi") {
        let json = openapi::to_json().context("serialize openapi")?;
        std::io::stdout()
            .write_all(json.as_bytes())
            .context("write openapi")?;
        return Ok(None);
    }

    load_dotenv()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let settings = Settings::from_env().context("invalid configuration")?;
    let pool = db::connect(&settings.database_url)
        .await
        .context("connect database")?;
    migrate_all(&pool).await?;
    Ok(Some((settings, pool)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some((settings, pool)) = prepare().await? else {
        return Ok(());
    };

    let workspace_export =
        std::sync::Arc::new(workspace::PgWorkspaceExportAdapter::new(pool.clone()));
    let identity = identity::module(
        pool.clone(),
        &identity::Config {
            jwt_secret: settings.jwt_secret,
            access_token_ttl_secs: settings.access_token_ttl_secs,
            refresh_token_ttl_secs: settings.refresh_token_ttl_secs,
            cookie_secure: settings.cookie_secure,
        },
        workspace_export,
    )?;
    let workspace = workspace::module(pool.clone(), identity.user_directory.clone());
    let documents = documents::module(pool.clone(), workspace.permission_checker.clone());
    let spreadsheets = spreadsheets::module(pool.clone(), workspace.permission_checker.clone());
    let presentations = presentations::module(pool.clone(), workspace.permission_checker.clone());
    let pdfs = pdfs::module(pool.clone(), workspace.permission_checker.clone());
    let storage = storage::module(
        pool.clone(),
        workspace.permission_checker.clone(),
        identity.user_profiles.clone(),
        mergers(),
    );
    let doc_bus = platform::docbus::DocBus::connect(&settings.redis_url)
        .await
        .context("connect collaboration bus")?;
    let collaboration = collaboration::module(
        pool.clone(),
        documents.access.clone(),
        collaboration::BusHandle::redis(doc_bus.clone()),
    );
    let livesync = livesync::module(
        workspace.permission_checker.clone(),
        identity.user_profiles.clone(),
        livesync::BusHandle::redis(doc_bus),
    );

    // Every instance runs the relay (a per-table advisory lock lets one work at a time) and the
    // broadcast listener; modules register their handlers here once they have any.
    let mut subscriptions = Subscriptions::new();
    for handler in workspace
        .event_handlers
        .into_iter()
        .chain(documents.event_handlers)
        .chain(spreadsheets.event_handlers)
        .chain(presentations.event_handlers)
        .chain(pdfs.event_handlers)
    {
        subscriptions.add(handler);
    }
    let events = EventRuntime::start(EventRuntimeConfig {
        pool: pool.clone(),
        redis_url: settings.redis_url,
        outbox_schemas: vec![
            "identity",
            "documents",
            "spreadsheets",
            "presentations",
            "pdfs",
            "storage",
            "workspace",
        ],
        subscriptions,
        broadcast_handlers: collaboration
            .broadcast_handlers
            .iter()
            .chain(&livesync.broadcast_handlers)
            .cloned()
            .collect(),
        poll_interval: Duration::from_millis(500),
    })
    .await
    .context("start event runtime")?;

    let listener = TcpListener::bind(&settings.api_addr)
        .await
        .with_context(|| format!("bind {}", settings.api_addr))?;
    tracing::info!(addr = %settings.api_addr, "api listening");
    let verifier: Arc<dyn BearerVerifier> = Arc::new(bearer::Adapter(identity.token_verifier));
    let routes = vec![
        ("/api/v1/auth", identity.router),
        ("/api/v1/documents", documents.router),
        ("/api/v1/spreadsheets", spreadsheets.router),
        ("/api/v1/presentations", presentations.router),
        ("/api/v1/pdfs", pdfs.router),
        ("/api/v1/packages", storage.router),
        ("/api/v1/files", workspace.router),
        ("/api/v1/collab", collaboration.router),
        ("/api/v1/livesync", livesync.router),
    ];
    let cors = cors(&settings.cors_allowed_origins)?;
    axum::serve(listener, app(verifier, routes, cors))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    events.stop().await;
    Ok(())
}
