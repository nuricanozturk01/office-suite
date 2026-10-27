use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use identity_contracts::{UserId, UserProfiles};
use livesync_contracts::{Participant, ServerMessage};
use shared_kernel::Uuid;
use time::OffsetDateTime;
use tokio::sync::{Mutex, broadcast};
use workspace_contracts::{PermissionChecker, ResourceId, Role};

use super::{BusEvent, BusMessage, BusStream, LiveBus};
use crate::domain::{Presence, Seen};

/// Events a room delivers to its local connections.
const EVENT_CAPACITY: usize = 64;
/// How often an instance repeats who is connected to it.
const HEARTBEAT: Duration = Duration::from_secs(5);
/// How long another instance's participants stay without a heartbeat.
const REMOTE_TTL: Duration = Duration::from_secs(15);

/// Why a connection was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JoinError {
    /// The resource does not exist or the user may not read it (the same answer for both).
    NotFound,
}

/// What happens in a room.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    /// Something clients should hear.
    Message(ServerMessage),
    /// A user's access to the resource shrank; their connections must re-check.
    AccessChanged { user: UserId },
}

struct Room {
    events: broadcast::Sender<Event>,
    presence: Mutex<Presence>,
}

/// Live rooms, one per resource that has at least one connection on this instance.
pub(crate) struct LiveSyncService {
    rooms: Mutex<HashMap<ResourceId, Arc<Room>>>,
    checker: Arc<dyn PermissionChecker>,
    profiles: Arc<dyn UserProfiles>,
    bus: Arc<dyn LiveBus>,
    instance: Uuid,
    recheck: Duration,
}

impl std::fmt::Debug for LiveSyncService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveSyncService").finish_non_exhaustive()
    }
}

impl LiveSyncService {
    pub(crate) fn new(
        checker: Arc<dyn PermissionChecker>,
        profiles: Arc<dyn UserProfiles>,
        bus: Arc<dyn LiveBus>,
        recheck: Duration,
    ) -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
            checker,
            profiles,
            bus,
            instance: Uuid::now_v7(),
            recheck,
        }
    }

    /// Opens a connection for `user` to `resource`.
    pub(crate) async fn join(
        self: &Arc<Self>,
        user: UserId,
        resource: ResourceId,
    ) -> Result<Session, JoinError> {
        let Some(role) = self
            .checker
            .role(user, resource)
            .await
            .filter(|role| role.can_read())
        else {
            return Err(JoinError::NotFound);
        };
        let display_name = self
            .profiles
            .display_name(user)
            .await
            .unwrap_or_else(|| "Someone".to_owned());
        let participant = Participant {
            session: Uuid::now_v7(),
            user_id: user,
            display_name,
            role,
            editing: false,
        };
        let room = self.enter(resource, &participant).await;
        let events = room.events.subscribe();
        let others = room.presence.lock().await.others(participant.session);
        let _ = room.events.send(Event::Message(ServerMessage::Joined {
            participant: participant.clone(),
        }));
        self.publish(
            resource,
            BusEvent::Joined {
                participant: participant.clone(),
            },
        )
        .await;
        Ok(Session {
            service: self.clone(),
            room,
            resource,
            participant,
            events,
            welcome: others,
        })
    }

    /// The room of `resource` (created when it is the first connection here) with `participant`
    /// added. Both happen under the room list's lock, so a room that is closing down cannot take
    /// the newcomer with it.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the list stays locked while the newcomer is added, so a closing room cannot miss it"
    )]
    async fn enter(self: &Arc<Self>, resource: ResourceId, participant: &Participant) -> Arc<Room> {
        let mut rooms = self.rooms.lock().await;
        let room = rooms
            .entry(resource)
            .or_insert_with(|| {
                let room = Arc::new(Room {
                    events: broadcast::channel(EVENT_CAPACITY).0,
                    presence: Mutex::new(Presence::default()),
                });
                tokio::spawn(run_room(self.clone(), resource, room.clone()));
                room
            })
            .clone();
        room.presence.lock().await.add_local(participant.clone());
        room
    }

    /// Tells the connections to `resource` on this instance that a new version was saved.
    pub(crate) async fn saved(
        &self,
        resource: ResourceId,
        version: i64,
        by: Option<UserId>,
        at: OffsetDateTime,
    ) {
        if let Some(room) = self.rooms.lock().await.get(&resource) {
            let _ = room
                .events
                .send(Event::Message(ServerMessage::Saved { version, by, at }));
        }
    }

    /// Makes the connections of `user` to `resource` check their access again.
    pub(crate) async fn access_changed(&self, resource: ResourceId, user: UserId) {
        if let Some(room) = self.rooms.lock().await.get(&resource) {
            let _ = room.events.send(Event::AccessChanged { user });
        }
    }

    /// Removes the room when nobody is connected here any more; true when it was removed.
    async fn close_if_empty(&self, resource: ResourceId, room: &Room) -> bool {
        let mut rooms = self.rooms.lock().await;
        let empty = !room.presence.lock().await.has_local();
        if empty {
            rooms.remove(&resource);
        }
        empty
    }

    async fn may_read(&self, user: UserId, resource: ResourceId) -> bool {
        self.checker
            .role(user, resource)
            .await
            .is_some_and(Role::can_read)
    }

    async fn publish(&self, resource: ResourceId, event: BusEvent) {
        self.bus
            .publish(
                resource,
                BusMessage {
                    instance: self.instance,
                    event,
                },
            )
            .await;
    }
}

