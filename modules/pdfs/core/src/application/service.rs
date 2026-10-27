use std::sync::Arc;

use pdfs_contracts::{OwnerId, PdfId};
use platform::pagination::UpdatedCursor;
use shared_kernel::{Clock, IdGenerator};
use thiserror::Error;

use super::{AccessPolicy, Action, Page, PdfRepository, RepositoryError};
use crate::domain::{Pdf, PdfTitle, TitleError};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum PdfError {
    /// Input failed validation.
    #[error(transparent)]
    Invalid(#[from] TitleError),
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("pdf not found")]
    NotFound,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<RepositoryError> for PdfError {
    fn from(_: RepositoryError) -> Self {
        Self::Internal
    }
}

/// Pdf use cases.
pub(crate) struct PdfService {
    repo: Arc<dyn PdfRepository>,
    policy: Arc<dyn AccessPolicy>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl std::fmt::Debug for PdfService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfService").finish_non_exhaustive()
    }
}

impl PdfService {
    pub(crate) fn new(
        repo: Arc<dyn PdfRepository>,
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

    pub(crate) async fn create(&self, actor: OwnerId, title: &str) -> Result<Pdf, PdfError> {
        let title = PdfTitle::new(title)?;
        let mut doc = Pdf::create(
            PdfId::from_uuid(self.ids.next()),
            actor,
            title,
            self.clock.now(),
        );
        self.repo.insert(&mut doc).await?;
        Ok(doc)
    }

    /// Loads a pdf the actor may access; denial is reported as `NotFound`.
    async fn authorized(&self, actor: OwnerId, id: PdfId, action: Action) -> Result<Pdf, PdfError> {
        let doc = self.repo.find(id).await?.ok_or(PdfError::NotFound)?;
        if self.policy.allows(actor, &doc, action).await {
            Ok(doc)
        } else {
            Err(PdfError::NotFound)
        }
    }

    pub(crate) async fn get(&self, actor: OwnerId, id: PdfId) -> Result<Pdf, PdfError> {
        self.authorized(actor, id, Action::Read).await
    }

    pub(crate) async fn rename(
        &self,
        actor: OwnerId,
        id: PdfId,
        title: &str,
    ) -> Result<Pdf, PdfError> {
        let title = PdfTitle::new(title)?;
        let mut doc = self.authorized(actor, id, Action::Write).await?;
        doc.rename(title, self.clock.now());
        self.repo.update(&mut doc).await?;
        Ok(doc)
    }

    pub(crate) async fn delete(&self, actor: OwnerId, id: PdfId) -> Result<(), PdfError> {
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
    ) -> Result<Page<Pdf>, PdfError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        Ok(self.repo.list_by_owner(actor, cursor, limit).await?)
    }
}
