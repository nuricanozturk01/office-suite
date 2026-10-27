use std::sync::Arc;

use identity_contracts::{UserId, UserProfiles};
use shared_kernel::{Clock, Uuid};
use storage_contracts::{MAX_PACKAGE_BYTES, MergeError, PackageMerger};
use thiserror::Error;
use workspace_contracts::{PermissionChecker, ResourceId};

use super::{HistoryEntry, PackageRepository, RepositoryError, StoredPackage};
use crate::domain::PackageName;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum StorageError {
    /// Invalid input.
    #[error("invalid input")]
    Invalid,
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("not found")]
    NotFound,
    /// The package changed since the caller read it.
    #[error("version conflict")]
    Conflict,
    /// The file exceeds the size cap.
    #[error("file is too large")]
    TooLarge,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<RepositoryError> for StorageError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::Conflict => Self::Conflict,
            RepositoryError::Storage => Self::Internal,
        }
    }
}

/// What a save produced.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SaveOutcome {
    /// Version after the save.
    pub(crate) version: i64,
    /// Somebody else's concurrent changes were merged into the saved file.
    pub(crate) merged: bool,
}

/// Kept versions of a file, with who may act on them.
#[derive(Debug)]
pub(crate) struct VersionList {
    /// The caller may restore an older version.
    pub(crate) can_restore: bool,
    /// Newest first, each with the name of whoever saved it (when known).
    pub(crate) entries: Vec<(HistoryEntry, Option<String>)>,
}

/// Package use cases; access follows the role the workspace grants on the resource.
pub(crate) struct StorageService {
    repo: Arc<dyn PackageRepository>,
    checker: Arc<dyn PermissionChecker>,
    clock: Arc<dyn Clock>,
    profiles: Arc<dyn UserProfiles>,
    mergers: Vec<Arc<dyn PackageMerger>>,
}

impl std::fmt::Debug for StorageService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageService").finish_non_exhaustive()
    }
}

impl StorageService {
    pub(crate) fn new(
        repo: Arc<dyn PackageRepository>,
        checker: Arc<dyn PermissionChecker>,
        clock: Arc<dyn Clock>,
        profiles: Arc<dyn UserProfiles>,
        mergers: Vec<Arc<dyn PackageMerger>>,
    ) -> Self {
        Self {
            repo,
            checker,
            clock,
            profiles,
            mergers,
        }
    }

    pub(crate) async fn read(
        &self,
        actor: Uuid,
        resource: Uuid,
    ) -> Result<StoredPackage, StorageError> {
        let role = self.role(actor, resource).await;
        if !role.is_some_and(workspace_contracts::Role::can_read) {
            return Err(StorageError::NotFound);
        }
        self.repo.get(resource).await?.ok_or(StorageError::NotFound)
    }

    /// Saves a file. With `merge`, a save based on an older version is combined with the
    /// changes made since (when a merger understands the format) instead of being refused.
    pub(crate) async fn write(
        &self,
        actor: Uuid,
        resource: Uuid,
        name: &str,
        bytes: &[u8],
        base_version: Option<i64>,
        merge: bool,
    ) -> Result<SaveOutcome, StorageError> {
        let role = self.role(actor, resource).await;
        if !role.is_some_and(workspace_contracts::Role::can_write) {
            return Err(StorageError::NotFound);
        }
        let name = PackageName::new(name).map_err(|_| StorageError::Invalid)?;
        if bytes.is_empty() {
            return Err(StorageError::Invalid);
        }
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(StorageError::TooLarge);
        }
        let first = self
            .repo
            .put(
                resource,
                name.as_str(),
                bytes,
                base_version,
                actor,
                self.clock.now(),
            )
            .await;
        match (first, base_version) {
            (Ok(version), _) => Ok(SaveOutcome {
                version,
                merged: false,
            }),
            (Err(RepositoryError::Conflict), Some(base)) if merge => {
                self.save_merged(actor, resource, &name, bytes, base).await
            }
            (Err(err), _) => Err(err.into()),
        }
    }

    async fn save_merged(
        &self,
        actor: Uuid,
        resource: Uuid,
        name: &PackageName,
        ours: &[u8],
        base: i64,
    ) -> Result<SaveOutcome, StorageError> {
        let format = self
            .mergers
            .iter()
            .find(|m| m.handles(name.as_str()))
            .cloned()
            .ok_or(StorageError::Conflict)?;
        let theirs = self
            .repo
            .get(resource)
            .await?
            .ok_or(StorageError::Conflict)?;
        let base_file = self
            .repo
            .version(resource, base)
            .await?
            .ok_or(StorageError::Conflict)?;
        let (base_bytes, ours_bytes, theirs_bytes) = (base_file.bytes, ours.to_vec(), theirs.bytes);
        let merged = tokio::task::spawn_blocking(move || {
            format.merge(&base_bytes, &ours_bytes, &theirs_bytes)
        })
        .await
        .map_err(|_| StorageError::Internal)?
        .map_err(|err| match err {
            MergeError::Conflict | MergeError::Unreadable => StorageError::Conflict,
        })?;
        if merged.len() > MAX_PACKAGE_BYTES {
            return Err(StorageError::TooLarge);
        }
        // Still guarded by the lock: if somebody saved while we merged, the caller retries.
        let version = self
            .repo
            .put(
                resource,
                name.as_str(),
                &merged,
                Some(theirs.version),
                actor,
                self.clock.now(),
            )
            .await?;
        Ok(SaveOutcome {
            version,
            merged: true,
        })
    }

    /// Kept versions of a file, newest first.
    pub(crate) async fn history(
        &self,
        actor: Uuid,
        resource: Uuid,
    ) -> Result<VersionList, StorageError> {
        let role = self.role(actor, resource).await;
        if !role.is_some_and(workspace_contracts::Role::can_read) {
            return Err(StorageError::NotFound);
        }
        let can_restore = role.is_some_and(workspace_contracts::Role::can_write);
        let history = self.repo.history(resource).await?;
        let mut names: std::collections::HashMap<Uuid, Option<String>> =
            std::collections::HashMap::new();
        for entry in &history {
            if let std::collections::hash_map::Entry::Vacant(slot) = names.entry(entry.saved_by) {
                let name = self
                    .profiles
                    .display_name(UserId::from_uuid(entry.saved_by))
                    .await;
                slot.insert(name);
            }
        }
        let entries = history
            .into_iter()
            .map(|entry| {
                let name = names.get(&entry.saved_by).cloned().flatten();
                (entry, name)
            })
            .collect();
        Ok(VersionList {
            can_restore,
            entries,
        })
    }

    /// A kept version's file.
    pub(crate) async fn read_version(
        &self,
        actor: Uuid,
        resource: Uuid,
        version: i64,
    ) -> Result<StoredPackage, StorageError> {
        let role = self.role(actor, resource).await;
        if !role.is_some_and(workspace_contracts::Role::can_read) {
            return Err(StorageError::NotFound);
        }
        self.repo
            .version(resource, version)
            .await?
            .ok_or(StorageError::NotFound)
    }

    /// Makes a kept version the current one, as a new version.
    pub(crate) async fn restore(
        &self,
        actor: Uuid,
        resource: Uuid,
        version: i64,
    ) -> Result<SaveOutcome, StorageError> {
        let old = self.read_version(actor, resource, version).await?;
        self.write(actor, resource, &old.name, &old.bytes, None, false)
            .await
    }

    async fn role(&self, actor: Uuid, resource: Uuid) -> Option<workspace_contracts::Role> {
        self.checker
            .role(UserId::from_uuid(actor), ResourceId::from_uuid(resource))
            .await
    }
}
