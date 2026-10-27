//! Broadcast consumers: react on every instance to events from other modules.

use std::sync::Arc;

use async_trait::async_trait;
use platform::{broadcast::BroadcastHandler, event_bus::EventEnvelope};
use storage_contracts::PackageSaved;
use workspace_contracts::PermissionRevoked;

use crate::application::LiveSyncService;

/// Tells the open connections that somebody saved a new version.
struct FileSaved {
    service: Arc<LiveSyncService>,
}

#[async_trait]
impl BroadcastHandler for FileSaved {
    fn event_type(&self) -> &'static str {
        "PackageSaved"
    }

    async fn handle(&self, event: &EventEnvelope) {
        let Ok(saved) = serde_json::from_value::<PackageSaved>(event.payload.clone()) else {
            tracing::warn!(event_id = %event.id, "undecodable PackageSaved payload");
            return;
        };
        self.service
            .saved(saved.resource_id, saved.version, saved.by, saved.at)
            .await;
    }
}

/// Closes the connections of a user whose access shrank or ended.
struct AccessRevoked {
    service: Arc<LiveSyncService>,
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
            .access_changed(revoked.resource_id, revoked.user_id)
            .await;
    }
}

pub(crate) fn broadcast_handlers(service: &Arc<LiveSyncService>) -> Vec<Arc<dyn BroadcastHandler>> {
    vec![
        Arc::new(FileSaved {
            service: service.clone(),
        }),
        Arc::new(AccessRevoked {
            service: service.clone(),
        }),
    ]
}
