use presentations_contracts::{
    OwnerId, PresentationCreated, PresentationDeleted, PresentationId, PresentationRenamed,
};
use serde_json::Value;
use time::OffsetDateTime;

use super::{DeckContent, PresentationTitle};

/// Domain events collected by [`Presentation`]; the repository writes them to the outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PresentationEvent {
    /// The presentation was created.
    Created(PresentationCreated),
    /// The presentation was renamed.
    Renamed(PresentationRenamed),
    /// The presentation was deleted.
    Deleted(PresentationDeleted),
}

/// Presentation aggregate root. `version` locks content writes; renames leave it unchanged.
#[derive(Debug)]
pub(crate) struct Presentation {
    id: PresentationId,
    owner_id: OwnerId,
    title: PresentationTitle,
    content: DeckContent,
    version: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    events: Vec<PresentationEvent>,
}

impl Presentation {
    /// Creates a presentation with one blank slide and records `PresentationCreated`.
    #[must_use]
    pub(crate) fn create(
        id: PresentationId,
        owner_id: OwnerId,
        title: PresentationTitle,
        slide_id: &str,
        now: OffsetDateTime,
    ) -> Self {
        let event = PresentationCreated {
            presentation_id: id,
            owner_id,
            title: title.as_str().to_owned(),
            created_at: now,
        };
        Self {
            id,
            owner_id,
            title,
            content: DeckContent::empty(slide_id),
            version: 1,
            created_at: now,
            updated_at: now,
            events: vec![PresentationEvent::Created(event)],
        }
    }

    /// Creates a presentation around an already validated deck (an imported file).
    #[must_use]
    pub(crate) fn create_with_content(
        id: PresentationId,
        owner_id: OwnerId,
        title: PresentationTitle,
        content: DeckContent,
        now: OffsetDateTime,
    ) -> Self {
        let event = PresentationCreated {
            presentation_id: id,
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
            events: vec![PresentationEvent::Created(event)],
        }
    }

    /// Rebuilds a persisted presentation (no events).
    #[must_use]
    pub(crate) const fn restore(
        id: PresentationId,
        owner_id: OwnerId,
        title: PresentationTitle,
        content: DeckContent,
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
    pub(crate) fn rename(&mut self, title: PresentationTitle, now: OffsetDateTime) {
        self.events
            .push(PresentationEvent::Renamed(PresentationRenamed {
                presentation_id: self.id,
                title: title.as_str().to_owned(),
                at: now,
            }));
        self.title = title;
        self.updated_at = now;
    }

    /// Marks the presentation deleted (the repository performs the soft delete).
    pub(crate) fn delete(&mut self, now: OffsetDateTime) {
        self.events
            .push(PresentationEvent::Deleted(PresentationDeleted {
                presentation_id: self.id,
                at: now,
            }));
        self.updated_at = now;
    }

    /// Presentation id.
    #[must_use]
    pub(crate) const fn id(&self) -> PresentationId {
        self.id
    }

    /// Owner id.
    #[must_use]
    pub(crate) const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    /// Current title.
    #[must_use]
    pub(crate) const fn title(&self) -> &PresentationTitle {
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
    pub(crate) fn take_events(&mut self) -> Vec<PresentationEvent> {
        std::mem::take(&mut self.events)
    }

    /// JSON borrowed from the deck, for persistence.
    #[must_use]
    pub(crate) const fn content_value(&self) -> &Value {
        self.content.as_value()
    }
}

#[cfg(test)]
mod tests {
    use shared_kernel::Uuid;

    use super::*;

    fn deck() -> Presentation {
        Presentation::create(
            PresentationId::from_uuid(Uuid::nil()),
            OwnerId::from_uuid(Uuid::nil()),
            PresentationTitle::new("Budget").unwrap(),
            "slide-1",
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn rename_collects_renamed_event_and_keeps_version() {
        let mut deck = deck();
        deck.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
        deck.rename(PresentationTitle::new("Plan").unwrap(), later);
        assert_eq!(
            deck.take_events(),
            [PresentationEvent::Renamed(PresentationRenamed {
                presentation_id: deck.id(),
                title: "Plan".to_owned(),
                at: later,
            })]
        );
        assert_eq!(deck.version(), 1);
    }

    #[test]
    fn delete_collects_deleted_event() {
        let mut deck = deck();
        deck.take_events();
        let later = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(2);
        deck.delete(later);
        assert_eq!(
            deck.take_events(),
            [PresentationEvent::Deleted(PresentationDeleted {
                presentation_id: deck.id(),
                at: later
            })]
        );
        assert_eq!(deck.updated_at(), later);
    }

    #[test]
    fn create_records_one_event_and_a_deck() {
        let mut deck = Presentation::create(
            PresentationId::from_uuid(Uuid::nil()),
            OwnerId::from_uuid(Uuid::nil()),
            PresentationTitle::new("Budget").unwrap(),
            "slide-1",
            OffsetDateTime::UNIX_EPOCH,
        );
        assert_eq!(deck.version(), 1);
        assert_eq!(deck.content_value()["slides"][0]["id"], "slide-1");
        assert_eq!(deck.take_events().len(), 1);
        assert!(deck.take_events().is_empty());
    }
}
