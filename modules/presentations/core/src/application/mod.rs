//! Use cases and outbound ports.

mod policy;
mod ports;
mod service;

pub(crate) use policy::OwnerOrShared;
pub(crate) use ports::{
    AccessPolicy, Action, Page, PresentationRepository, PresentationSummary, RepositoryError,
};
pub(crate) use service::{PresentationError, PresentationService};
