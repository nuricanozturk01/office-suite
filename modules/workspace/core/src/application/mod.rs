//! Use cases and outbound ports.

mod ports;
mod service;

pub(crate) use ports::{
    OutboxEvent, Page, RepositoryError, ResourceChange, ResourceInfo, ResourceStore, ShareRecord,
    SharedItem, WorkspaceRepository,
};
pub(crate) use service::{WorkspaceError, WorkspaceService};
