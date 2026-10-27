//! End-to-end WebSocket tests: real server, real Postgres, real Yjs clients.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "panicking helpers are allowed in test code"
)]

use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{Extension, Router};
use collaboration::domain::protocol::{self, Incoming};
use documents_contracts::{AccessLevel, DocumentAccess, DocumentId, OwnerId};
use futures_util::{SinkExt, StreamExt};
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

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct UuidTokens;

impl BearerVerifier for UuidTokens {
    fn verify(&self, token: &str) -> Option<Uuid> {
        token.parse().ok()
    }
}

#[derive(Debug)]
struct FakeAccess(HashMap<Uuid, AccessLevel>);

#[async_trait::async_trait]
impl DocumentAccess for FakeAccess {
    async fn access_level(&self, user: OwnerId, _document: DocumentId) -> AccessLevel {
        self.0
            .get(&user.as_uuid())
            .copied()
            .unwrap_or(AccessLevel::None)
    }
}

/// Starts a server instance (fresh in-memory rooms) on an ephemeral port.
async fn serve(pool: PgPool, access: HashMap<Uuid, AccessLevel>) -> String {
    let verifier: Arc<dyn BearerVerifier> = Arc::new(UuidTokens);
    let module = collaboration::module(
        pool,
        Arc::new(FakeAccess(access)),
        collaboration::BusHandle::none(),
    );
    let app: Router = Router::new()
        .nest("/collab", module.router)
        .layer(Extension(verifier));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("ws://{addr}/collab")
}

async fn connect(base: &str, document: Uuid, token: &str) -> Result<Ws, tungstenite::Error> {
    let mut request = format!("{base}/{document}").into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        format!("bearer, {token}").parse().unwrap(),
    );
    connect_async(request).await.map(|(ws, _)| ws)
}

