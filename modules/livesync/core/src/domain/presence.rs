use std::collections::HashMap;
use std::time::{Duration, Instant};

use livesync_contracts::Participant;
use shared_kernel::Uuid;

/// What a report from another instance told us that we did not know.
#[derive(Debug, Default)]
pub(crate) struct Seen {
    /// Participants not known before.
    pub(crate) fresh: Vec<Participant>,
    /// Known participants whose editing flag differs now.
    pub(crate) changed: Vec<(Uuid, bool)>,
}

/// Who is in one room: the connections of this instance and, learned over the bus, those of the
/// other instances (each remembered until its instance stops vouching for it).
#[derive(Debug, Default)]
pub(crate) struct Presence {
    local: HashMap<Uuid, Participant>,
    remote: HashMap<Uuid, (Participant, Instant)>,
}

impl Presence {
    pub(crate) fn add_local(&mut self, participant: Participant) {
        self.local.insert(participant.session, participant);
    }

    /// Returns whether the connection was known.
    pub(crate) fn remove_local(&mut self, session: Uuid) -> bool {
        self.local.remove(&session).is_some()
    }

    pub(crate) fn local(&self) -> Vec<Participant> {
        self.local.values().cloned().collect()
    }

    pub(crate) fn has_local(&self) -> bool {
        !self.local.is_empty()
    }

    /// Remembers participants another instance reported, as of `now`.
    pub(crate) fn see_remote(&mut self, participants: Vec<Participant>, now: Instant) -> Seen {
        let mut seen = Seen::default();
        for participant in participants {
            let session = participant.session;
            match self.remote.get(&session) {
                None => seen.fresh.push(participant.clone()),
                Some((known, _)) if known.editing != participant.editing => {
                    seen.changed.push((session, participant.editing));
                }
                Some(_) => {}
            }
            self.remote.insert(session, (participant, now));
        }
        seen
    }

    /// Records whether a connection (local or of another instance) is editing; returns whether
    /// the flag changed.
    pub(crate) fn set_editing(&mut self, session: Uuid, editing: bool) -> bool {
        let entry = self.local.get_mut(&session).or_else(|| {
            self.remote
                .get_mut(&session)
                .map(|(participant, _)| participant)
        });
        match entry {
            Some(participant) if participant.editing != editing => {
                participant.editing = editing;
                true
            }
            _ => false,
        }
    }

    /// Returns whether the connection was known.
    pub(crate) fn remote_left(&mut self, session: Uuid) -> bool {
        self.remote.remove(&session).is_some()
    }

    /// Forgets remote participants not confirmed within `ttl`; returns their sessions.
    pub(crate) fn expire_remote(&mut self, now: Instant, ttl: Duration) -> Vec<Uuid> {
        let stale: Vec<Uuid> = self
            .remote
            .iter()
            .filter(|(_, (_, seen))| now.saturating_duration_since(*seen) > ttl)
            .map(|(session, _)| *session)
            .collect();
        for session in &stale {
            self.remote.remove(session);
        }
        stale
    }

    /// Everybody except the connection `me`.
    pub(crate) fn others(&self, me: Uuid) -> Vec<Participant> {
        self.local
            .values()
            .chain(self.remote.values().map(|(participant, _)| participant))
            .filter(|participant| participant.session != me)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use identity_contracts::UserId;
    use workspace_contracts::Role;

    use super::*;

    fn person(name: &str) -> Participant {
        Participant {
            session: Uuid::now_v7(),
            user_id: UserId::from_uuid(Uuid::now_v7()),
            display_name: name.to_owned(),
            role: Role::Editor,
            editing: false,
        }
    }

    #[test]
    fn others_leave_out_the_asking_connection() {
        let (ada, grace) = (person("Ada"), person("Grace"));
        let mut presence = Presence::default();
        presence.add_local(ada.clone());
        let _ = presence.see_remote(vec![grace.clone()], Instant::now());

        assert_eq!(presence.others(ada.session), vec![grace]);
        assert!(presence.remove_local(ada.session));
        assert!(!presence.has_local());
    }

    #[test]
    fn activity_flips_only_when_it_changes() {
        let (ada, grace) = (person("Ada"), person("Grace"));
        let mut presence = Presence::default();
        presence.add_local(ada.clone());
        let _ = presence.see_remote(vec![grace.clone()], Instant::now());

        assert!(presence.set_editing(ada.session, true));
        assert!(!presence.set_editing(ada.session, true));
        assert!(presence.set_editing(grace.session, true));
        assert!(
            !presence.set_editing(Uuid::now_v7(), true),
            "unknown connection"
        );
        assert!(presence.others(Uuid::nil()).iter().all(|p| p.editing));
    }

    #[test]
    fn remote_participants_expire_unless_confirmed() {
        let mut presence = Presence::default();
        let grace = person("Grace");
        let then = Instant::now();
        assert_eq!(
            presence.see_remote(vec![grace.clone()], then).fresh.len(),
            1
        );

        let ttl = Duration::from_secs(15);
        assert!(
            presence
                .expire_remote(then + Duration::from_secs(10), ttl)
                .is_empty()
        );
        assert!(
            presence
                .see_remote(vec![grace.clone()], then + Duration::from_secs(10))
                .fresh
                .is_empty()
        );
        assert!(
            presence
                .expire_remote(then + Duration::from_secs(20), ttl)
                .is_empty()
        );
        assert_eq!(
            presence.expire_remote(then + Duration::from_secs(40), ttl),
            vec![grace.session]
        );
        assert!(presence.others(Uuid::nil()).is_empty());
    }
}
