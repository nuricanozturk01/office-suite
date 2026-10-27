//! Redis broadcast against a real Redis: every listener (instance) receives each event.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use platform::{
    broadcast::{BroadcastHandler, Listener, RedisBroadcaster},
    event_bus::EventEnvelope,
    outbox::Broadcaster,
};
use secrecy::SecretString;
use testcontainers_modules::{redis::Redis, testcontainers::runners::AsyncRunner};
use tokio::sync::mpsc;
use uuid::Uuid;

struct Forward(&'static str, mpsc::UnboundedSender<EventEnvelope>);

#[async_trait]
impl BroadcastHandler for Forward {
    fn event_type(&self) -> &'static str {
        self.0
    }
    async fn handle(&self, event: &EventEnvelope) {
        self.1.send(event.clone()).unwrap();
    }
}

fn event(kind: &str) -> EventEnvelope {
    EventEnvelope {
        id: Uuid::now_v7(),
        event_type: kind.to_owned(),
        payload: serde_json::json!({"k": 1}),
    }
}

async fn redis_url() -> (SecretString, impl Sized) {
    let container = Redis::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(6379).await.unwrap();
    (
        SecretString::from(format!("redis://127.0.0.1:{port}")),
        container,
    )
}

async fn instance(
    url: &SecretString,
    kind: &'static str,
) -> (
    mpsc::UnboundedReceiver<EventEnvelope>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let handlers: Vec<Arc<dyn BroadcastHandler>> = vec![Arc::new(Forward(kind, tx))];
    let listener = Listener::connect(url, handlers).await.unwrap();
    (rx, tokio::spawn(listener.run(std::future::pending())))
}

async fn next(rx: &mut mpsc::UnboundedReceiver<EventEnvelope>) -> Option<EventEnvelope> {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .ok()
        .flatten()
}

#[tokio::test]
async fn broadcast_reaches_every_listener() {
    let (url, _c) = redis_url().await;
    let (mut a, _ja) = instance(&url, "PermissionRevoked").await;
    let (mut b, _jb) = instance(&url, "PermissionRevoked").await;
    let sent = event("PermissionRevoked");
    RedisBroadcaster::connect(&url)
        .await
        .unwrap()
        .publish(&sent)
        .await
        .unwrap();
    assert_eq!(next(&mut a).await, Some(sent.clone()));
    assert_eq!(next(&mut b).await, Some(sent));
}

#[tokio::test]
async fn unrelated_event_type_is_ignored() {
    let (url, _c) = redis_url().await;
    let (mut a, _ja) = instance(&url, "PermissionRevoked").await;
    let bus = RedisBroadcaster::connect(&url).await.unwrap();
    bus.publish(&event("SomethingElse")).await.unwrap();
    assert_eq!(next(&mut a).await, None);
}

#[tokio::test]
async fn malformed_message_does_not_stop_listener() {
    let (url, _c) = redis_url().await;
    let (mut a, _ja) = instance(&url, "PermissionRevoked").await;
    let client = redis::Client::open(secrecy::ExposeSecret::expose_secret(&url)).unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    redis::cmd("PUBLISH")
        .arg("events.PermissionRevoked")
        .arg("not json")
        .query_async::<()>(&mut conn)
        .await
        .unwrap();
    let sent = event("PermissionRevoked");
    RedisBroadcaster::connect(&url)
        .await
        .unwrap()
        .publish(&sent)
        .await
        .unwrap();
    assert_eq!(next(&mut a).await, Some(sent));
}
