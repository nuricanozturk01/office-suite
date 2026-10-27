use std::{
    collections::HashMap,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use documents_contracts::{AccessLevel, DocumentAccess, DocumentId, OwnerId};
use thiserror::Error;
use tokio::sync::{Mutex, broadcast};

use super::{Frame, Room, RoomEnv, room::RoomApplyError};
use crate::domain::protocol::{self, Incoming};

/// Why joining a document failed.
#[derive(Debug, Error)]
pub(crate) enum JoinError {
    /// Missing, or hidden from the caller (no existence oracle).
    #[error("document not found")]
    NotFound,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

/// Why a client frame ended the session.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum SessionError {
    /// A read-only user tried to change the document.
    #[error("write not permitted")]
    Forbidden,
    /// The frame or update was not valid.
    #[error("malformed message")]
    Malformed,
    /// Infrastructure failure.
    #[error("internal error")]
    Internal,
}

/// Per-document holder: loading a room locks only this document, never the whole map.
type Slot = Arc<Mutex<Weak<Room>>>;

/// Collaboration use cases.
pub(crate) struct CollaborationService {
    env: RoomEnv,
    recheck: Duration,
    access: Arc<dyn DocumentAccess>,
    rooms: Mutex<HashMap<DocumentId, Slot>>,
    next_session: AtomicU64,
}

impl std::fmt::Debug for CollaborationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollaborationService")
            .finish_non_exhaustive()
    }
}

impl CollaborationService {
    pub(crate) fn new(env: RoomEnv, access: Arc<dyn DocumentAccess>, recheck: Duration) -> Self {
        Self {
            env,
            recheck,
            access,
            rooms: Mutex::new(HashMap::new()),
            next_session: AtomicU64::new(1),
        }
    }

    /// Authorizes the user and attaches them to the document's room.
    pub(crate) async fn join(
        &self,
        user: OwnerId,
        document: DocumentId,
    ) -> Result<Session, JoinError> {
        let level = self.access.access_level(user, document).await;
        if level == AccessLevel::None {
            return Err(JoinError::NotFound);
        }
        let room = self.room(document).await?;
        Ok(Session {
            id: self.next_session.fetch_add(1, Ordering::Relaxed),
            user,
            document,
            level,
            access: self.access.clone(),
            recheck: self.recheck,
            room,
        })
    }

    /// Closes the sessions of `user` on `document` that are connected to this instance.
    pub(crate) async fn access_changed(&self, document: DocumentId, user: OwnerId) {
        let slot = self.rooms.lock().await.get(&document).cloned();
        // A slot that is locked is being (re)loaded: its sessions are not attached yet.
        let room = slot.and_then(|slot| slot.try_lock().ok().and_then(|weak| weak.upgrade()));
        if let Some(room) = room {
            room.access_changed(user);
        }
    }

    /// Returns the live room for a document, loading it once even under concurrent joins.
    async fn room(&self, document: DocumentId) -> Result<Arc<Room>, JoinError> {
        let slot = {
            let mut rooms = self.rooms.lock().await;
            // Drop entries nobody uses anymore (the map itself holds one reference).
            rooms.retain(|_, slot| {
                Arc::strong_count(slot) > 1 || slot.try_lock().is_ok_and(|w| w.strong_count() > 0)
            });
            rooms.entry(document).or_default().clone()
        };
        let mut weak = slot.lock().await;
        if let Some(room) = weak.upgrade()
            && !room.is_closed()
        {
            return Ok(room);
        }
        let room = Room::load(document, self.env.clone()).await.map_err(|_| {
            tracing::error!(%document, "failed to load document state");
            JoinError::Internal
        })?;
        *weak = Arc::downgrade(&room);
        drop(weak);
        Ok(room)
    }
}

/// One authorized connection to a document.
#[derive(Debug)]
pub(crate) struct Session {
    pub(crate) id: u64,
    user: OwnerId,
    document: DocumentId,
    level: AccessLevel,
    access: Arc<dyn DocumentAccess>,
    recheck: Duration,
    room: Arc<Room>,
}

impl Session {
    pub(crate) const fn user(&self) -> OwnerId {
        self.user
    }

    pub(crate) const fn recheck_interval(&self) -> Duration {
        self.recheck
    }

    /// Whether the user now has less access than when they joined (defence in depth for a
    /// lost revoke event).
    pub(crate) async fn access_lost(&self) -> bool {
        self.access.access_level(self.user, self.document).await < self.level
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Frame> {
        self.room.subscribe()
    }

    /// First frame to send: the server's state vector.
    pub(crate) async fn hello(&self) -> Vec<u8> {
        self.room.hello().await
    }

    /// Handles a client frame and returns frames to send back to this client only.
    pub(crate) async fn handle(&self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, SessionError> {
        match protocol::parse(bytes).map_err(|_| SessionError::Malformed)? {
            Incoming::SyncStep1(remote) => Ok(vec![self.room.diff_for(&remote).await]),
            Incoming::Update(update) => {
                if self.level < AccessLevel::Write {
                    return Err(SessionError::Forbidden);
                }
                self.room
                    .apply(self.id, update)
                    .await
                    .map_err(|err| match err {
                        RoomApplyError::Malformed => SessionError::Malformed,
                        RoomApplyError::Storage => SessionError::Internal,
                    })?;
                Ok(Vec::new())
            }
            Incoming::Awareness => {
                self.room.relay(self.id, bytes).await;
                Ok(Vec::new())
            }
            Incoming::Ignored => Ok(Vec::new()),
        }
    }
}
