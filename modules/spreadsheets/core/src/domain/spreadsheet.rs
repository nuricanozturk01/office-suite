use serde_json::Value;
use spreadsheets_contracts::{
    OwnerId, SpreadsheetCreated, SpreadsheetDeleted, SpreadsheetId, SpreadsheetRenamed,
};
use time::OffsetDateTime;

use super::{SpreadsheetTitle, WorkbookContent};

/// Domain events collected by [`Spreadsheet`]; the repository writes them to the outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpreadsheetEvent {
    /// The spreadsheet was created.
    Created(SpreadsheetCreated),
    /// The spreadsheet was renamed.
    Renamed(SpreadsheetRenamed),
    /// The spreadsheet was deleted.
    Deleted(SpreadsheetDeleted),
}

/// Spreadsheet aggregate root. `version` locks content writes; renames leave it unchanged.
#[derive(Debug)]
pub(crate) struct Spreadsheet {
    id: SpreadsheetId,
    owner_id: OwnerId,
    title: SpreadsheetTitle,
    content: WorkbookContent,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    events: Vec<SpreadsheetEvent>,
}

impl Spreadsheet {
    /// Creates a spreadsheet with one empty sheet and records `SpreadsheetCreated`.
    #[must_use]
    pub(crate) fn create(
        id: SpreadsheetId,
        owner_id: OwnerId,
        title: SpreadsheetTitle,
        sheet_id: &str,
        sheet_name: &str,
        now: OffsetDateTime,
    ) -> Self {
        let event = SpreadsheetCreated {
            spreadsheet_id: id,
            owner_id,
            title: title.as_str().to_owned(),
            created_at: now,
        };
        Self {
            id,
            owner_id,
            title,
            content: WorkbookContent::empty(sheet_id, sheet_name),
            version: 1,
            created_at: now,
            updated_at: now,
            events: vec![SpreadsheetEvent::Created(event)],
        }
    }

    /// Creates a spreadsheet around an already validated workbook (an imported file).
    #[must_use]
    pub(crate) fn create_with_content(
        id: SpreadsheetId,
        owner_id: OwnerId,
        title: SpreadsheetTitle,
        content: WorkbookContent,
        now: OffsetDateTime,
    ) -> Self {
        let event = SpreadsheetCreated {
            spreadsheet_id: id,
            owner_id,
            title: title.as_str().to_owned(),
            created_at: now,
        };
        Self {
            id,
            owner_id,
            title,
            content,
            version: 1,
            created_at: now,
            updated_at: now,
            events: vec![SpreadsheetEvent::Created(event)],
        }
    }

    /// Rebuilds a persisted spreadsheet (no events).
    #[must_use]
    pub(crate) const fn restore(
        id: SpreadsheetId,
        owner_id: OwnerId,
        title: SpreadsheetTitle,
        content: WorkbookContent,
        version: i64,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            owner_id,
            title,
            content,
            version,
            created_at,
            updated_at,
            events: Vec::new(),
        }
    }

    /// Changes the title without touching the content version.
    pub(crate) fn rename(&mut self, title: SpreadsheetTitle, now: OffsetDateTime) {
        self.events
            .push(SpreadsheetEvent::Renamed(SpreadsheetRenamed {
                spreadsheet_id: self.id,
                title: title.as_str().to_owned(),
                at: now,
            }));
        self.title = title;
        self.updated_at = now;
    }

    /// Marks the spreadsheet deleted (the repository performs the soft delete).
    pub(crate) fn delete(&mut self, now: OffsetDateTime) {
        self.events
            .push(SpreadsheetEvent::Deleted(SpreadsheetDeleted {
                spreadsheet_id: self.id,
                at: now,
            }));
        self.updated_at = now;
    }

    /// Spreadsheet id.
    #[must_use]
    pub(crate) const fn id(&self) -> SpreadsheetId {
        self.id
    }

    /// Owner id.
    #[must_use]
    pub(crate) const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    /// Current title.
    #[must_use]
    pub(crate) const fn title(&self) -> &SpreadsheetTitle {
        &self.title
    }

    /// Content version.
    #[must_use]
    pub(crate) const fn version(&self) -> i64 {
        self.version
    }

    /// Creation time (UTC).
    #[must_use]
    pub(crate) const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Last modification time (UTC).
    #[must_use]
    pub(crate) const fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// Drains collected domain events (to be written to the outbox).
    pub(crate) fn take_events(&mut self) -> Vec<SpreadsheetEvent> {
        std::mem::take(&mut self.events)
    }

    /// JSON borrowed from the workbook, for persistence.
    #[must_use]
    pub(crate) const fn content_value(&self) -> &Value {
        self.content.as_value()
    }
}

#[cfg(test)]
mod tests {
    use shared_kernel::Uuid;

    use super::*;

    fn sheet() -> Spreadsheet {
        Spreadsheet::create(
            SpreadsheetId::from_uuid(Uuid::nil()),
            OwnerId::from_uuid(Uuid::nil()),
            SpreadsheetTitle::new("Budget").unwrap(),
            "sheet-1",
            "Sayfa1",
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn rename_collects_renamed_event_and_keeps_version() {
        let mut sheet = sheet();
        sheet.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
        sheet.rename(SpreadsheetTitle::new("Plan").unwrap(), later);
        assert_eq!(
            sheet.take_events(),
            [SpreadsheetEvent::Renamed(SpreadsheetRenamed {
                spreadsheet_id: sheet.id(),
                title: "Plan".to_owned(),
                at: later,
            })]
        );
        assert_eq!(sheet.version(), 1);
    }

    #[test]
    fn delete_collects_deleted_event() {
        let mut sheet = sheet();
        sheet.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(2);
        sheet.delete(later);
        assert_eq!(
            sheet.take_events(),
            [SpreadsheetEvent::Deleted(SpreadsheetDeleted {
                spreadsheet_id: sheet.id(),
                at: later
            })]
        );
        assert_eq!(sheet.updated_at(), later);
    }

    #[test]
    fn create_records_one_event_and_a_sheet() {
        let mut sheet = Spreadsheet::create(
            SpreadsheetId::from_uuid(Uuid::nil()),
            OwnerId::from_uuid(Uuid::nil()),
            SpreadsheetTitle::new("Budget").unwrap(),
            "sheet-1",
            "Sayfa1",
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(sheet.version(), 1);
        assert_eq!(sheet.content_value()["sheets"][0]["name"], "Sayfa1");
        assert_eq!(sheet.take_events().len(), 1);
        assert!(sheet.take_events().is_empty());
    }
}
