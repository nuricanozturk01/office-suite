//! Shared helpers: real servers on ephemeral ports, fake ports, WebSocket clients.

#![allow(
    dead_code,
    reason = "each test binary uses a different subset of these helpers"
)]
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "panicking helpers are allowed in test code"
)]

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{Extension, Router};
use futures_util::StreamExt;
use identity_contracts::{UserId, UserProfiles};
use livesync_contracts::ServerMessage;
use platform::{broadcast::BroadcastHandler, http::BearerVerifier};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, Message, client::IntoClientRequest},
};
use uuid::Uuid;
use workspace_contracts::{PermissionChecker, ResourceId, Role};

pub(crate) type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

/// Roles that tests can change while connections are open.
#[derive(Debug, Default)]
pub(crate) struct ToggleChecker(pub(crate) std::sync::Mutex<HashMap<Uuid, Role>>);

#[async_trait::async_trait]
impl PermissionChecker for ToggleChecker {
    async fn role(&self, user: UserId, _resource: ResourceId) -> Option<Role> {
        self.0.lock().unwrap().get(&user.as_uuid()).copied()
    }
}

/// Names users after their id's first characters.
#[derive(Debug)]
pub(crate) struct Names;

#[async_trait::async_trait]
impl UserProfiles for Names {
    async fn display_name(&self, user: UserId) -> Option<String> {
        Some(format!("user-{}", &user.as_uuid().to_string()[..8]))
    }
}

pub(crate) fn name_of(user: Uuid) -> String {
    format!("user-{}", &user.to_string()[..8])
}

/// Starts a server instance on an ephemeral port; also returns the module's broadcast handlers.
pub(crate) async fn serve(
    checker: Arc<ToggleChecker>,
    bus: livesync::BusHandle,
    options: livesync::Options,
) -> (String, Vec<Arc<dyn BroadcastHandler>>) {
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    let module = livesync::module_with(checker, Arc::new(Names), bus, options);
    let app: Router = Router::new()
        .nest("/live", module.router)
        .layer(Extension(verifier));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("ws://{addr}/live"), module.broadcast_handlers)
}

pub(crate) async fn connect(
    base: &str,
    resource: Uuid,
    token: &str,
) -> Result<Ws, tungstenite::Error> {
    let mut request = format!("{base}/{resource}").into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("bearer, {token}").parse().unwrap(),
    );
    connect_async(request).await.map(|(ws, _)| ws)
}

/// The next protocol message; panics after five seconds without one.
pub(crate) async fn next_message(ws: &mut Ws) -> ServerMessage {
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for a message")
            .expect("stream ended")
            .unwrap();
        if let Message::Text(text) = frame {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

/// Whether nothing arrives within `wait`.
pub(crate) async fn is_quiet(ws: &mut Ws, wait: Duration) -> bool {
    tokio::time::timeout(wait, ws.next()).await.is_err()
}

pub(crate) async fn redis_url() -> (secrecy::SecretString, impl Sized) {
    let container = testcontainers_modules::redis::Redis::default()
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(6379).await.unwrap();
    (
        secrecy::SecretString::from(format!("redis://127.0.0.1:{port}")),
        container,
    )
}

pub(crate) async fn redis_bus(url: &secrecy::SecretString) -> livesync::BusHandle {
    livesync::BusHandle::redis(platform::docbus::DocBus::connect(url).await.unwrap())
}

use testcontainers_modules::testcontainers::runners::AsyncRunner;
