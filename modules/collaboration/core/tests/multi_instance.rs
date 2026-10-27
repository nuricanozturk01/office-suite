//! Several server instances over one database must never lose or diverge edits.

mod common;

use std::{collections::HashMap, sync::Arc, time::Duration};

use common::{
    FakeAccess, awareness_frame, connect, content, count_frames, database, next_binary,
    pull_update, read_document, redis_bus, redis_url, serve, serve_with, type_text, wait_awareness,
};
use documents_contracts::AccessLevel;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use yrs::Doc;

fn writer_access(user: Uuid) -> Arc<FakeAccess> {
    Arc::new(FakeAccess(HashMap::from([(user, AccessLevel::Write)])))
}

fn options(catch_up: Duration) -> collaboration::Options {
    collaboration::Options {
        catch_up_interval: catch_up,
        ..collaboration::Options::default()
    }
}

#[tokio::test]
async fn compaction_by_one_instance_keeps_updates_of_the_other() {
    let (pool, _db) = database().await;
    let user = Uuid::now_v7();
    let access: Arc<FakeAccess> = Arc::new(FakeAccess(HashMap::from([(user, AccessLevel::Write)])));
    let document = Uuid::now_v7();
    // Two instances: separate in-memory rooms over the same log.
    let (a, b) = (
        serve(pool.clone(), access.clone()).await,
        serve(pool.clone(), access.clone()).await,
    );

    let mut on_b = connect(&b, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_b).await;
    let doc_b = Doc::new();
    for n in 0..10 {
        on_b.send(Message::Binary(type_text(&doc_b, &format!("b{n} ")).into()))
            .await
            .unwrap();
    }

    // Instance A takes enough edits to compact; its snapshot must still contain B's edits.
    let mut on_a = connect(&a, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    let doc_a = Doc::new();
    for n in 0..200 {
        on_a.send(Message::Binary(type_text(&doc_a, &format!("a{n} ")).into()))
            .await
            .unwrap();
    }
    for _ in 0..50 {
        let snapshots: i64 =
            sqlx::query_scalar("SELECT count(*) FROM collaboration.document_snapshots")
                .fetch_one(&pool)
                .await
                .unwrap();
        if snapshots > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let fresh = serve(pool, access).await;
    let doc = read_document(&fresh, document, &user.to_string()).await;
    let text = content(&doc);
    let mut tokens: Vec<_> = text.split_whitespace().collect();
    tokens.sort_unstable();
    let mut expected: Vec<String> = (0..10).map(|n| format!("b{n}")).collect();
    expected.extend((0..200).map(|n| format!("a{n}")));
    expected.sort_unstable();
    assert_eq!(
        tokens, expected,
        "every edit of both instances survives compaction"
    );
}

#[tokio::test]
async fn update_on_a_reaches_session_on_b_via_redis() {
    let (pool, _db) = database().await;
    let (url, _redis) = redis_url().await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let access = Arc::new(FakeAccess(HashMap::from([
        (alice, AccessLevel::Write),
        (bob, AccessLevel::Write),
    ])));
    let document = Uuid::now_v7();
    // A very long catch-up interval proves the update travelled over Redis.
    let slow = options(Duration::from_mins(10));
    let a = serve_with(pool.clone(), access.clone(), redis_bus(&url).await, slow).await;
    let b = serve_with(pool, access, redis_bus(&url).await, slow).await;

    let mut on_a = connect(&a, document, &alice.to_string()).await.unwrap();
    let mut on_b = connect(&b, document, &bob.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    next_binary(&mut on_b).await;
    let (doc_a, doc_b) = (Doc::new(), Doc::new());
    on_a.send(Message::Binary(type_text(&doc_a, "from a").into()))
        .await
        .unwrap();
    pull_update(&mut on_b, &doc_b).await;
    assert_eq!(content(&doc_b), "from a");
}

#[tokio::test]
async fn awareness_on_a_reaches_b() {
    let (pool, _db) = database().await;
    let (url, _redis) = redis_url().await;
    let user = Uuid::now_v7();
    let access = writer_access(user);
    let document = Uuid::now_v7();
    let slow = options(Duration::from_mins(10));
    let a = serve_with(pool.clone(), access.clone(), redis_bus(&url).await, slow).await;
    let b = serve_with(pool, access, redis_bus(&url).await, slow).await;

    let mut on_a = connect(&a, document, &user.to_string()).await.unwrap();
    let mut on_b = connect(&b, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    next_binary(&mut on_b).await;
    on_a.send(Message::Binary(awareness_frame().into()))
        .await
        .unwrap();
    wait_awareness(&mut on_b).await;
}

#[tokio::test]
async fn catch_up_delivers_update_when_redis_messages_are_lost() {
    let (pool, _db) = database().await;
    let user = Uuid::now_v7();
    let access = writer_access(user);
    let document = Uuid::now_v7();
    // No bus at all: only the database catch-up can connect the two instances.
    let fast = options(Duration::from_millis(100));
    let a = serve_with(
        pool.clone(),
        access.clone(),
        collaboration::BusHandle::none(),
        fast,
    )
    .await;
    let b = serve_with(pool, access, collaboration::BusHandle::none(), fast).await;

    let mut on_a = connect(&a, document, &user.to_string()).await.unwrap();
    let mut on_b = connect(&b, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    next_binary(&mut on_b).await;
    let (doc_a, doc_b) = (Doc::new(), Doc::new());
    on_a.send(Message::Binary(type_text(&doc_a, "healed").into()))
        .await
        .unwrap();
    pull_update(&mut on_b, &doc_b).await;
    assert_eq!(content(&doc_b), "healed");
}

#[tokio::test]
async fn own_echo_and_duplicate_paths_deliver_each_update_once() {
    let (pool, _db) = database().await;
    let (url, _redis) = redis_url().await;
    let user = Uuid::now_v7();
    let access = writer_access(user);
    let document = Uuid::now_v7();
    // Redis AND fast catch-up both deliver to B; A also sees its own bus echo.
    let fast = options(Duration::from_millis(100));
    let a = serve_with(pool.clone(), access.clone(), redis_bus(&url).await, fast).await;
    let b = serve_with(pool, access, redis_bus(&url).await, fast).await;

    let mut a1 = connect(&a, document, &user.to_string()).await.unwrap();
    let mut a2 = connect(&a, document, &user.to_string()).await.unwrap();
    let mut on_b = connect(&b, document, &user.to_string()).await.unwrap();
    for ws in [&mut a1, &mut a2, &mut on_b] {
        next_binary(ws).await;
    }
    let doc = Doc::new();
    a1.send(Message::Binary(type_text(&doc, "once").into()))
        .await
        .unwrap();
    let window = Duration::from_millis(800);
    assert_eq!(
        count_frames(&mut a2, window).await.0,
        1,
        "peer on the same instance"
    );
    assert_eq!(
        count_frames(&mut on_b, window).await.0,
        1,
        "peer on the other instance"
    );
}

#[tokio::test]
async fn catch_up_applies_the_snapshot_when_log_rows_were_compacted_away() {
    let (pool, _db) = database().await;
    let user = Uuid::now_v7();
    let access = writer_access(user);
    let document = Uuid::now_v7();
    // C follows the log only (no bus) and polls late, so A's compaction happens first.
    let slow = options(Duration::from_secs(3));
    let c = serve_with(
        pool.clone(),
        access.clone(),
        collaboration::BusHandle::none(),
        slow,
    )
    .await;
    let a = serve(pool.clone(), access).await;

    let mut on_c = connect(&c, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_c).await;
    let mut on_a = connect(&a, document, &user.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    let doc_a = Doc::new();
    for n in 0..200 {
        on_a.send(Message::Binary(type_text(&doc_a, &format!("a{n} ")).into()))
            .await
            .unwrap();
    }
    for _ in 0..50 {
        let snapshots: i64 =
            sqlx::query_scalar("SELECT count(*) FROM collaboration.document_snapshots")
                .fetch_one(&pool)
                .await
                .unwrap();
        if snapshots > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // C's room must converge although the rows it missed no longer exist in the log.
    let doc_c = Doc::new();
    let mut tokens = 0;
    while tokens < 200 {
        pull_update(&mut on_c, &doc_c).await;
        tokens = content(&doc_c).split_whitespace().count();
    }
    assert_eq!(tokens, 200);
}
