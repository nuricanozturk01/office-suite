//! Broadcast consumers: react on every instance to events from other modules.

use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::{DocumentId, OwnerId};
use platform::{broadcast::BroadcastHandler, event_bus::EventEnvelope};
use workspace_contracts::PermissionRevoked;

use crate::application::CollaborationService;

/// Closes open sessions of a user whose access to a document shrank or ended.
struct AccessRevoked {
    service: Arc<CollaborationService>,
}

#[async_trait]
impl BroadcastHandler for AccessRevoked {
    fn event_type(&self) -> &'static str {
        "PermissionRevoked"
    }

    async fn handle(&self, event: &EventEnvelope) {
        let Ok(revoked) = serde_json::from_value::<PermissionRevoked>(event.payload.clone()) else {
            tracing::warn!(event_id = %event.id, "undecodable PermissionRevoked payload");
            return;
        };
        self.service
            .access_changed(
                DocumentId::from_uuid(revoked.resource_id.as_uuid()),
                OwnerId::from_uuid(revoked.user_id.as_uuid()),
            )
            .await;
    }
}

pub(crate) fn broadcast_handlers(
    service: &Arc<CollaborationService>,
) -> Vec<Arc<dyn BroadcastHandler>> {
    vec![Arc::new(AccessRevoked {
        service: service.clone(),
    })]
}
