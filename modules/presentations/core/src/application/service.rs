use std::sync::Arc;

use platform::pagination::UpdatedCursor;
use presentations_contracts::{OwnerId, PresentationId};
use serde_json::Value;
use shared_kernel::{Clock, IdGenerator};
use thiserror::Error;

use super::{
    AccessPolicy, Action, Page, PresentationRepository, PresentationSummary, RepositoryError,
};
use crate::domain::{ContentError, DeckContent, Presentation, PresentationTitle, TitleError};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum PresentationError {
    /// Input failed validation.
    #[error("validation failed")]
    Invalid,
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("presentation not found")]
    NotFound,
    /// Deck version is stale.
    #[error("version conflict")]
    Conflict,
    /// The uploaded file exceeds the size cap.
    #[error("file is too large")]
    TooLarge,
    /// The uploaded file is not a readable `.pptx` deck.
    #[error("unreadable file")]
    Unreadable,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<TitleError> for PresentationError {
    fn from(_: TitleError) -> Self {
        Self::Invalid
    }
}

impl From<ContentError> for PresentationError {
    fn from(err: ContentError) -> Self {
        match err {
            ContentError::TooLarge | ContentError::Invalid => Self::Invalid,
        }
    }
}

impl From<RepositoryError> for PresentationError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::Conflict => Self::Conflict,
            RepositoryError::Storage => Self::Internal,
        }
    }
}

/// Presentation use cases.
pub(crate) struct PresentationService {
    repo: Arc<dyn PresentationRepository>,
    policy: Arc<dyn AccessPolicy>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl std::fmt::Debug for PresentationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentationService")
            .finish_non_exhaustive()
    }
}

impl PresentationService {
    pub(crate) fn new(
        repo: Arc<dyn PresentationRepository>,
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
    ) -> Result<Presentation, PresentationError> {
        let title = PresentationTitle::new(title)?;
        let mut presentation = Presentation::create(
            PresentationId::from_uuid(self.ids.next()),
            actor,
            title,
            &self.ids.next().to_string(),
            self.clock.now(),
        );
        self.repo.insert(&mut presentation).await?;
        Ok(presentation)
    }

    /// Creates a presentation from an `.pptx` upload. The parse is CPU bound and untrusted, so
    /// it runs on the blocking pool. Returns what the importer dropped or approximated.
    pub(crate) async fn import_pptx(
        &self,
        actor: OwnerId,
        title: &str,
        bytes: Vec<u8>,
    ) -> Result<(Presentation, Vec<String>), PresentationError> {
        let title = PresentationTitle::new(title)?;
        let imported =
            tokio::task::spawn_blocking(move || presentations_engine::import_pptx(&bytes))
                .await
                .map_err(|_| PresentationError::Internal)?
                .map_err(|err| match err {
                    presentations_engine::ImportError::TooLarge => PresentationError::TooLarge,
                    presentations_engine::ImportError::Invalid(_) => PresentationError::Unreadable,
                })?;
        let content = DeckContent::parse(imported.deck)?;
        let mut presentation = Presentation::create_with_content(
            PresentationId::from_uuid(self.ids.next()),
            actor,
            title,
            content,
            self.clock.now(),
        );
        self.repo.insert(&mut presentation).await?;
        Ok((presentation, imported.warnings))
    }

    /// Renders a readable presentation as `.pptx` bytes; returns the presentation for its title.
    pub(crate) async fn export_pptx(
        &self,
        actor: OwnerId,
        id: PresentationId,
    ) -> Result<(Presentation, Vec<u8>), PresentationError> {
        let presentation = self.authorized(actor, id, Action::Read).await?;
        let deck = presentation.content_value().clone();
        let bytes = tokio::task::spawn_blocking(move || presentations_engine::export_pptx(&deck))
            .await
            .map_err(|_| PresentationError::Internal)?
            .map_err(|_| PresentationError::Internal)?;
        Ok((presentation, bytes))
    }

    async fn authorized(
        &self,
        actor: OwnerId,
        id: PresentationId,
        action: Action,
    ) -> Result<Presentation, PresentationError> {
        let presentation = self
            .repo
            .find(id)
            .await?
            .ok_or(PresentationError::NotFound)?;
        if self.policy.allows(actor, &presentation, action).await {
            Ok(presentation)
        } else {
            Err(PresentationError::NotFound)
        }
    }

    pub(crate) async fn get(
        &self,
        actor: OwnerId,
        id: PresentationId,
    ) -> Result<Presentation, PresentationError> {
        self.authorized(actor, id, Action::Read).await
    }

    pub(crate) async fn rename(
        &self,
        actor: OwnerId,
        id: PresentationId,
        title: &str,
    ) -> Result<Presentation, PresentationError> {
        let title = PresentationTitle::new(title)?;
        let mut presentation = self.authorized(actor, id, Action::Write).await?;
        presentation.rename(title, self.clock.now());
        self.repo.update_title(&mut presentation).await?;
        Ok(presentation)
    }

    pub(crate) async fn save_content(
        &self,
        actor: OwnerId,
        id: PresentationId,
        content: Value,
        expected_version: i64,
    ) -> Result<Presentation, PresentationError> {
        let content = DeckContent::parse(content)?;
        let presentation = self.authorized(actor, id, Action::Write).await?;
        if presentation.version() != expected_version {
            return Err(PresentationError::Conflict);
        }
        let now = self.clock.now();
        let version = self
            .repo
            .save_content(id, content.as_value(), expected_version, now)
            .await?;
        Ok(Presentation::restore(
            presentation.id(),
            presentation.owner_id(),
            presentation.title().clone(),
            content,
            version,
            presentation.created_at(),
            now,
        ))
    }

    pub(crate) async fn delete(
        &self,
        actor: OwnerId,
        id: PresentationId,
    ) -> Result<(), PresentationError> {
        let mut presentation = self.authorized(actor, id, Action::Delete).await?;
        presentation.delete(self.clock.now());
        self.repo.delete(&mut presentation).await?;
        Ok(())
    }

    pub(crate) async fn list(
        &self,
        actor: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: Option<u32>,
    ) -> Result<Page<PresentationSummary>, PresentationError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        Ok(self.repo.list_by_owner(actor, cursor, limit).await?)
    }
}
