//! Transport-neutral event types shared by the outbox relay and broadcast (CLAUDE.md §4.2.5).

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// A serialized integration event as stored in a module's outbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique event id (dedupe key).
    pub id: Uuid,
    /// Event type name, e.g. `DocumentCreated`.
    pub event_type: String,
    /// Event body as defined by the publishing module's contracts.
    pub payload: serde_json::Value,
}

/// Failure of an [`EventHandler`]; the relay keeps the event and retries on both variants.
#[derive(Debug, Error)]
pub enum HandlerError {
    /// A retry may succeed (database or network hiccup).
    #[error("transient handler failure: {0}")]
    Transient(String),
    /// A retry will not help; logged loudly (dead-lettering is not implemented yet).
    #[error("permanent handler failure: {0}")]
    Permanent(String),
}

/// A durable consumer: runs once per event cluster-wide.
///
/// Implementations must be idempotent and call [`crate::outbox::claim`] in the same transaction
/// as their effect.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Stable consumer name, the dedupe namespace.
    fn consumer(&self) -> &'static str;
    /// Event type this handler subscribes to.
    fn event_type(&self) -> &'static str;
    /// Applies the event.
    ///
    /// # Errors
    /// Returns [`HandlerError`] when the event could not be applied.
    async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError>;
}

/// Registry of durable handlers by event type.
#[derive(Default)]
pub struct Subscriptions {
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl std::fmt::Debug for Subscriptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriptions")
            .field("handlers", &self.handlers.len())
            .finish()
    }
}

impl Subscriptions {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler.
    pub fn add(&mut self, handler: Arc<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// Handlers subscribed to `event_type`.
    #[must_use]
    pub fn for_type(&self, event_type: &str) -> Vec<Arc<dyn EventHandler>> {
        self.handlers
            .iter()
            .filter(|h| h.event_type() == event_type)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct Named(&'static str, &'static str);

    #[async_trait]
    impl EventHandler for Named {
        fn consumer(&self) -> &'static str {
            self.0
        }
        fn event_type(&self) -> &'static str {
            self.1
        }
        async fn handle(&self, _event: &EventEnvelope) -> Result<(), HandlerError> {
            Ok(())
        }
    }

    #[test]
    fn for_type_returns_only_matching_handlers() {
        let mut subs = Subscriptions::new();
        subs.add(Arc::new(Named("a", "Created")));
        subs.add(Arc::new(Named("b", "Created")));
        subs.add(Arc::new(Named("c", "Deleted")));
        let created: Vec<_> = subs
            .for_type("Created")
            .iter()
            .map(|h| h.consumer())
            .collect();
        assert_eq!(created, ["a", "b"]);
        assert_eq!(subs.for_type("Deleted").len(), 1);
        assert!(subs.for_type("Unknown").is_empty());
    }

    #[test]
    fn envelope_json_roundtrip() {
        let env = EventEnvelope {
            id: Uuid::nil(),
            event_type: "Created".to_owned(),
            payload: json!({"k": 1}),
        };
        let back: EventEnvelope =
            serde_json::from_str(&serde_json::to_string(&env).unwrap()).unwrap();
        assert_eq!(back, env);
    }
}
