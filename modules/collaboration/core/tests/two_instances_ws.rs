//! Two real server instances over one Postgres and one Redis behave like one.

mod common;

use std::{collections::HashMap, sync::Arc, time::Duration};

use common::{
    FakeAccess, connect, content, database, next_binary, pull_update, read_document, redis_bus,
    redis_url, serve_with, type_text,
};
use documents_contracts::AccessLevel;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use yrs::Doc;

#[tokio::test]
async fn editors_on_different_instances_converge_and_late_joiners_get_the_union() {
    let (pool, _db) = database().await;
    let (url, _redis) = redis_url().await;
    let (alice, bob) = (Uuid::now_v7(), Uuid::now_v7());
    let access = Arc::new(FakeAccess(HashMap::from([
        (alice, AccessLevel::Write),
        (bob, AccessLevel::Write),
    ])));
    let document = Uuid::now_v7();
    let options = collaboration::Options::default();
    let a = serve_with(pool.clone(), access.clone(), redis_bus(&url).await, options).await;
    let b = serve_with(pool.clone(), access.clone(), redis_bus(&url).await, options).await;

    let mut on_a = connect(&a, document, &alice.to_string()).await.unwrap();
    let mut on_b = connect(&b, document, &bob.to_string()).await.unwrap();
    next_binary(&mut on_a).await;
    next_binary(&mut on_b).await;
    let (doc_a, doc_b) = (Doc::new(), Doc::new());

    on_a.send(Message::Binary(type_text(&doc_a, "alice ").into()))
        .await
        .unwrap();
    pull_update(&mut on_b, &doc_b).await;
    on_b.send(Message::Binary(type_text(&doc_b, "bob ").into()))
        .await
        .unwrap();
    pull_update(&mut on_a, &doc_a).await;
    assert_eq!(content(&doc_a), content(&doc_b), "live editors converge");

    // A late joiner on either instance sees both edits.
    for base in [&a, &b] {
        let late = read_document(base, document, &alice.to_string()).await;
        let text = content(&late);
        assert!(text.contains("alice") && text.contains("bob"), "{text}");
    }

    // The durable log holds the union too (a restart would not lose anything).
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM collaboration.document_updates WHERE document_id = $1",
    )
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, 2);
    let fresh = serve_with(
        pool,
        access,
        collaboration::BusHandle::none(),
        collaboration::Options {
            catch_up_interval: Duration::from_mins(10),
            ..options
        },
    )
    .await;
    let restored = read_document(&fresh, document, &bob.to_string()).await;
    assert_eq!(content(&restored), content(&doc_a));
}
