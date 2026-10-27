use async_trait::async_trait;
use shared_kernel::Uuid;
use thiserror::Error;
use time::OffsetDateTime;

/// Storage failure.
#[derive(Debug, Error)]
pub(crate) enum RepositoryError {
    /// The base version no longer matches.
    #[error("version conflict")]
    Conflict,
    /// Infrastructure failure.
    #[error("storage failure")]
    Storage,
}

/// A stored file.
#[derive(Debug)]
pub(crate) struct StoredPackage {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) version: i64,
}

/// One saved version, without its bytes.
#[derive(Debug)]
pub(crate) struct HistoryEntry {
    pub(crate) version: i64,
    pub(crate) name: String,
    pub(crate) size: i64,
    pub(crate) saved_by: Uuid,
    pub(crate) saved_at: OffsetDateTime,
}

/// Persistence of file packages.
#[async_trait]
pub(crate) trait PackageRepository: Send + Sync + std::fmt::Debug {
    async fn get(&self, resource: Uuid) -> Result<Option<StoredPackage>, RepositoryError>;

    /// Inserts or replaces the package. With `base_version`, replaces only that version.
    async fn put(
        &self,
        resource: Uuid,
        name: &str,
        bytes: &[u8],
        base_version: Option<i64>,
        actor: Uuid,
        now: OffsetDateTime,
    ) -> Result<i64, RepositoryError>;

    /// A past version, if still kept.
    async fn version(
        &self,
        resource: Uuid,
        version: i64,
    ) -> Result<Option<StoredPackage>, RepositoryError>;

    /// Kept versions, newest first.
    async fn history(&self, resource: Uuid) -> Result<Vec<HistoryEntry>, RepositoryError>;
}
