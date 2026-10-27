//! Two server instances over one Redis show the same room.

#![expect(clippy::panic, reason = "panic is allowed in test code")]

mod common;

use std::{sync::Arc, time::Duration};

use common::{ToggleChecker, connect, name_of, next_message, redis_bus, redis_url, serve};
use livesync_contracts::ServerMessage;
use uuid::Uuid;
use workspace_contracts::Role;

#[tokio::test]
async fn presence_crosses_instances_both_ways() {
    let (url, _redis) = redis_url().await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let checker = ToggleChecker::default();
    checker
        .0
        .lock()
        .unwrap()
        .extend([(alice, Role::Editor), (bob, Role::Editor)]);
    let checker = Arc::new(checker);
    let options = livesync::Options::default();
    let (a, _) = serve(checker.clone(), redis_bus(&url).await, options).await;
    let (b, _) = serve(checker, redis_bus(&url).await, options).await;
    let resource = Uuid::now_v7();

    let mut on_a = connect(&a, resource, &alice.to_string()).await.unwrap();
    next_message(&mut on_a).await; // welcome
    // Give the room's bus subscription a moment before the second instance joins.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut on_b = connect(&b, resource, &bob.to_string()).await.unwrap();
    next_message(&mut on_b).await; // welcome (may be empty: the other instance has not spoken yet)

    let ServerMessage::Joined { participant } = next_message(&mut on_a).await else {
        panic!("alice hears about bob across instances");
    };
    assert_eq!(participant.display_name, name_of(bob));

    let ServerMessage::Joined { participant } = next_message(&mut on_b).await else {
        panic!("bob learns about alice from her instance");
    };
    assert_eq!(participant.display_name, name_of(alice));
}
