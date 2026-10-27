//! Shared helpers for WebSocket integration tests: real servers, real Postgres, Yjs clients.

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
use collaboration::domain::protocol::{self, Incoming};
use documents_contracts::{AccessLevel, DocumentAccess, DocumentId, OwnerId};
use futures_util::StreamExt;
use platform::http::BearerVerifier;
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, Message, client::IntoClientRequest},
};
use uuid::Uuid;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update, updates::decoder::Decode};

pub(crate) type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

#[derive(Debug)]
pub(crate) struct FakeAccess(pub(crate) HashMap<Uuid, AccessLevel>);

#[async_trait::async_trait]
impl DocumentAccess for FakeAccess {
    async fn access_level(&self, user: OwnerId, _document: DocumentId) -> AccessLevel {
        self.0
            .get(&user.as_uuid())
            .copied()
            .unwrap_or(AccessLevel::None)
    }
}

/// Starts a single-instance server (no cross-instance bus) on an ephemeral port.
pub(crate) async fn serve(pool: PgPool, access: Arc<dyn DocumentAccess>) -> String {
    serve_with(
        pool,
        access,
        collaboration::BusHandle::none(),
        collaboration::Options::default(),
    )
    .await
}

/// Starts a server instance (fresh in-memory rooms) on an ephemeral port.
pub(crate) async fn serve_with(
    pool: PgPool,
    access: Arc<dyn DocumentAccess>,
    bus: collaboration::BusHandle,
    options: collaboration::Options,
) -> String {
    serve_module(pool, access, bus, options).await.0
}

/// Like [`serve_with`], also returning the module's broadcast handlers.
pub(crate) async fn serve_module(
    pool: PgPool,
    access: Arc<dyn DocumentAccess>,
    bus: collaboration::BusHandle,
    options: collaboration::Options,
) -> (String, Vec<Arc<dyn platform::broadcast::BroadcastHandler>>) {
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    let module = collaboration::module_with(pool, access, bus, options);
    let app: Router = Router::new()
        .nest("/collab", module.router)
        .layer(Extension(verifier));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("ws://{addr}/collab"), module.broadcast_handlers)
}

/// Access levels that tests can change while sessions are open.
#[derive(Debug, Default)]
pub(crate) struct ToggleAccess(pub(crate) std::sync::Mutex<HashMap<Uuid, AccessLevel>>);

#[async_trait::async_trait]
impl DocumentAccess for ToggleAccess {
    async fn access_level(&self, user: OwnerId, _document: DocumentId) -> AccessLevel {
        self.0
            .lock()
            .unwrap()
            .get(&user.as_uuid())
            .copied()
            .unwrap_or(AccessLevel::None)
    }
}

pub(crate) async fn connect(
    base: &str,
    document: Uuid,
    token: &str,
) -> Result<Ws, tungstenite::Error> {
    let mut request = format!("{base}/{document}").into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("bearer, {token}").parse().unwrap(),
    );
    connect_async(request).await.map(|(ws, _)| ws)
}

pub(crate) async fn next_binary(ws: &mut Ws) -> Vec<u8> {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for a frame")
            .expect("stream ended")
            .unwrap();
        if let Message::Binary(bytes) = msg {
            return bytes.to_vec();
        }
    }
}

/// Reads frames until an update arrives and applies it to `doc`.
pub(crate) async fn pull_update(ws: &mut Ws, doc: &Doc) {
    loop {
        if let Incoming::Update(bytes) = protocol::parse(&next_binary(ws).await).unwrap() {
            doc.transact_mut()
                .apply_update(Update::decode_v1(&bytes).unwrap())
                .unwrap();
            return;
        }
    }
}

pub(crate) fn type_text(doc: &Doc, text: &str) -> Vec<u8> {
    let before = doc.transact().state_vector();
    doc.get_or_insert_text("t")
        .insert(&mut doc.transact_mut(), 0, text);
    let diff = doc.transact().encode_diff_v1(&before);
    protocol::update(diff)
}

pub(crate) fn content(doc: &Doc) -> String {
    doc.get_or_insert_text("t").get_string(&doc.transact())
}

/// Joins a fresh connection and returns the document as the server currently knows it.
pub(crate) async fn read_document(base: &str, document: Uuid, token: &str) -> Doc {
    let mut ws = connect(base, document, token).await.unwrap();
    next_binary(&mut ws).await; // server hello
    let doc = Doc::new();
    futures_util::SinkExt::send(
        &mut ws,
        Message::Binary(protocol::step1(StateVector::default()).into()),
    )
    .await
    .unwrap();
    pull_update(&mut ws, &doc).await;
    doc
}

pub(crate) async fn database() -> (PgPool, impl Sized) {
    let container = Postgres::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = platform::db::connect(&secrecy::SecretString::from(url))
        .await
        .unwrap();
    platform::db::migrate(&pool, &collaboration::migrator())
        .await
        .unwrap();
    (pool, container)
}

/// Starts a Redis container and returns its URL.
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

/// A bus handle connected to `url` (each call is one "instance").
pub(crate) async fn redis_bus(url: &secrecy::SecretString) -> collaboration::BusHandle {
    collaboration::BusHandle::redis(platform::docbus::DocBus::connect(url).await.unwrap())
}

/// A valid y-protocols awareness frame.
pub(crate) fn awareness_frame() -> Vec<u8> {
    use yrs::{
        sync::{
            Message,
            awareness::{AwarenessUpdate, AwarenessUpdateEntry},
        },
        updates::encoder::Encode,
    };
    let entry = AwarenessUpdateEntry {
        clock: 1,
        json: "{\"name\":\"a\"}".into(),
    };
    Message::Awareness(AwarenessUpdate {
        clients: HashMap::from([(yrs::block::ClientID::new(7), entry)]),
    })
    .encode_v1()
}

/// Counts `(updates, awareness)` frames received within `window`.
pub(crate) async fn count_frames(ws: &mut Ws, window: Duration) -> (usize, usize) {
    let (mut updates, mut awareness) = (0, 0);
    let deadline = tokio::time::Instant::now() + window;
    while let Ok(Some(Ok(msg))) = tokio::time::timeout_at(deadline, ws.next()).await {
        if let Message::Binary(bytes) = msg {
            match protocol::parse(&bytes) {
                Ok(Incoming::Update(_)) => updates += 1,
                Ok(Incoming::Awareness) => awareness += 1,
                _ => {}
            }
        }
    }
    (updates, awareness)
}

/// Reads frames until an awareness frame arrives.
pub(crate) async fn wait_awareness(ws: &mut Ws) {
    loop {
        if matches!(
            protocol::parse(&next_binary(ws).await),
            Ok(Incoming::Awareness)
        ) {
            return;
        }
    }
}
