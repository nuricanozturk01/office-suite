//! Adapter exposing the user repository as the `UserDirectory` and `UserProfiles` contract ports.

use std::sync::Arc;

use async_trait::async_trait;
use identity_contracts::{UserDirectory, UserId, UserProfiles};

use crate::{application::UserRepository, domain::Email};

pub(crate) struct RepositoryDirectory(pub(crate) Arc<dyn UserRepository>);

impl std::fmt::Debug for RepositoryDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepositoryDirectory")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl UserDirectory for RepositoryDirectory {
    async fn find_by_email(&self, email: &str) -> Option<UserId> {
        let email = Email::parse(email).ok()?;
        self.0
            .find_by_email(&email)
            .await
            .ok()
            .flatten()
            .map(|u| u.id())
    }
}

#[async_trait]
impl UserProfiles for RepositoryDirectory {
    async fn display_name(&self, user: UserId) -> Option<String> {
        self.0
            .find_by_id(user)
            .await
            .ok()
            .flatten()
            .map(|u| u.display_name().as_str().to_owned())
    }
}
