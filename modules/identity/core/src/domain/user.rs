use identity_contracts::{ProfileUpdated, UserDeletionScheduled, UserId, UserRegistered};
use time::OffsetDateTime;

use super::{AvatarUrl, Bio, DisplayName, Email, ValueError};

/// User aggregate root.
#[derive(Debug)]
pub struct User {
    id: UserId,
    email: Email,
    display_name: DisplayName,
    password_hash: String,
    avatar_url: Option<AvatarUrl>,
    bio: Option<Bio>,
    deletion_scheduled_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    events: Vec<UserRegistered>,
}

impl User {
    /// Registers a new user and records a `UserRegistered` event.
    #[must_use]
    pub fn register(
        id: UserId,
        email: Email,
        display_name: DisplayName,
        password_hash: String,
        now: OffsetDateTime,
    ) -> Self {
        let event = UserRegistered {
            user_id: id,
            display_name: display_name.as_str().to_owned(),
            registered_at: now,
        };
        Self {
            id,
            email,
            display_name,
            password_hash,
            avatar_url: None,
            bio: None,
            deletion_scheduled_at: None,
            created_at: now,
            events: vec![event],
        }
    }

    /// Rebuilds a persisted user (no events).
    #[must_use]
    // INVARIANT: restore() takes all User fields; this count is fixed by the domain model.
    #[expect(
        clippy::too_many_arguments,
        reason = "all fields are required to rebuild the aggregate"
    )]
    pub const fn restore(
        id: UserId,
        email: Email,
        display_name: DisplayName,
        password_hash: String,
        avatar_url: Option<AvatarUrl>,
        bio: Option<Bio>,
        deletion_scheduled_at: Option<OffsetDateTime>,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            email,
            display_name,
            password_hash,
            avatar_url,
            bio,
            deletion_scheduled_at,
            created_at,
            events: Vec::new(),
        }
    }

    /// Updates the user's profile and returns a `ProfileUpdated` event.
    pub fn update_profile(
        &mut self,
        display_name: DisplayName,
        bio: Option<Bio>,
        avatar_url: Option<AvatarUrl>,
        now: OffsetDateTime,
    ) -> ProfileUpdated {
        self.display_name = display_name;
        self.bio = bio;
        self.avatar_url = avatar_url;
        ProfileUpdated {
            user_id: self.id,
            display_name: self.display_name.as_str().to_owned(),
            updated_at: now,
        }
    }

    /// Updates the password hash. Caller must verify the old password before calling.
    ///
    /// # Errors
    /// [`ValueError::WrongPassword`] when `old_hash_matches` is false.
    pub fn change_password(
        &mut self,
        old_hash_matches: bool,
        new_hash: String,
    ) -> Result<(), ValueError> {
        if !old_hash_matches {
            return Err(ValueError::WrongPassword);
        }
        self.password_hash = new_hash;
        Ok(())
    }

    /// Schedules account deletion 30 days from `now`.
    ///
    /// # Errors
    /// [`ValueError::DeletionAlreadyScheduled`] when a deletion is already pending.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "&mut self methods cannot be const in stable Rust"
    )]
    pub fn schedule_deletion(
        &mut self,
        now: OffsetDateTime,
    ) -> Result<UserDeletionScheduled, ValueError> {
        if self.deletion_scheduled_at.is_some() {
            return Err(ValueError::DeletionAlreadyScheduled);
        }
        // INVARIANT: 30-day grace period; saturating_add avoids overflow on extreme dates.
        let execute_after = now.saturating_add(time::Duration::days(30));
        self.deletion_scheduled_at = Some(execute_after);
        Ok(UserDeletionScheduled {
            user_id: self.id,
            scheduled_at: now,
            execute_after,
        })
    }

    /// Cancels a pending deletion.
    ///
    /// # Errors
    /// [`ValueError::NoPendingDeletion`] when no deletion is scheduled.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "&mut self methods cannot be const in stable Rust"
    )]
    pub fn cancel_deletion(&mut self) -> Result<(), ValueError> {
        if self.deletion_scheduled_at.is_none() {
            return Err(ValueError::NoPendingDeletion);
        }
        self.deletion_scheduled_at = None;
        Ok(())
    }

    /// User id.
    #[must_use]
    pub const fn id(&self) -> UserId {
        self.id
    }

    /// Email.
    #[must_use]
    pub const fn email(&self) -> &Email {
        &self.email
    }

    /// Display name.
    #[must_use]
    pub const fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    /// PHC-encoded password hash.
    #[must_use]
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    /// Avatar URL.
    #[must_use]
    pub const fn avatar_url(&self) -> Option<&AvatarUrl> {
        self.avatar_url.as_ref()
    }

    /// Bio.
    #[must_use]
    pub const fn bio(&self) -> Option<&Bio> {
        self.bio.as_ref()
    }

    /// When the account will be deleted; `None` if no deletion is scheduled.
    #[must_use]
    pub const fn deletion_scheduled_at(&self) -> Option<OffsetDateTime> {
        self.deletion_scheduled_at
    }

    /// Registration time.
    #[must_use]
    pub const fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    /// Drains collected domain events.
    pub fn take_events(&mut self) -> Vec<UserRegistered> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_user() -> User {
        User::register(
            UserId::from_uuid(shared_kernel::Uuid::now_v7()),
            Email::parse("test@example.com").unwrap(),
            DisplayName::parse("Test User").unwrap(),
            "hash".to_owned(),
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    #[test]
    fn update_profile_changes_display_name_and_bio() {
        let mut user = make_test_user();
        let new_name = DisplayName::parse("Updated Name").unwrap();
        let bio = Bio::new("Hello").unwrap();
        let _event = user.update_profile(new_name, Some(bio), None, OffsetDateTime::UNIX_EPOCH);
        assert_eq!(user.display_name().as_str(), "Updated Name");
        assert!(user.bio().is_some());
        assert!(user.avatar_url().is_none());
    }

    #[test]
    fn schedule_deletion_sets_date_30_days_out() {
        let mut user = make_test_user();
        let now = OffsetDateTime::UNIX_EPOCH;
        let event = user.schedule_deletion(now).unwrap();
        assert!(user.deletion_scheduled_at().is_some());
        let diff = event.execute_after - now;
        assert!(diff.whole_days() >= 29 && diff.whole_days() <= 31);
    }

    #[test]
    fn schedule_deletion_twice_returns_err() {
        let mut user = make_test_user();
        let now = OffsetDateTime::UNIX_EPOCH;
        user.schedule_deletion(now).unwrap();
        assert_eq!(
            user.schedule_deletion(now),
            Err(ValueError::DeletionAlreadyScheduled)
        );
    }

    #[test]
    fn cancel_deletion_clears_date() {
        let mut user = make_test_user();
        let now = OffsetDateTime::UNIX_EPOCH;
        user.schedule_deletion(now).unwrap();
        user.cancel_deletion().unwrap();
        assert!(user.deletion_scheduled_at().is_none());
    }

    #[test]
    fn cancel_deletion_without_pending_returns_err() {
        let mut user = make_test_user();
        assert_eq!(user.cancel_deletion(), Err(ValueError::NoPendingDeletion));
    }

    #[test]
    fn change_password_wrong_current_returns_err() {
        let mut user = make_test_user();
        assert_eq!(
            user.change_password(false, "new_hash".to_owned()),
            Err(ValueError::WrongPassword)
        );
    }

    #[test]
    fn change_password_updates_hash() {
        let mut user = make_test_user();
        user.change_password(true, "new_hash".to_owned()).unwrap();
        assert_eq!(user.password_hash(), "new_hash");
    }
}
