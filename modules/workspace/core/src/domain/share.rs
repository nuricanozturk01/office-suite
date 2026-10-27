use identity_contracts::UserId;
use thiserror::Error;
use workspace_contracts::{ResourceId, Role};

/// Why a share cannot be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ShareError {
    /// Ownership is never granted through a share.
    #[error("the owner role cannot be granted")]
    OwnerRoleNotGrantable,
    /// The owner already has full access.
    #[error("cannot share a resource with its owner")]
    CannotShareWithOwner,
}

/// A grant of `role` on one resource to one user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Share {
    resource_id: ResourceId,
    grantee: UserId,
    role: Role,
    shared_by: UserId,
}

impl Share {
    /// Validates and builds a share; `owner` is the resource owner.
    ///
    /// # Errors
    /// [`ShareError`] for the owner role or when `grantee` is the owner.
    pub fn new(
        resource_id: ResourceId,
        owner: UserId,
        grantee: UserId,
        role: Role,
        shared_by: UserId,
    ) -> Result<Self, ShareError> {
        if role == Role::Owner {
            return Err(ShareError::OwnerRoleNotGrantable);
        }
        if grantee == owner {
            return Err(ShareError::CannotShareWithOwner);
        }
        Ok(Self {
            resource_id,
            grantee,
            role,
            shared_by,
        })
    }

    /// Shared resource.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// User who receives access.
    #[must_use]
    pub const fn grantee(&self) -> UserId {
        self.grantee
    }

    /// Granted role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// User who shared.
    #[must_use]
    pub const fn shared_by(&self) -> UserId {
        self.shared_by
    }
}

#[cfg(test)]
mod tests {
    use shared_kernel::Uuid;

    use super::*;

    fn ids() -> (ResourceId, UserId, UserId) {
        (
            ResourceId::from_uuid(Uuid::from_u128(1)),
            UserId::from_uuid(Uuid::from_u128(2)),
            UserId::from_uuid(Uuid::from_u128(3)),
        )
    }

    #[test]
    fn owner_role_is_not_grantable() {
        let (res, owner, other) = ids();
        assert_eq!(
            Share::new(res, owner, other, Role::Owner, owner),
            Err(ShareError::OwnerRoleNotGrantable)
        );
    }

    #[test]
    fn cannot_share_with_owner() {
        let (res, owner, _) = ids();
        assert_eq!(
            Share::new(res, owner, owner, Role::Editor, owner),
            Err(ShareError::CannotShareWithOwner)
        );
    }

    #[test]
    fn valid_share_exposes_fields() {
        let (res, owner, other) = ids();
        let share = Share::new(res, owner, other, Role::Commenter, owner).unwrap();
        assert_eq!(
            (
                share.resource_id(),
                share.grantee(),
                share.role(),
                share.shared_by()
            ),
            (res, other, Role::Commenter, owner)
        );
    }
}
