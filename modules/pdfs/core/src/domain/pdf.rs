use pdfs_contracts::{OwnerId, PdfCreated, PdfDeleted, PdfId, PdfRenamed};
use time::OffsetDateTime;

use super::PdfTitle;

/// Domain events collected by [`Pdf`]; the application layer writes them to the outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfEvent {
    /// The pdf was created.
    Created(PdfCreated),
    /// The pdf was renamed.
    Renamed(PdfRenamed),
    /// The pdf was deleted.
    Deleted(PdfDeleted),
}

/// Pdf aggregate root.
#[derive(Debug)]
pub struct Pdf {
    id: PdfId,
    owner_id: OwnerId,
    title: PdfTitle,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    events: Vec<PdfEvent>,
}

impl Pdf {
    /// Creates a new pdf and records a `PdfCreated` event.
    #[must_use]
    pub fn create(id: PdfId, owner_id: OwnerId, title: PdfTitle, now: OffsetDateTime) -> Self {
        let event = PdfCreated {
            pdf_id: id,
            owner_id,
            title: title.as_str().to_owned(),
            created_at: now,
        };
        Self {
            id,
            owner_id,
            title,
            created_at: now,
            updated_at: now,
            events: vec![PdfEvent::Created(event)],
        }
    }

    /// Rebuilds a persisted pdf (no events).
    #[must_use]
    pub const fn restore(
        id: PdfId,
        owner_id: OwnerId,
        title: PdfTitle,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            owner_id,
            title,
            created_at,
            updated_at,
            events: Vec::new(),
        }
    }

    /// Changes the title.
    pub fn rename(&mut self, title: PdfTitle, now: OffsetDateTime) {
        self.events.push(PdfEvent::Renamed(PdfRenamed {
            pdf_id: self.id,
            title: title.as_str().to_owned(),
            at: now,
        }));
        self.title = title;
        self.updated_at = now;
    }

    /// Marks the pdf deleted (the repository performs the soft delete).
    pub fn delete(&mut self, now: OffsetDateTime) {
        self.events.push(PdfEvent::Deleted(PdfDeleted {
            pdf_id: self.id,
            at: now,
        }));
        self.updated_at = now;
    }

    /// Pdf id.
    #[must_use]
    pub const fn id(&self) -> PdfId {
        self.id
    }

    /// Owner id.
    #[must_use]
    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    /// Current title.
    #[must_use]
    pub const fn title(&self) -> &PdfTitle {
        &self.title
    }

    /// Creation time (UTC).
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Last modification time (UTC).
    #[must_use]
    pub const fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// Drains collected domain events (to be written to the outbox).
    pub fn take_events(&mut self) -> Vec<PdfEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use shared_kernel::Uuid;

    use super::*;

    fn doc() -> Pdf {
        Pdf::create(
            PdfId::from_uuid(Uuid::nil()),
            OwnerId::from_uuid(Uuid::nil()),
            PdfTitle::new("Notes").unwrap(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn create_records_event_once() {
        let mut doc = doc();
        let events = doc.take_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], PdfEvent::Created(c) if c.title == "Notes"));
        assert!(doc.take_events().is_empty());
    }

    #[test]
    fn rename_collects_renamed_event() {
        let mut doc = doc();
        doc.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
        doc.rename(PdfTitle::new("Plan").unwrap(), later);
        assert_eq!(
            doc.take_events(),
            [PdfEvent::Renamed(PdfRenamed {
                pdf_id: doc.id(),
                title: "Plan".to_owned(),
                at: later,
            })]
        );
    }

    #[test]
    fn delete_collects_deleted_event() {
        let mut doc = doc();
        doc.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(2);
        doc.delete(later);
        assert_eq!(
            doc.take_events(),
            [PdfEvent::Deleted(PdfDeleted {
                pdf_id: doc.id(),
                at: later
            })]
        );
        assert_eq!(doc.updated_at(), later);
    }

    #[test]
    fn rename_updates_title_and_timestamp() {
        let mut doc = doc();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
        doc.rename(PdfTitle::new("Plan").unwrap(), later);
        assert_eq!(doc.title().as_str(), "Plan");
        assert_eq!(doc.updated_at(), later);
        assert_eq!(doc.created_at(), OffsetDateTime::UNIX_EPOCH);
    }
}
