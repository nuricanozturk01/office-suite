use std::sync::Arc;

use async_trait::async_trait;
use identity_contracts::UserId;
use presentations_contracts::OwnerId;
use workspace_contracts::{PermissionChecker, ResourceId};

use super::{AccessPolicy, Action};
use crate::domain::Presentation;

/// The owner may do anything; other users get what their shared role allows. Deleting stays
/// owner-only. Editors may rename (as in Google Drive).
pub(crate) struct OwnerOrShared {
    checker: Arc<dyn PermissionChecker>,
}

impl OwnerOrShared {
    pub(crate) fn new(checker: Arc<dyn PermissionChecker>) -> Self {
        Self { checker }
    }
}

#[async_trait]
impl AccessPolicy for OwnerOrShared {
    async fn allows(&self, actor: OwnerId, presentation: &Presentation, action: Action) -> bool {
        if presentation.owner_id() == actor {
            return true;
        }
        if action == Action::Delete {
            return false;
        }
        let role = self
            .checker
            .role(
                UserId::from_uuid(actor.as_uuid()),
                ResourceId::from_uuid(presentation.id().as_uuid()),
            )
            .await;
        role.is_some_and(|role| match action {
            Action::Read => role.can_read(),
            Action::Write => role.can_write(),
            Action::Delete => false,
        })
    }
}