/// One open connection.
pub(crate) struct Session {
    service: Arc<LiveSyncService>,
    room: Arc<Room>,
    resource: ResourceId,
    participant: Participant,
    events: broadcast::Receiver<Event>,
    welcome: Vec<Participant>,
}

impl Session {
    pub(crate) const fn id(&self) -> Uuid {
        self.participant.session
    }

    pub(crate) const fn user(&self) -> UserId {
        self.participant.user_id
    }

    /// The first message of the connection.
    pub(crate) fn welcome(&self) -> ServerMessage {
        ServerMessage::Welcome {
            session: self.participant.session,
            participants: self.welcome.clone(),
        }
    }

    pub(crate) fn recheck_interval(&self) -> Duration {
        self.service.recheck
    }

    /// Next room event, in order.
    pub(crate) async fn next_event(&mut self) -> Result<Event, broadcast::error::RecvError> {
        self.events.recv().await
    }

    /// Whether the user can no longer read the resource.
    pub(crate) async fn access_lost(&self) -> bool {
        !self
            .service
            .may_read(self.participant.user_id, self.resource)
            .await
    }

    /// Tells everybody whether this connection is changing the file right now.
    pub(crate) async fn set_editing(&self, editing: bool) {
        if editing && !self.participant.role.can_write() {
            return; // viewers and commenters do not change the file
        }
        let session = self.participant.session;
        if !self
            .room
            .presence
            .lock()
            .await
            .set_editing(session, editing)
        {
            return;
        }
        let _ = self
            .room
            .events
            .send(Event::Message(ServerMessage::Activity { session, editing }));
        self.service
            .publish(self.resource, BusEvent::Activity { session, editing })
            .await;
    }

    /// Closes the connection: tells everybody else.
    pub(crate) async fn leave(self) {
        let session = self.participant.session;
        self.room.presence.lock().await.remove_local(session);
        let _ = self
            .room
            .events
            .send(Event::Message(ServerMessage::Left { session }));
        self.service
            .publish(self.resource, BusEvent::Left { session })
            .await;
    }
}

async fn next_bus(stream: &mut Option<Box<dyn BusStream>>) -> Option<BusMessage> {
    match stream {
        Some(stream) => stream.next().await,
        None => std::future::pending().await,
    }
}

/// Keeps one room in step with the other instances until nobody is connected here any more.
async fn run_room(service: Arc<LiveSyncService>, resource: ResourceId, room: Arc<Room>) {
    let mut stream = service.bus.subscribe(resource).await.ok();
    if stream.is_none() {
        tracing::warn!(%resource, "live bus unavailable; presence is local to this instance");
    }
    let mut heartbeat = tokio::time::interval(HEARTBEAT);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await; // the first tick completes immediately
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if service.close_if_empty(resource, &room).await {
                    return;
                }
                let (local, stale) = {
                    let mut presence = room.presence.lock().await;
                    (presence.local(), presence.expire_remote(Instant::now(), REMOTE_TTL))
                };
                for session in stale {
                    let _ = room.events.send(Event::Message(ServerMessage::Left { session }));
                }
                service.publish(resource, BusEvent::Presence { participants: local }).await;
            }
            message = next_bus(&mut stream) => match message {
                Some(message) => handle_bus(&service, resource, &room, message).await,
                None => stream = None,
            },
        }
    }
}

async fn handle_bus(
    service: &LiveSyncService,
    resource: ResourceId,
    room: &Room,
    message: BusMessage,
) {
    if message.instance == service.instance {
        return;
    }
    match message.event {
        BusEvent::Joined { participant } => {
            let (seen, local) = {
                let mut presence = room.presence.lock().await;
                let seen = presence.see_remote(vec![participant], Instant::now());
                (seen, presence.local())
            };
            announce(room, seen);
            // The newcomer's instance may not know who is here yet.
            service
                .publish(
                    resource,
                    BusEvent::Presence {
                        participants: local,
                    },
                )
                .await;
        }
        BusEvent::Left { session } => {
            if room.presence.lock().await.remote_left(session) {
                let _ = room
                    .events
                    .send(Event::Message(ServerMessage::Left { session }));
            }
        }
        BusEvent::Presence { participants } => {
            let seen = room
                .presence
                .lock()
                .await
                .see_remote(participants, Instant::now());
            announce(room, seen);
        }
        BusEvent::Activity { session, editing } => {
            if room.presence.lock().await.set_editing(session, editing) {
                let _ = room
                    .events
                    .send(Event::Message(ServerMessage::Activity { session, editing }));
            }
        }
    }
}

/// Tells the local connections what a report from another instance added or changed.
fn announce(room: &Room, seen: Seen) {
    for participant in seen.fresh {
        let _ = room
            .events
            .send(Event::Message(ServerMessage::Joined { participant }));
    }
    for (session, editing) in seen.changed {
        let _ = room
            .events
            .send(Event::Message(ServerMessage::Activity { session, editing }));
    }
}
