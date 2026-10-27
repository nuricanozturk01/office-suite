//! Event consumers that keep the resource read-model in sync with documents and spreadsheets.

use std::sync::Arc;

use async_trait::async_trait;
use documents_contracts::{DocumentCreated, DocumentDeleted, DocumentRenamed};
use identity_contracts::UserId;
use pdfs_contracts::{PdfCreated, PdfDeleted, PdfRenamed};
use platform::event_bus::{EventEnvelope, EventHandler, HandlerError};
use presentations_contracts::{PresentationCreated, PresentationDeleted, PresentationRenamed};
use serde::de::DeserializeOwned;
use serde_json::Value;
use spreadsheets_contracts::{SpreadsheetCreated, SpreadsheetDeleted, SpreadsheetRenamed};
use workspace_contracts::{ResourceId, ResourceKind};

use crate::application::{ResourceChange, ResourceStore};

type Decode = fn(&Value) -> Option<ResourceChange>;

struct Projection {
    consumer: &'static str,
    event_type: &'static str,
    decode: Decode,
    store: Arc<dyn ResourceStore>,
}

#[async_trait]
impl EventHandler for Projection {
    fn consumer(&self) -> &'static str {
        self.consumer
    }

    fn event_type(&self) -> &'static str {
        self.event_type
    }

    async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError> {
        let change = (self.decode)(&event.payload)
            .ok_or_else(|| HandlerError::Permanent("undecodable event payload".to_owned()))?;
        self.store
            .apply(self.consumer, event.id, change)
            .await
            .map_err(|e| HandlerError::Transient(e.to_string()))
    }
}

fn parse<T: DeserializeOwned>(payload: &Value) -> Option<T> {
    serde_json::from_value(payload.clone()).ok()
}

fn document_created(p: &Value) -> Option<ResourceChange> {
    let e: DocumentCreated = parse(p)?;
    Some(ResourceChange::Created {
        id: ResourceId::from_uuid(e.document_id.as_uuid()),
        kind: ResourceKind::Document,
        owner: UserId::from_uuid(e.owner_id.as_uuid()),
        title: e.title,
        at: e.created_at,
    })
}

fn document_renamed(p: &Value) -> Option<ResourceChange> {
    let e: DocumentRenamed = parse(p)?;
    Some(ResourceChange::Renamed {
        id: ResourceId::from_uuid(e.document_id.as_uuid()),
        title: e.title,
        at: e.at,
    })
}

fn document_deleted(p: &Value) -> Option<ResourceChange> {
    let e: DocumentDeleted = parse(p)?;
    Some(ResourceChange::Deleted {
        id: ResourceId::from_uuid(e.document_id.as_uuid()),
        at: e.at,
    })
}

fn spreadsheet_created(p: &Value) -> Option<ResourceChange> {
    let e: SpreadsheetCreated = parse(p)?;
    Some(ResourceChange::Created {
        id: ResourceId::from_uuid(e.spreadsheet_id.as_uuid()),
        kind: ResourceKind::Spreadsheet,
        owner: UserId::from_uuid(e.owner_id.as_uuid()),
        title: e.title,
        at: e.created_at,
    })
}

fn spreadsheet_renamed(p: &Value) -> Option<ResourceChange> {
    let e: SpreadsheetRenamed = parse(p)?;
    Some(ResourceChange::Renamed {
        id: ResourceId::from_uuid(e.spreadsheet_id.as_uuid()),
        title: e.title,
        at: e.at,
    })
}

fn spreadsheet_deleted(p: &Value) -> Option<ResourceChange> {
    let e: SpreadsheetDeleted = parse(p)?;
    Some(ResourceChange::Deleted {
        id: ResourceId::from_uuid(e.spreadsheet_id.as_uuid()),
        at: e.at,
    })
}

fn presentation_created(p: &Value) -> Option<ResourceChange> {
    let e: PresentationCreated = parse(p)?;
    Some(ResourceChange::Created {
        id: ResourceId::from_uuid(e.presentation_id.as_uuid()),
        kind: ResourceKind::Presentation,
        owner: UserId::from_uuid(e.owner_id.as_uuid()),
        title: e.title,
        at: e.created_at,
    })
}

fn presentation_renamed(p: &Value) -> Option<ResourceChange> {
    let e: PresentationRenamed = parse(p)?;
    Some(ResourceChange::Renamed {
        id: ResourceId::from_uuid(e.presentation_id.as_uuid()),
        title: e.title,
        at: e.at,
    })
}

fn presentation_deleted(p: &Value) -> Option<ResourceChange> {
    let e: PresentationDeleted = parse(p)?;
    Some(ResourceChange::Deleted {
        id: ResourceId::from_uuid(e.presentation_id.as_uuid()),
        at: e.at,
    })
}

fn pdf_created(p: &Value) -> Option<ResourceChange> {
    let e: PdfCreated = parse(p)?;
    Some(ResourceChange::Created {
        id: ResourceId::from_uuid(e.pdf_id.as_uuid()),
        kind: ResourceKind::Pdf,
        owner: UserId::from_uuid(e.owner_id.as_uuid()),
        title: e.title,
        at: e.created_at,
    })
}

fn pdf_renamed(p: &Value) -> Option<ResourceChange> {
    let e: PdfRenamed = parse(p)?;
    Some(ResourceChange::Renamed {
        id: ResourceId::from_uuid(e.pdf_id.as_uuid()),
        title: e.title,
        at: e.at,
    })
}

fn pdf_deleted(p: &Value) -> Option<ResourceChange> {
    let e: PdfDeleted = parse(p)?;
    Some(ResourceChange::Deleted {
        id: ResourceId::from_uuid(e.pdf_id.as_uuid()),
        at: e.at,
    })
}

/// One durable handler per consumed event type.
pub(crate) fn projections(store: &Arc<dyn ResourceStore>) -> Vec<Arc<dyn EventHandler>> {
    let specs: [(&'static str, &'static str, Decode); 12] = [
        (
            "workspace.resources.DocumentCreated",
            "DocumentCreated",
            document_created,
        ),
        (
            "workspace.resources.DocumentRenamed",
            "DocumentRenamed",
            document_renamed,
        ),
        (
            "workspace.resources.DocumentDeleted",
            "DocumentDeleted",
            document_deleted,
        ),
        (
            "workspace.resources.SpreadsheetCreated",
            "SpreadsheetCreated",
            spreadsheet_created,
        ),
        (
            "workspace.resources.SpreadsheetRenamed",
            "SpreadsheetRenamed",
            spreadsheet_renamed,
        ),
        (
            "workspace.resources.SpreadsheetDeleted",
            "SpreadsheetDeleted",
            spreadsheet_deleted,
        ),
        (
            "workspace.resources.PresentationCreated",
            "PresentationCreated",
            presentation_created,
        ),
        (
            "workspace.resources.PresentationRenamed",
            "PresentationRenamed",
            presentation_renamed,
        ),
        (
            "workspace.resources.PresentationDeleted",
            "PresentationDeleted",
            presentation_deleted,
        ),
        ("workspace.resources.PdfCreated", "PdfCreated", pdf_created),
        ("workspace.resources.PdfRenamed", "PdfRenamed", pdf_renamed),
        ("workspace.resources.PdfDeleted", "PdfDeleted", pdf_deleted),
    ];
    specs
        .into_iter()
        .map(|(consumer, event_type, decode)| {
            Arc::new(Projection {
                consumer,
                event_type,
                decode,
                store: store.clone(),
            }) as Arc<dyn EventHandler>
        })
        .collect()
}
