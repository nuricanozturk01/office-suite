//! Per-channel Redis bus against a real Redis.

#![expect(clippy::unwrap_used, reason = "unwrap is allowed in test code")]

use std::{sync::Arc, time::Duration};

use platform::docbus::{DocBus, RecvError};
use secrecy::SecretString;
use testcontainers_modules::{redis::Redis, testcontainers::runners::AsyncRunner};

async fn redis() -> (SecretString, redis::Client, impl Sized) {
    let container = Redis::default().start().await.unwrap();
    let port = container.get_host_port_ipv4(6379).await.unwrap();
    let url = format!("redis://127.0.0.1:{port}");
    let client = redis::Client::open(url.as_str()).unwrap();
    (SecretString::from(url), client, container)
}

async fn next(sub: &mut platform::docbus::Subscription) -> Option<Arc<[u8]>> {
    tokio::time::timeout(Duration::from_secs(3), sub.recv())
        .await
        .ok()?
        .ok()
}

#[tokio::test]
async fn message_reaches_all_local_subscribers_of_a_channel() {
    let (url, _client, _c) = redis().await;
    let bus = DocBus::connect(&url).await.unwrap();
    let mut a = bus.subscribe("doc.1").await.unwrap();
    let mut b = bus.subscribe("doc.1").await.unwrap();
    bus.publish("doc.1", b"hello".to_vec()).await.unwrap();
    assert_eq!(next(&mut a).await.as_deref(), Some(&b"hello"[..]));
    assert_eq!(next(&mut b).await.as_deref(), Some(&b"hello"[..]));
}

#[tokio::test]
async fn two_buses_exchange_messages_like_two_instances() {
    let (url, _client, _c) = redis().await;
    let (one, two) = (
        DocBus::connect(&url).await.unwrap(),
        DocBus::connect(&url).await.unwrap(),
    );
    let mut sub = two.subscribe("doc.2").await.unwrap();
    one.publish("doc.2", vec![1, 2, 3]).await.unwrap();
    assert_eq!(next(&mut sub).await.as_deref(), Some(&[1u8, 2, 3][..]));
}

#[tokio::test]
async fn other_channels_are_not_delivered() {
    let (url, _client, _c) = redis().await;
    let bus = DocBus::connect(&url).await.unwrap();
    let mut sub = bus.subscribe("doc.a").await.unwrap();
    bus.publish("doc.b", b"x".to_vec()).await.unwrap();
    assert!(next(&mut sub).await.is_none());
}

#[tokio::test]
async fn dropping_last_subscription_unsubscribes() {
    let (url, client, _c) = redis().await;
    let bus = DocBus::connect(&url).await.unwrap();
    let first = bus.subscribe("doc.3").await.unwrap();
    let second = bus.subscribe("doc.3").await.unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    let count = |mut conn: redis::aio::MultiplexedConnection| async move {
        let reply: Vec<(String, i64)> = redis::cmd("PUBSUB")
            .arg("NUMSUB")
            .arg("doc.3")
            .query_async(&mut conn)
            .await
            .unwrap();
        reply[0].1
    };
    drop(first);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(count(conn.clone()).await, 1, "still one local subscriber");
    drop(second);
    let mut gone = false;
    for _ in 0..20 {
        if count(conn.clone()).await == 0 {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(gone, "channel unsubscribed from redis after the last drop");
    let _ = &mut conn;
}

#[tokio::test]
async fn resubscribes_after_connection_loss() {
    let (url, client, _c) = redis().await;
    let bus = DocBus::connect(&url).await.unwrap();
    let mut sub = bus.subscribe("doc.4").await.unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    redis::cmd("CLIENT")
        .arg("KILL")
        .arg("TYPE")
        .arg("pubsub")
        .query_async::<i64>(&mut conn)
        .await
        .unwrap();
    // Messages during the gap are lost by design; keep publishing until the bus is back.
    let mut received = false;
    for _ in 0..50 {
        bus.publish("doc.4", b"again".to_vec()).await.unwrap();
        if let Ok(Ok(msg)) = tokio::time::timeout(Duration::from_millis(200), sub.recv()).await {
            assert_eq!(&*msg, b"again");
            received = true;
            break;
        }
    }
    assert!(
        received,
        "delivery resumed after the pubsub connection was killed"
    );
}

#[test]
fn recv_error_is_comparable() {
    assert_eq!(RecvError::Closed, RecvError::Closed);
    assert_ne!(RecvError::Lagged(1), RecvError::Closed);
}
