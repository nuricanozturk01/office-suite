use std::sync::Arc;

use identity_contracts::{UserDirectory, UserId};
use shared_kernel::Clock;
use thiserror::Error;
use workspace_contracts::{PermissionRevoked, ResourceId, ResourceShared, Role};

use super::{
    OutboxEvent, Page, RepositoryError, ResourceInfo, ShareRecord, SharedItem, WorkspaceRepository,
};
use crate::domain::{Share, ShareError};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;

/// Use-case failure.
#[derive(Debug, Error)]
pub(crate) enum WorkspaceError {
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("not found")]
    NotFound,
    /// No account is registered under the given email.
    #[error("user not found")]
    UserNotFound,
    /// The share violates a domain rule.
    #[error(transparent)]
    Invalid(#[from] ShareError),
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

impl From<RepositoryError> for WorkspaceError {
    fn from(_: RepositoryError) -> Self {
        Self::Internal
    }
}

/// Sharing use cases. Only a resource's owner may manage its shares.
pub(crate) struct WorkspaceService {
    repo: Arc<dyn WorkspaceRepository>,
    directory: Arc<dyn UserDirectory>,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for WorkspaceService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceService").finish_non_exhaustive()
    }
}

impl WorkspaceService {
    pub(crate) fn new(
        repo: Arc<dyn WorkspaceRepository>,
        directory: Arc<dyn UserDirectory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            directory,
            clock,
        }
    }

    /// The resource, if `actor` owns it; everything else is `NotFound`.
    async fn owned(&self, actor: UserId, id: ResourceId) -> Result<ResourceInfo, WorkspaceError> {
        match self.repo.find_resource(id).await? {
            Some(resource) if resource.owner_id == actor => Ok(resource),
            _ => Err(WorkspaceError::NotFound),
        }
    }

    /// Shares (or changes the role of) `email` on a resource.
    pub(crate) async fn share(
        &self,
        actor: UserId,
        id: ResourceId,
        email: &str,
        role: Role,
    ) -> Result<ShareRecord, WorkspaceError> {
        let resource = self.owned(actor, id).await?;
        let grantee = self
            .directory
            .find_by_email(email)
            .await
            .ok_or(WorkspaceError::UserNotFound)?;
        let share = Share::new(id, resource.owner_id, grantee, role, actor)?;
        let email = email.trim().to_lowercase();
        let now = self.clock.now();
        let events_for = |previous: Option<Role>| -> Result<Vec<OutboxEvent>, RepositoryError> {
            let shared = ResourceShared {
                resource_id: id,
                grantee,
                role,
                shared_by: actor,
                at: now,
            };
            let mut events = vec![event("ResourceShared", &shared, now)?];
            if previous.is_some_and(|old| old > role) {
                events.push(revoked(id, grantee, now)?);
            }
            Ok(events)
        };
        Ok(self
            .repo
            .upsert_share(&share, &email, now, &events_for)
            .await?)
    }

    /// Removes a user's access.
    pub(crate) async fn revoke(
        &self,
        actor: UserId,
        id: ResourceId,
        user: UserId,
    ) -> Result<(), WorkspaceError> {
        self.owned(actor, id).await?;
        let now = self.clock.now();
        let removed = self
            .repo
            .delete_share(id, user, &[revoked(id, user, now)?])
            .await?;
        if removed {
            Ok(())
        } else {
            Err(WorkspaceError::NotFound)
        }
    }

    /// Lists the shares of an owned resource.
    pub(crate) async fn list_shares(
        &self,
        actor: UserId,
        id: ResourceId,
    ) -> Result<Vec<ShareRecord>, WorkspaceError> {
        self.owned(actor, id).await?;
        Ok(self.repo.list_shares(id).await?)
    }

    /// Resources other users shared with `user`.
    pub(crate) async fn shared_with_me(
        &self,
        user: UserId,
        cursor: Option<ResourceId>,
        limit: Option<u32>,
    ) -> Result<Page<SharedItem>, WorkspaceError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        Ok(self.repo.shared_with_me(user, cursor, limit).await?)
    }
}

