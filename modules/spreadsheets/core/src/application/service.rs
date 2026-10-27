use std::sync::Arc;

use platform::pagination::UpdatedCursor;
use serde_json::Value;
use shared_kernel::{Clock, IdGenerator};
use spreadsheets_contracts::{OwnerId, SpreadsheetId};
use thiserror::Error;

use super::{
    AccessPolicy, Action, Page, RepositoryError, SpreadsheetRepository, SpreadsheetSummary,
};
use crate::domain::{ContentError, Spreadsheet, SpreadsheetTitle, TitleError, WorkbookContent};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;
const MAX_SHEET_NAME: usize = 100;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum SpreadsheetError {
    /// Input failed validation.
    #[error("validation failed")]
    Invalid,
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("spreadsheet not found")]
    NotFound,
    /// Workbook version is stale.
    #[error("version conflict")]
    Conflict,
    /// The uploaded file exceeds the size cap.
    #[error("file is too large")]
    TooLarge,
    /// The uploaded file is not a readable `.xlsx` workbook.
    #[error("unreadable file")]
    Unreadable,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<TitleError> for SpreadsheetError {
    fn from(_: TitleError) -> Self {
        Self::Invalid
    }
}

impl From<ContentError> for SpreadsheetError {
    fn from(err: ContentError) -> Self {
        match err {
            ContentError::TooLarge | ContentError::Invalid => Self::Invalid,
        }
    }
}

impl From<RepositoryError> for SpreadsheetError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::Conflict => Self::Conflict,
            RepositoryError::Storage => Self::Internal,
        }
    }
}

/// Spreadsheet use cases.
pub(crate) struct SpreadsheetService {
    repo: Arc<dyn SpreadsheetRepository>,
    policy: Arc<dyn AccessPolicy>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl std::fmt::Debug for SpreadsheetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpreadsheetService").finish_non_exhaustive()
    }
}

impl SpreadsheetService {
    pub(crate) fn new(
        repo: Arc<dyn SpreadsheetRepository>,
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
        sheet_name: &str,
    ) -> Result<Spreadsheet, SpreadsheetError> {
        let title = SpreadsheetTitle::new(title)?;
        let sheet_name = normalize_sheet_name(sheet_name).ok_or(SpreadsheetError::Invalid)?;
        let mut spreadsheet = Spreadsheet::create(
            SpreadsheetId::from_uuid(self.ids.next()),
            actor,
            title,
            &self.ids.next().to_string(),
            &sheet_name,
            self.clock.now(),
        );
        self.repo.insert(&mut spreadsheet).await?;
        Ok(spreadsheet)
    }

    /// Creates a spreadsheet from an `.xlsx` upload. The parse is CPU bound and untrusted, so
    /// it runs on the blocking pool. Returns what the importer dropped or approximated.
    pub(crate) async fn import_xlsx(
        &self,
        actor: OwnerId,
        title: &str,
        bytes: Vec<u8>,
    ) -> Result<(Spreadsheet, Vec<String>), SpreadsheetError> {
        let title = SpreadsheetTitle::new(title)?;
        let imported =
            tokio::task::spawn_blocking(move || spreadsheets_engine::import_xlsx(&bytes))
                .await
                .map_err(|_| SpreadsheetError::Internal)?
                .map_err(|err| match err {
                    spreadsheets_engine::ImportError::TooLarge => SpreadsheetError::TooLarge,
                    spreadsheets_engine::ImportError::Invalid(_) => SpreadsheetError::Unreadable,
                })?;
        let content = WorkbookContent::parse(imported.workbook)?;
        let mut spreadsheet = Spreadsheet::create_with_content(
            SpreadsheetId::from_uuid(self.ids.next()),
            actor,
            title,
            content,
            self.clock.now(),
        );
        self.repo.insert(&mut spreadsheet).await?;
        Ok((spreadsheet, imported.warnings))
    }

    /// Renders a readable spreadsheet as `.xlsx` bytes; returns the spreadsheet for its title.
    pub(crate) async fn export_xlsx(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
    ) -> Result<(Spreadsheet, Vec<u8>), SpreadsheetError> {
        let spreadsheet = self.authorized(actor, id, Action::Read).await?;
        let workbook = spreadsheet.content_value().clone();
        let bytes =
            tokio::task::spawn_blocking(move || spreadsheets_engine::export_xlsx(&workbook))
                .await
                .map_err(|_| SpreadsheetError::Internal)?
                .map_err(|_| SpreadsheetError::Internal)?;
        Ok((spreadsheet, bytes))
    }

    async fn authorized(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
        action: Action,
    ) -> Result<Spreadsheet, SpreadsheetError> {
        let spreadsheet = self
            .repo
            .find(id)
            .await?
            .ok_or(SpreadsheetError::NotFound)?;
        if self.policy.allows(actor, &spreadsheet, action).await {
            Ok(spreadsheet)
        } else {
            Err(SpreadsheetError::NotFound)
        }
    }

    pub(crate) async fn get(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
    ) -> Result<Spreadsheet, SpreadsheetError> {
        self.authorized(actor, id, Action::Read).await
    }

    pub(crate) async fn rename(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
        title: &str,
    ) -> Result<Spreadsheet, SpreadsheetError> {
        let title = SpreadsheetTitle::new(title)?;
        let mut spreadsheet = self.authorized(actor, id, Action::Write).await?;
        spreadsheet.rename(title, self.clock.now());
        self.repo.update_title(&mut spreadsheet).await?;
        Ok(spreadsheet)
    }

    pub(crate) async fn save_content(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
        content: Value,
        expected_version: i64,
    ) -> Result<Spreadsheet, SpreadsheetError> {
        let content = WorkbookContent::parse(content)?;
        let spreadsheet = self.authorized(actor, id, Action::Write).await?;
        if spreadsheet.version() != expected_version {
            return Err(SpreadsheetError::Conflict);
        }
        let now = self.clock.now();
        let version = self
            .repo
            .save_content(id, content.as_value(), expected_version, now)
            .await?;
        Ok(Spreadsheet::restore(
            spreadsheet.id(),
            spreadsheet.owner_id(),
            spreadsheet.title().clone(),
            content,
            version,
            spreadsheet.created_at(),
            now,
        ))
    }

    pub(crate) async fn delete(
        &self,
        actor: OwnerId,
        id: SpreadsheetId,
    ) -> Result<(), SpreadsheetError> {
        let mut spreadsheet = self.authorized(actor, id, Action::Delete).await?;
        spreadsheet.delete(self.clock.now());
        self.repo.delete(&mut spreadsheet).await?;
        Ok(())
    }

    pub(crate) async fn list(
        &self,
        actor: OwnerId,
        cursor: Option<UpdatedCursor>,
        limit: Option<u32>,
    ) -> Result<Page<SpreadsheetSummary>, SpreadsheetError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        Ok(self.repo.list_by_owner(actor, cursor, limit).await?)
    }
}

fn normalize_sheet_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_SHEET_NAME {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
