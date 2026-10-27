//! Presence, save pushes and access changes over real `WebSockets` (single instance).

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "unwrap and panic are allowed in test code"
)]

mod common;

use std::{sync::Arc, time::Duration};

use common::{ToggleChecker, connect, is_quiet, name_of, next_message, serve};
use identity_contracts::UserId;
use livesync_contracts::ServerMessage;
use platform::event_bus::EventEnvelope;
use storage_contracts::PackageSaved;
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use workspace_contracts::{PermissionRevoked, ResourceId, Role};

use futures_util::{SinkExt, StreamExt};

fn checker(users: &[(Uuid, Role)]) -> Arc<ToggleChecker> {
    let checker = ToggleChecker::default();
    checker.0.lock().unwrap().extend(users.iter().copied());
    Arc::new(checker)
}

fn envelope(event_type: &str, payload: serde_json::Value) -> EventEnvelope {
    EventEnvelope {
        id: Uuid::now_v7(),
        event_type: event_type.to_owned(),
        payload,
    }
}

#[tokio::test]
async fn people_see_each_other_come_and_go() {
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let (base, _) = serve(
        checker(&[(alice, Role::Editor), (bob, Role::Viewer)]),
        livesync::BusHandle::none(),
        livesync::Options::default(),
    )
    .await;
    let resource = Uuid::now_v7();

    let mut on_alice = connect(&base, resource, &alice.to_string()).await.unwrap();
    let ServerMessage::Welcome { participants, .. } = next_message(&mut on_alice).await else {
        panic!("first message is a welcome");
    };
    assert!(participants.is_empty(), "alone in the room");

    let mut on_bob = connect(&base, resource, &bob.to_string()).await.unwrap();
    let ServerMessage::Welcome {
        session: bob_session,
        participants,
    } = next_message(&mut on_bob).await
    else {
        panic!("first message is a welcome");
    };
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].user_id, UserId::from_uuid(alice));
    assert_eq!(participants[0].display_name, name_of(alice));
    assert_eq!(participants[0].role, Role::Editor);
    assert!(!participants[0].editing);

    let ServerMessage::Joined { participant } = next_message(&mut on_alice).await else {
        panic!("alice hears about bob");
    };
    assert_eq!(participant.session, bob_session);
    assert_eq!(participant.display_name, name_of(bob));
    assert_eq!(participant.role, Role::Viewer);

    on_bob.close(None).await.unwrap();
    assert_eq!(
        next_message(&mut on_alice).await,
        ServerMessage::Left {
            session: bob_session
        }
    );
}

#[tokio::test]
async fn a_save_reaches_everybody_who_has_the_file_open() {
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let (base, handlers) = serve(
        checker(&[(alice, Role::Editor), (bob, Role::Viewer)]),
        livesync::BusHandle::none(),
        livesync::Options::default(),
    )
    .await;
    let (resource, elsewhere) = (Uuid::now_v7(), Uuid::now_v7());
    let mut on_alice = connect(&base, resource, &alice.to_string()).await.unwrap();
    let mut on_bob = connect(&base, resource, &bob.to_string()).await.unwrap();
    let mut on_other = connect(&base, elsewhere, &alice.to_string()).await.unwrap();
    next_message(&mut on_alice).await; // welcome
    next_message(&mut on_alice).await; // bob joined
    next_message(&mut on_bob).await; // welcome
    next_message(&mut on_other).await; // welcome

    let handler = handlers
        .iter()
        .find(|h| h.event_type() == "PackageSaved")
        .unwrap();
    let saved = PackageSaved {
        resource_id: ResourceId::from_uuid(resource),
        version: 7,
        by: Some(UserId::from_uuid(alice)),
        at: OffsetDateTime::now_utc(),
    };
    handler
        .handle(&envelope(
            "PackageSaved",
            serde_json::to_value(&saved).unwrap(),
        ))
        .await;

    for ws in [&mut on_alice, &mut on_bob] {
        let ServerMessage::Saved { version, by, .. } = next_message(ws).await else {
            panic!("a save is pushed");
        };
        assert_eq!(version, 7);
        assert_eq!(by, Some(UserId::from_uuid(alice)));
    }
    assert!(
        is_quiet(&mut on_other, Duration::from_millis(400)).await,
        "another file's viewers hear nothing"
    );
}

#[tokio::test]
async fn people_without_access_cannot_connect() {
    let (alice, stranger) = (Uuid::now_v7(), Uuid::now_v7());
    let (base, _) = serve(
        checker(&[(alice, Role::Viewer)]),
        livesync::BusHandle::none(),
        livesync::Options::default(),
    )
    .await;
    assert!(
        connect(&base, Uuid::now_v7(), &stranger.to_string())
            .await
            .is_err()
    );
    assert!(connect(&base, Uuid::now_v7(), "not-a-token").await.is_err());
}

#[tokio::test]
async fn revoked_access_closes_the_connection() {
    let alice = Uuid::now_v7();
    let roles = checker(&[(alice, Role::Editor)]);
    let (base, handlers) = serve(
        roles.clone(),
        livesync::BusHandle::none(),
        livesync::Options::default(),
    )
    .await;
    let resource = Uuid::now_v7();
    let mut ws = connect(&base, resource, &alice.to_string()).await.unwrap();
    next_message(&mut ws).await; // welcome

    roles.0.lock().unwrap().remove(&alice);
    let handler = handlers
        .iter()
        .find(|h| h.event_type() == "PermissionRevoked")
        .unwrap();
    let revoked = PermissionRevoked {
        resource_id: ResourceId::from_uuid(resource),
        user_id: UserId::from_uuid(alice),
        at: OffsetDateTime::now_utc(),
    };
    handler
        .handle(&envelope(
            "PermissionRevoked",
            serde_json::to_value(&revoked).unwrap(),
        ))
        .await;

    let frame = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match frame {
        Message::Close(Some(close)) => assert_eq!(u16::from(close.code), 1008),
        other => panic!("expected a close frame, got {other:?}"),
    }
}

#[tokio::test]
async fn editors_announce_that_they_are_editing_and_viewers_cannot() {
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let (base, _) = serve(
        checker(&[(alice, Role::Editor), (bob, Role::Viewer)]),
        livesync::BusHandle::none(),
        livesync::Options::default(),
    )
    .await;
    let resource = Uuid::now_v7();
    let mut on_alice = connect(&base, resource, &alice.to_string()).await.unwrap();
    let ServerMessage::Welcome {
        session: alice_session,
        ..
    } = next_message(&mut on_alice).await
    else {
        panic!("first message is a welcome");
    };
    let mut on_bob = connect(&base, resource, &bob.to_string()).await.unwrap();
    next_message(&mut on_bob).await; // welcome
    next_message(&mut on_alice).await; // bob joined

    let activity =
        |editing| Message::Text(format!(r#"{{"type":"activity","editing":{editing}}}"#).into());
    on_alice.send(activity(true)).await.unwrap();
    assert_eq!(
        next_message(&mut on_bob).await,
        ServerMessage::Activity {
            session: alice_session,
            editing: true
        }
    );

    on_bob.send(activity(true)).await.unwrap();
    assert!(
        is_quiet(&mut on_alice, Duration::from_millis(400)).await,
        "a viewer's activity is not announced"
    );
}