async fn next_binary(ws: &mut Ws) -> Vec<u8> {
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
async fn pull_update(ws: &mut Ws, doc: &Doc) {
    loop {
        if let Incoming::Update(bytes) = protocol::parse(&next_binary(ws).await).unwrap() {
            doc.transact_mut()
                .apply_update(Update::decode_v1(&bytes).unwrap())
                .unwrap();
            return;
        }
    }
}

fn type_text(doc: &Doc, text: &str) -> Vec<u8> {
    let before = doc.transact().state_vector();
    doc.get_or_insert_text("t")
        .insert(&mut doc.transact_mut(), 0, text);
    let diff = doc.transact().encode_diff_v1(&before);
    protocol::update(diff)
}

fn content(doc: &Doc) -> String {
    doc.get_or_insert_text("t").get_string(&doc.transact())
}

async fn database() -> (PgPool, impl Sized) {
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

#[tokio::test]
async fn peers_converge_and_state_survives_restart() {
    let (pool, _db) = database().await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let access = HashMap::from([(alice, AccessLevel::Write), (bob, AccessLevel::Write)]);
    let document = Uuid::now_v7();
    let base = serve(pool.clone(), access.clone()).await;

    let mut a = connect(&base, document, &alice.to_string()).await.unwrap();
    let mut b = connect(&base, document, &bob.to_string()).await.unwrap();
    let (doc_a, doc_b) = (Doc::new(), Doc::new());
    next_binary(&mut a).await; // server hello
    next_binary(&mut b).await;

    a.send(Message::Binary(type_text(&doc_a, "hello ").into()))
        .await
        .unwrap();
    pull_update(&mut b, &doc_b).await;
    assert_eq!(content(&doc_b), "hello ");

    b.send(Message::Binary(type_text(&doc_b, "world").into()))
        .await
        .unwrap();
    pull_update(&mut a, &doc_a).await;
    assert_eq!(content(&doc_a), content(&doc_b));

    a.close(None).await.unwrap();
    b.close(None).await.unwrap();

    // A brand-new server instance has empty rooms and must rebuild from Postgres.
    let restarted = serve(pool, access).await;
    let mut c = connect(&restarted, document, &alice.to_string())
        .await
        .unwrap();
    next_binary(&mut c).await; // server hello
    let doc_c = Doc::new();
    c.send(Message::Binary(
        protocol::step1(StateVector::default()).into(),
    ))
    .await
    .unwrap();
    pull_update(&mut c, &doc_c).await;
    assert_eq!(content(&doc_c), content(&doc_a));
    assert!(content(&doc_c).contains("hello"));
}

#[tokio::test]
async fn read_only_users_cannot_write_and_strangers_cannot_join() {
    let (pool, _db) = database().await;
    let (writer, reader, stranger) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    let access = HashMap::from([(writer, AccessLevel::Write), (reader, AccessLevel::Read)]);
    let document = Uuid::now_v7();
    let base = serve(pool, access).await;

    // Bad token -> 401, no access -> 404 (both rejected during the handshake).
    for (token, expected) in [("not-a-uuid".to_owned(), 401), (stranger.to_string(), 404)] {
        match connect(&base, document, &token).await {
            Err(tungstenite::Error::Http(res)) => assert_eq!(res.status().as_u16(), expected),
            other => panic!("expected HTTP {expected}, got {:?}", other.map(|_| ())),
        }
    }

    let mut r = connect(&base, document, &reader.to_string()).await.unwrap();
    let mut w = connect(&base, document, &writer.to_string()).await.unwrap();
    next_binary(&mut r).await;
    next_binary(&mut w).await;

    // The reader's write attempt ends the session with a policy violation...
    let doc = Doc::new();
    r.send(Message::Binary(type_text(&doc, "nope").into()))
        .await
        .unwrap();
    let close = loop {
        match tokio::time::timeout(Duration::from_secs(5), r.next())
            .await
            .unwrap()
        {
            Some(Ok(Message::Close(frame))) => break frame.unwrap(),
            Some(Ok(_)) => {}
            other => panic!("expected close frame, got {other:?}"),
        }
    };
    assert_eq!(u16::from(close.code), 1008);

    // ...and nothing reached the writer.
    let quiet = tokio::time::timeout(Duration::from_millis(300), w.next()).await;
    assert!(
        quiet.is_err(),
        "writer must not receive the rejected update"
    );

    // The reader can still read: a fresh connection gets an empty document.
    let mut r2 = connect(&base, document, &reader.to_string()).await.unwrap();
    next_binary(&mut r2).await;
    r2.send(Message::Binary(
        protocol::step1(StateVector::default()).into(),
    ))
    .await
    .unwrap();
    let reply = protocol::parse(&next_binary(&mut r2).await).unwrap();
    assert!(matches!(reply, Incoming::Update(_)));
}

#[tokio::test]
async fn compaction_keeps_content_intact() {
    let (pool, _db) = database().await;
    let user = Uuid::now_v7();
    let access = HashMap::from([(user, AccessLevel::Write)]);
    let document = Uuid::now_v7();
    let base = serve(pool.clone(), access.clone()).await;

    let mut ws = connect(&base, document, &user.to_string()).await.unwrap();
    next_binary(&mut ws).await;
    let doc = Doc::new();
    for _ in 0..205 {
        ws.send(Message::Binary(type_text(&doc, "x").into()))
            .await
            .unwrap();
    }
    // Syncing is ordered per connection: once the step-1 reply arrives, every update is applied.
    ws.send(Message::Binary(
        protocol::step1(StateVector::default()).into(),
    ))
    .await
    .unwrap();
    next_binary(&mut ws).await;
    ws.close(None).await.unwrap();

    // Compaction runs in the background; wait for it.
    let mut snapshots: i64 = 0;
    for _ in 0..50 {
        snapshots = sqlx::query_scalar("SELECT count(*) FROM collaboration.document_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
        if snapshots > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM collaboration.document_updates")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(snapshots, 1);
    assert!(
        remaining < 205,
        "compaction must drop folded updates, {remaining} remain"
    );

    let restarted = serve(pool, access).await;
    let mut ws = connect(&restarted, document, &user.to_string())
        .await
        .unwrap();
    next_binary(&mut ws).await;
    let restored = Doc::new();
    ws.send(Message::Binary(
        protocol::step1(StateVector::default()).into(),
    ))
    .await
    .unwrap();
    pull_update(&mut ws, &restored).await;
    assert_eq!(content(&restored), "x".repeat(205));
}
