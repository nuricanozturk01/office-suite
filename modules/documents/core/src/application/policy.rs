use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::OwnerId;
use identity_contracts::UserId;
use workspace_contracts::{PermissionChecker, ResourceId};

use super::{AccessPolicy, Action};
use crate::domain::Document;

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
    async fn allows(&self, actor: OwnerId, doc: &Document, action: Action) -> bool {
        if doc.owner_id() == actor {
            return true;
        }
        if action == Action::Delete {
            return false;
        }
        let role = self
            .checker
            .role(
                UserId::from_uuid(actor.as_uuid()),
                ResourceId::from_uuid(doc.id().as_uuid()),
            )
            .await;
        role.is_some_and(|role| match action {
            Action::Read => role.can_read(),
            Action::Write => role.can_write(),
            Action::Delete => false,
        })
    }
}
