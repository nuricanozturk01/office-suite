//! Use cases and outbound ports.

mod ports;
mod service;

pub(crate) use ports::{HistoryEntry, PackageRepository, RepositoryError, StoredPackage};
pub(crate) use service::{StorageError, StorageService, VersionList};