fn revoked(
    resource_id: ResourceId,
    user_id: UserId,
    at: time::OffsetDateTime,
) -> Result<OutboxEvent, RepositoryError> {
    event(
        "PermissionRevoked",
        &PermissionRevoked {
            resource_id,
            user_id,
            at,
        },
        at,
    )
}

fn event<T: serde::Serialize>(
    event_type: &'static str,
    body: &T,
    at: time::OffsetDateTime,
) -> Result<OutboxEvent, RepositoryError> {
    let payload = serde_json::to_value(body).map_err(|_| RepositoryError::Storage)?;
    Ok(OutboxEvent {
        event_type,
        payload,
        at,
    })
}

#[cfg(test)]
#[expect(
    clippy::significant_drop_tightening,
    reason = "test fakes hold short-lived locks"
)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use shared_kernel::Uuid;
    use time::OffsetDateTime;

    use super::*;

    const fn uid(n: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(n))
    }

    #[derive(Default)]
    struct State {
        owners: HashMap<ResourceId, UserId>,
        shares: Vec<ShareRecord>,
        resource_of_share: Vec<ResourceId>,
        events: Vec<&'static str>,
    }

    #[derive(Default)]
    struct FakeRepo(Mutex<State>);

    #[async_trait]
    impl WorkspaceRepository for FakeRepo {
        async fn find_resource(
            &self,
            id: ResourceId,
        ) -> Result<Option<ResourceInfo>, RepositoryError> {
            let s = self.0.lock().unwrap();
            Ok(s.owners.get(&id).map(|owner_id| ResourceInfo {
                owner_id: *owner_id,
            }))
        }
        async fn upsert_share(
            &self,
            share: &Share,
            email: &str,
            now: OffsetDateTime,
            events_for: &(
                 dyn Fn(Option<Role>) -> Result<Vec<OutboxEvent>, RepositoryError> + Send + Sync
             ),
        ) -> Result<ShareRecord, RepositoryError> {
            let mut s = self.0.lock().unwrap();
            let existing = s
                .shares
                .iter()
                .zip(&s.resource_of_share)
                .position(|(r, id)| r.grantee == share.grantee() && *id == share.resource_id());
            let previous = existing.map(|i| s.shares[i].role);
            let record = ShareRecord {
                grantee: share.grantee(),
                email: email.to_owned(),
                role: share.role(),
                shared_by: share.shared_by(),
                created_at: now,
                updated_at: now,
            };
            if let Some(i) = existing {
                s.shares[i] = record.clone();
            } else {
                s.shares.push(record.clone());
                s.resource_of_share.push(share.resource_id());
            }
            let events = events_for(previous)?;
            s.events.extend(events.iter().map(|e| e.event_type));
            Ok(record)
        }
        async fn delete_share(
            &self,
            resource: ResourceId,
            grantee: UserId,
            events: &[OutboxEvent],
        ) -> Result<bool, RepositoryError> {
            let mut s = self.0.lock().unwrap();
            let pos = s
                .shares
                .iter()
                .zip(&s.resource_of_share)
                .position(|(r, id)| r.grantee == grantee && *id == resource);
            Ok(pos.is_some_and(|i| {
                s.shares.remove(i);
                s.resource_of_share.remove(i);
                s.events.extend(events.iter().map(|e| e.event_type));
                true
            }))
        }
        async fn list_shares(
            &self,
            resource: ResourceId,
        ) -> Result<Vec<ShareRecord>, RepositoryError> {
            let s = self.0.lock().unwrap();
            Ok(s.shares
                .iter()
                .zip(&s.resource_of_share)
                .filter(|(_, id)| **id == resource)
                .map(|(r, _)| r.clone())
                .collect())
        }
        async fn shared_with_me(
            &self,
            _user: UserId,
            _cursor: Option<ResourceId>,
            _limit: u32,
        ) -> Result<Page<SharedItem>, RepositoryError> {
            Ok(Page {
                items: Vec::new(),
                next_cursor: None,
            })
        }
    }

    #[derive(Debug)]
    struct FakeDirectory;

    #[async_trait]
    impl UserDirectory for FakeDirectory {
        async fn find_by_email(&self, email: &str) -> Option<UserId> {
            match email {
                "owner@x.com" => Some(uid(1)),
                "bob@x.com" => Some(uid(2)),
                _ => None,
            }
        }
    }

    struct Fixed;

    impl Clock for Fixed {
        fn now(&self) -> OffsetDateTime {
            OffsetDateTime::UNIX_EPOCH
        }
    }

    fn setup() -> (WorkspaceService, Arc<FakeRepo>, ResourceId) {
        let repo = Arc::new(FakeRepo::default());
        let resource = ResourceId::from_uuid(Uuid::from_u128(100));
        repo.0.lock().unwrap().owners.insert(resource, uid(1));
        let service = WorkspaceService::new(repo.clone(), Arc::new(FakeDirectory), Arc::new(Fixed));
        (service, repo, resource)
    }

    #[tokio::test]
    async fn non_owner_gets_not_found() {
        let (service, _, res) = setup();
        let err = service
            .share(uid(2), res, "bob@x.com", Role::Editor)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound));
        let unknown = ResourceId::from_uuid(Uuid::from_u128(999));
        let err = service
            .share(uid(1), unknown, "bob@x.com", Role::Editor)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound));
    }

    #[tokio::test]
    async fn unknown_email_is_user_not_found() {
        let (service, _, res) = setup();
        let err = service
            .share(uid(1), res, "ghost@x.com", Role::Viewer)
            .await
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::UserNotFound));
    }

    #[tokio::test]
    async fn sharing_with_owner_or_as_owner_role_is_rejected() {
        let (service, _, res) = setup();
        let err = service
            .share(uid(1), res, "owner@x.com", Role::Editor)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            WorkspaceError::Invalid(ShareError::CannotShareWithOwner)
        ));
        let err = service
            .share(uid(1), res, "bob@x.com", Role::Owner)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            WorkspaceError::Invalid(ShareError::OwnerRoleNotGrantable)
        ));
    }

    #[tokio::test]
    async fn share_stores_the_normalized_email() {
        let (service, _, res) = setup();
        let record = service
            .share(uid(1), res, "  BOB@x.com ", Role::Viewer)
            .await;
        // The directory only knows the lowercase address, so the padded variant is unknown...
        assert!(matches!(record, Err(WorkspaceError::UserNotFound)));
        let record = service
            .share(uid(1), res, "bob@x.com", Role::Viewer)
            .await
            .unwrap();
        assert_eq!(record.email, "bob@x.com");
    }

    #[tokio::test]
    async fn share_twice_updates_role() {
        let (service, repo, res) = setup();
        service
            .share(uid(1), res, "bob@x.com", Role::Viewer)
            .await
            .unwrap();
        let record = service
            .share(uid(1), res, "bob@x.com", Role::Editor)
            .await
            .unwrap();
        assert_eq!(record.role, Role::Editor);
        assert_eq!(repo.0.lock().unwrap().shares.len(), 1);
    }

    #[tokio::test]
    async fn downgrade_emits_permission_revoked_but_upgrade_does_not() {
        let (service, repo, res) = setup();
        service
            .share(uid(1), res, "bob@x.com", Role::Viewer)
            .await
            .unwrap();
        service
            .share(uid(1), res, "bob@x.com", Role::Editor)
            .await
            .unwrap();
        assert_eq!(
            repo.0.lock().unwrap().events,
            ["ResourceShared", "ResourceShared"]
        );
        service
            .share(uid(1), res, "bob@x.com", Role::Viewer)
            .await
            .unwrap();
        let events = repo.0.lock().unwrap().events.clone();
        assert_eq!(&events[2..], ["ResourceShared", "PermissionRevoked"]);
    }

    #[tokio::test]
    async fn revoke_removes_share_and_emits_event_once() {
        let (service, repo, res) = setup();
        service
            .share(uid(1), res, "bob@x.com", Role::Editor)
            .await
            .unwrap();
        service.revoke(uid(1), res, uid(2)).await.unwrap();
        assert_eq!(
            repo.0.lock().unwrap().events.last(),
            Some(&"PermissionRevoked")
        );
        let err = service.revoke(uid(1), res, uid(2)).await.unwrap_err();
        assert!(matches!(err, WorkspaceError::NotFound));
    }
}
