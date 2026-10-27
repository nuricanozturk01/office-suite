//! Adapter exposing the service as the `DocumentAccess` contract port.

use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::{AccessLevel, DocumentAccess, DocumentId, OwnerId};

use crate::application::DocumentService;

#[derive(Debug)]
pub(crate) struct ServiceAccess(pub(crate) Arc<DocumentService>);

#[async_trait]
impl DocumentAccess for ServiceAccess {
    async fn access_level(&self, user: OwnerId, document: DocumentId) -> AccessLevel {
        self.0.access_level(user, document).await
    }
}
