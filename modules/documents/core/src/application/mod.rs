//! Use cases and outbound ports.

mod convert;
mod policy;
mod ports;
mod service;

pub(crate) use convert::{export_docx, import_docx};
pub(crate) use policy::OwnerOrShared;
pub(crate) use ports::{AccessPolicy, Action, DocumentRepository, Page, RepositoryError};
pub(crate) use service::{DocumentError, DocumentService};
