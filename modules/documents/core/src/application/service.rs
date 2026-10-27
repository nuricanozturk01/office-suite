use std::sync::Arc;

use documents_contracts::{AccessLevel, DocumentId, OwnerId};
use platform::pagination::UpdatedCursor;
use shared_kernel::{Clock, IdGenerator};
use thiserror::Error;

use super::{AccessPolicy, Action, DocumentRepository, Page, RepositoryError};
use crate::domain::{Document, DocumentTitle, TitleError};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum DocumentError {
    /// Input failed validation.
    #[error(transparent)]
    Invalid(#[from] TitleError),
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("document not found")]
    NotFound,
    /// The uploaded file exceeds the size cap.
    #[error("file is too large")]
    TooLarge,
    /// The uploaded file is not a readable `.docx` document.
    #[error("unreadable file")]
    Unreadable,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<RepositoryError> for DocumentError {
    fn from(_: RepositoryError) -> Self {
        Self::Internal
    }
}

/// Document use cases.
pub(crate) struct DocumentService {
    repo: Arc<dyn DocumentRepository>,
    policy: Arc<dyn AccessPolicy>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl std::fmt::Debug for DocumentService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentService").finish_non_exhaustive()
    }
}

impl DocumentService {
    pub(crate) fn new(
        repo: Arc<dyn DocumentRepository>,
        policy: Arc<dyn AccessPolicy>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            repo,
            policy,
            clock,
            ids,
        }
    }

    pub(crate) async fn create(
        &self,
        actor: OwnerId,
        title: &str,
    ) -> Result<Document, DocumentError> {
        let title = DocumentTitle::new(title)?;
        let mut doc = Document::create(
            DocumentId::from_uuid(self.ids.next()),
            actor,
            title,
            self.clock.now(),
        );
        self.repo.insert(&mut doc).await?;
        Ok(doc)
    }

    /// Loads a document the actor may access; denial is reported as `NotFound`.
    async fn authorized(
        &self,
        actor: OwnerId,
        id: DocumentId,
        action: Action,
    ) -> Result<Document, DocumentError> {
        let doc = self.repo.find(id).await?.ok_or(DocumentError::NotFound)?;
        if self.policy.allows(actor, &doc, action).await {
            Ok(doc)
        } else {
            Err(DocumentError::NotFound)
        }
    }

    /// Highest access level the actor holds on a document.
    pub(crate) async fn access_level(&self, actor: OwnerId, id: DocumentId) -> AccessLevel {
        let Ok(Some(doc)) = self.repo.find(id).await else {
            return AccessLevel::None;
        };
        if self.policy.allows(actor, &doc, Action::Write).await {
            AccessLevel::Write
        } else if self.policy.allows(actor, &doc, Action::Read).await {
            AccessLevel::Read
        } else {
            AccessLevel::None
        }
    }

    pub(crate) async fn get(
        &self,
        actor: OwnerId,
        id: DocumentId,
    ) -> Result<Document, DocumentError> {
        self.authorized(actor, id, Action::Read).await
    }

    pub(crate) async fn rename(
        &self,
        actor: OwnerId,
        id: DocumentId,
        title: &str,
    ) -> Result<Document, DocumentError> {
        let title = DocumentTitle::new(title)?;
        let mut doc = self.authorized(actor, id, Action::Write).await?;
        doc.rename(title, self.clock.now());
        self.repo.update(&mut doc).await?;
        Ok(doc)
    }

    pub(crate) async fn delete(&self, actor: OwnerId, id: DocumentId) -> Result<(), DocumentError> {
        let mut doc = self.authorized(actor, id, Action::Delete).await?;
        doc.delete(self.clock.now());
        self.repo.delete(&mut doc).await?;
        Ok(())
    }

    pub(crate) async fn list(
        &self,
        actor: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: Option<u32>,
    ) -> Result<Page<Document>, DocumentError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        Ok(self.repo.list_by_owner(actor, cursor, limit).await?)
    }
}
