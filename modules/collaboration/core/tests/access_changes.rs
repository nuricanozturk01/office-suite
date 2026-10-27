//! Open sessions must not outlive the access they were granted.

mod common;

use std::{sync::Arc, time::Duration};

use common::{ToggleAccess, connect, database, next_binary, serve_module, type_text};
use documents_contracts::AccessLevel;
use futures_util::{SinkExt, StreamExt};
use platform::event_bus::EventEnvelope;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use yrs::Doc;

fn revoked(document: Uuid, user: Uuid) -> EventEnvelope {
    EventEnvelope {
        id: Uuid::now_v7(),
        event_type: "PermissionRevoked".to_owned(),
        payload: serde_json::json!({
            "resource_id": document,
            "user_id": user,
            "at": "2026-01-01T00:00:00Z",
        }),
    }
}

/// Waits for a close frame and returns its code.
async fn close_code(ws: &mut common::Ws) -> Option<u16> {
    loop {
        match tokio::time::timeout(Duration::from_secs(3), ws.next()).await {
            Ok(Some(Ok(Message::Close(Some(frame))))) => return Some(u16::from(frame.code)),
            Ok(Some(Ok(_))) => {}
            _ => return None,
        }
    }
}

fn options(recheck: Duration) -> collaboration::Options {
    collaboration::Options {
        recheck_interval: recheck,
        ..collaboration::Options::default()
    }
}

#[tokio::test]
async fn revoke_event_closes_only_the_affected_users_session() {
    let (pool, _db) = database().await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let access = Arc::new(ToggleAccess::default());
    access
        .0
        .lock()
        .unwrap()
        .extend([(alice, AccessLevel::Write), (bob, AccessLevel::Write)]);
    let document = Uuid::now_v7();
    // A long recheck interval proves the close came from the event, not the timer.
    let (base, handlers) = serve_module(
        pool,
        access,
        collaboration::BusHandle::none(),
        options(Duration::from_mins(10)),
    )
    .await;
    let handler = &handlers[0];

    let mut a = connect(&base, document, &alice.to_string()).await.unwrap();
    let mut b = connect(&base, document, &bob.to_string()).await.unwrap();
    next_binary(&mut a).await;
    next_binary(&mut b).await;

    handler.handle(&revoked(document, alice)).await;
    assert_eq!(close_code(&mut a).await, Some(1008));

    // Bob keeps his session and can still edit.
    let quiet = tokio::time::timeout(Duration::from_millis(300), b.next()).await;
    assert!(quiet.is_err(), "bob's session stays open and silent");
    let doc = Doc::new();
    b.send(Message::Binary(type_text(&doc, "still here").into()))
        .await
        .unwrap();
}

#[tokio::test]
async fn recheck_closes_session_after_downgrade_even_without_event() {
    let (pool, _db) = database().await;
    let alice = Uuid::now_v7();
    let access = Arc::new(ToggleAccess::default());
    access.0.lock().unwrap().insert(alice, AccessLevel::Write);
    let document = Uuid::now_v7();
    let (base, _handlers) = serve_module(
        pool,
        access.clone(),
        collaboration::BusHandle::none(),
        options(Duration::from_millis(100)),
    )
    .await;

    let mut ws = connect(&base, document, &alice.to_string()).await.unwrap();
    next_binary(&mut ws).await;
    access.0.lock().unwrap().insert(alice, AccessLevel::Read);
    assert_eq!(close_code(&mut ws).await, Some(1008));
}

#[tokio::test]
async fn revoke_for_a_room_that_is_not_loaded_is_a_noop() {
    let (pool, _db) = database().await;
    let access = Arc::new(ToggleAccess::default());
    let (_base, handlers) = serve_module(
        pool,
        access,
        collaboration::BusHandle::none(),
        collaboration::Options::default(),
    )
    .await;
    handlers[0]
        .handle(&revoked(Uuid::now_v7(), Uuid::now_v7()))
        .await;
    // Garbage payloads are ignored too.
    let junk = EventEnvelope {
        id: Uuid::now_v7(),
        event_type: "PermissionRevoked".to_owned(),
        payload: serde_json::json!({"nope": 1}),
    };
    handlers[0].handle(&junk).await;
}
