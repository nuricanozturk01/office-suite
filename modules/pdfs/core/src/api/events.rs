//! Event consumers: keeps "last edited" current when the file behind a PDF is saved.

use std::sync::Arc;

use async_trait::async_trait;
use platform::{
    event_bus::{EventEnvelope, EventHandler, HandlerError},
    touch::{TouchTable, touch_updated_at},
};
use sqlx::PgPool;
use storage_contracts::PackageSaved;

const CONSUMER: &str = "pdfs.touch.PackageSaved";

/// Moves a PDF to the top of "last edited" lists after its file is saved.
struct TouchOnPackageSaved {
    pool: PgPool,
    table: TouchTable,
}

#[async_trait]
impl EventHandler for TouchOnPackageSaved {
    fn consumer(&self) -> &'static str {
        CONSUMER
    }

    fn event_type(&self) -> &'static str {
        "PackageSaved"
    }

    async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError> {
        let saved: PackageSaved = serde_json::from_value(event.payload.clone())
            .map_err(|_| HandlerError::Permanent("undecodable event payload".to_owned()))?;
        touch_updated_at(
            &self.pool,
            &self.table,
            CONSUMER,
            event.id,
            saved.resource_id.as_uuid(),
            saved.at,
        )
        .await
        .map_err(|e| HandlerError::Transient(e.to_string()))
    }
}

/// Durable handlers this module registers with the outbox relay.
pub(crate) fn handlers(pool: PgPool) -> Vec<Arc<dyn EventHandler>> {
    // INVARIANT: both identifiers are literals that satisfy `TouchTable::new`.
    #[expect(clippy::expect_used, reason = "constant identifiers are valid")]
    let table = TouchTable::new("pdfs", "pdfs").expect("valid table identifier");
    vec![Arc::new(TouchOnPackageSaved { pool, table })]
}
