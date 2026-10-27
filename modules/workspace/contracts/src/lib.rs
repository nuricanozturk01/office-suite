//! Public surface of the `workspace` module: sharing roles, the permission port and events.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use async_trait::async_trait;
use identity_contracts::UserId;
use serde::{Deserialize, Serialize};
use shared_kernel::typed_id;
use time::OffsetDateTime;
use utoipa::ToSchema;

typed_id!(
    /// Identifier of a shareable resource (a document or a spreadsheet).
    ResourceId
);

/// What a user may do with a resource; ordered from least to most privileged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// May read.
    Viewer,
    /// May read and comment.
    Commenter,
    /// May read and change content.
    Editor,
    /// Full control, including sharing and deletion. Never grantable through a share.
    Owner,
}

impl Role {
    /// Whether content may be changed.
    #[must_use]
    pub const fn can_write(self) -> bool {
        matches!(self, Self::Editor | Self::Owner)
    }

    /// Whether content may be read (every role).
    #[must_use]
    pub const fn can_read(self) -> bool {
        true
    }
}

/// Kind of a shareable resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// A Docs document.
    Document,
    /// A Sheets spreadsheet.
    Spreadsheet,
    /// A Slides presentation.
    Presentation,
    /// A PDF file.
    Pdf,
}

/// Resolves a user's role on a resource. Implemented by `workspace`; consumed by modules that
/// guard resource content.
#[async_trait]
pub trait PermissionChecker: Send + Sync + std::fmt::Debug {
    /// Role of `user` on `resource`, or `None` when there is no access (or no such resource).
    async fn role(&self, user: UserId, resource: ResourceId) -> Option<Role>;
}

/// Emitted (via outbox) when a resource is shared with a user or their role changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceShared {
    /// Shared resource.
    pub resource_id: ResourceId,
    /// User who gained (or changed) access.
    pub grantee: UserId,
    /// Granted role.
    pub role: Role,
    /// User who shared.
    pub shared_by: UserId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Emitted (via outbox) when a user loses access or is downgraded; open sessions must re-check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRevoked {
    /// Affected resource.
    pub resource_id: ResourceId,
    /// User whose access shrank.
    pub user_id: UserId,
    /// Time of the change (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// Error returned by the workspace export port.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// Infrastructure failure.
    #[error("storage failure")]
    Storage,
}

/// A single file row as returned by the GDPR export.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileExportRow {
    /// Resource UUID.
    pub id: String,
    /// User-facing file name.
    pub name: String,
    /// Kind: "document" | "spreadsheet" | "presentation" | "pdf".
    pub kind: String,
    /// Creation time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Last modification time (UTC).
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Port for GDPR data export: lists files owned by a user. Implemented by `workspace`.
#[async_trait]
pub trait WorkspaceExportPort: Send + Sync + std::fmt::Debug {
    /// All non-deleted resources owned by `owner`, newest first.
    async fn list_owned_files(&self, owner: UserId) -> Result<Vec<FileExportRow>, ExportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_ordered() {
        assert!(Role::Viewer < Role::Commenter);
        assert!(Role::Commenter < Role::Editor);
        assert!(Role::Editor < Role::Owner);
    }

    #[test]
    fn only_editor_and_owner_can_write() {
        let writers: Vec<_> = [Role::Viewer, Role::Commenter, Role::Editor, Role::Owner]
            .into_iter()
            .filter(|r| r.can_write())
            .collect();
        assert_eq!(writers, [Role::Editor, Role::Owner]);
        assert!(Role::Viewer.can_read());
    }

    #[test]
    fn role_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&Role::Editor).unwrap(), "\"editor\"");
        assert_eq!(
            serde_json::to_string(&ResourceKind::Spreadsheet).unwrap(),
            "\"spreadsheet\""
        );
    }

    #[test]
    fn permission_revoked_json_roundtrip() {
        let event = PermissionRevoked {
            resource_id: ResourceId::from_uuid(shared_kernel::Uuid::nil()),
            user_id: UserId::from_uuid(shared_kernel::Uuid::nil()),
            at: OffsetDateTime::UNIX_EPOCH,
        };
        let back: PermissionRevoked =
            serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
        assert_eq!(back, event);
    }
}
