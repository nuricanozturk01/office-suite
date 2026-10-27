use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use documents_contracts::{DocumentId, OwnerId};
use shared_kernel::Uuid;
use tokio::sync::{Mutex, broadcast};
use yrs::StateVector;

use super::{BusKind, BusMessage, BusStream, StoreError, UpdateBus, UpdateStore};
use crate::domain::{SyncError, SyncState, protocol};

/// Fold the log into a snapshot after this many appended updates.
const COMPACT_AFTER: usize = 200;
const CHANNEL_CAPACITY: usize = 256;
/// Origin of frames that came from another instance (session ids start at 1).
const REMOTE_ORIGIN: u64 = 0;
/// Sequence numbers are assigned before commit, so a catch-up re-reads a window below its cursor.
const CATCH_UP_OVERLAP: i64 = 1_000;
/// How many applied sequence numbers a room remembers to skip duplicate deliveries.
const SEEN_CAPACITY: usize = 4_096;

/// Message fanned out to every connection in a room.
#[derive(Debug, Clone)]
pub(crate) enum Frame {
    /// A frame for peers other than `origin`.
    Data { origin: u64, bytes: Arc<[u8]> },
    /// The room is no longer reliable; connections must drop and resync.
    Closed,
    /// `user`'s access to the document changed; their sessions must be closed.
    AccessChanged { user: OwnerId },
}

/// Everything a room needs from the outside world.
#[derive(Clone)]
pub(crate) struct RoomEnv {
    pub(crate) store: Arc<dyn UpdateStore>,
    pub(crate) bus: Arc<dyn UpdateBus>,
    /// Identity of this instance on the bus.
    pub(crate) instance: Uuid,
    /// How often the room re-reads the log for updates the bus did not deliver.
    pub(crate) catch_up: Duration,
}

/// Bounded memory of applied sequence numbers.
#[derive(Debug, Default)]
struct RecentSeqs {
    order: VecDeque<i64>,
    set: HashSet<i64>,
}

impl RecentSeqs {
    /// Returns `true` when `seq` was not seen before.
    fn insert(&mut self, seq: i64) -> bool {
        if !self.set.insert(seq) {
            return false;
        }
        self.order.push_back(seq);
        if self.order.len() > SEEN_CAPACITY
            && let Some(old) = self.order.pop_front()
        {
            self.set.remove(&old);
        }
        true
    }
}

#[derive(Debug)]
struct State {
    sync: SyncState,
    /// Highest sequence number returned by a load or catch-up (not by local appends).
    cursor: i64,
    pending: usize,
    seen: RecentSeqs,
}

/// In-memory replica of one document shared by all its connections on this instance. A cache:
/// the durable log is the truth, other instances are reached through the bus and the catch-up.
pub(crate) struct Room {
    document: DocumentId,
    env: RoomEnv,
    state: Mutex<State>,
    tx: broadcast::Sender<Frame>,
    closed: AtomicBool,
}

impl std::fmt::Debug for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Room")
            .field("document", &self.document)
            .finish_non_exhaustive()
    }
}

/// Why an update was not accepted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RoomApplyError {
    Malformed,
    Storage,
}

impl Room {
    /// Rebuilds a room from durable state and starts following other instances.
    pub(crate) async fn load(document: DocumentId, env: RoomEnv) -> Result<Arc<Self>, StoreError> {
        // Subscribe first so nothing published while loading is missed.
        let stream = env.bus.subscribe(document).await.unwrap_or_else(|_| {
            // Redis down: this room runs on the database catch-up alone.
            tracing::warn!(%document, "bus unavailable; relying on catch-up");
            Box::new(Silent)
        });
        let loaded = env.store.load(document).await?;
        let sync = SyncState::new();
        if let Some(snapshot) = &loaded.snapshot {
            sync.apply_update(snapshot)
                .map_err(|_: SyncError| StoreError)?;
        }
        let mut seen = RecentSeqs::default();
        let mut cursor = loaded.snapshot_seq;
        for (seq, payload) in &loaded.updates {
            sync.apply_update(payload)
                .map_err(|_: SyncError| StoreError)?;
            seen.insert(*seq);
            cursor = *seq;
        }
        let pending = loaded.updates.len();
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        let catch_up = env.catch_up;
        let room = Arc::new(Self {
            document,
            env,
            state: Mutex::new(State {
                sync,
                cursor,
                pending,
                seen,
            }),
            tx,
            closed: AtomicBool::new(false),
        });
        tokio::spawn(follow_others(Arc::downgrade(&room), stream, catch_up));
        Ok(room)
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Frame> {
        self.tx.subscribe()
    }

    /// Tells the sessions of `user` on this instance that their access changed.
    pub(crate) fn access_changed(&self, user: OwnerId) {
        let _ = self.tx.send(Frame::AccessChanged { user });
    }

    /// Frame announcing the server's state vector.
    pub(crate) async fn hello(&self) -> Vec<u8> {
        protocol::step1(self.state.lock().await.sync.state_vector())
    }

    /// Frame carrying what a peer with `remote` is missing.
    pub(crate) async fn diff_for(&self, remote: &StateVector) -> Vec<u8> {
        protocol::step2(self.state.lock().await.sync.diff_since(remote))
    }

    /// Validates, persists and fans out an update. Updates are serialized per document on this
    /// instance, so local log order equals local application order.
    pub(crate) async fn apply(&self, origin: u64, update: Vec<u8>) -> Result<(), RoomApplyError> {
        if self.is_closed() {
            return Err(RoomApplyError::Storage);
        }
        let mut state = self.state.lock().await;
        state
            .sync
            .apply_update(&update)
            .map_err(|_| RoomApplyError::Malformed)?;
        let Ok(seq) = self.env.store.append(self.document, &update).await else {
            // Memory is now ahead of the log: retire the room so peers resync from storage.
            self.closed.store(true, Ordering::Release);
            let _ = self.tx.send(Frame::Closed);
            return Err(RoomApplyError::Storage);
        };
        state.seen.insert(seq);
        state.pending += 1;
        // Sending inside the lock keeps broadcast order equal to log order.
        let _ = self.tx.send(Frame::Data {
            origin,
            bytes: protocol::update(update.clone()).into(),
        });

        let compact = state.pending >= COMPACT_AFTER;
        if compact {
            // Retry after another half batch if it fails, instead of on every edit.
            state.pending = 0;
        }
        drop(state);
        self.env
            .bus
            .publish(
                self.document,
                BusMessage {
                    instance: self.env.instance,
                    kind: BusKind::Update { seq },
                    bytes: update.into(),
                },
            )
            .await;
        if compact {
            self.compact_in_background();
        }
        Ok(())
    }

    /// Folds the durable log without holding the room lock (derived from the log, not from this
    /// room's memory: other instances may have appended rows this room has not seen).
    fn compact_in_background(&self) {
        let (store, document) = (self.env.store.clone(), self.document);
        tokio::spawn(async move {
            if store.compact(document).await.is_err() {
                tracing::warn!(%document, "compaction failed");
            }
        });
    }

    /// Relays a raw frame (awareness) to the other peers, here and on other instances.
    pub(crate) async fn relay(&self, origin: u64, bytes: &[u8]) {
        let _ = self.tx.send(Frame::Data {
            origin,
            bytes: bytes.into(),
        });
        self.env
            .bus
            .publish(
                self.document,
                BusMessage {
                    instance: self.env.instance,
                    kind: BusKind::Awareness,
                    bytes: bytes.into(),
                },
            )
            .await;
    }

    /// Handles a message from another instance.
    async fn apply_remote(&self, message: BusMessage) {
        if message.instance == self.env.instance {
            return;
        }
        match message.kind {
            BusKind::Awareness => {
                let _ = self.tx.send(Frame::Data {
                    origin: REMOTE_ORIGIN,
                    bytes: message.bytes,
                });
            }
            BusKind::Update { seq } => self.apply_logged(seq, &message.bytes).await,
        }
    }

    /// Applies an update that is already in the durable log, at most once per sequence number.
    async fn apply_logged(&self, seq: i64, update: &[u8]) {
        let mut state = self.state.lock().await;
        if !state.seen.insert(seq) {
            return;
        }
        if state.sync.apply_update(update).is_err() {
            tracing::warn!(document = %self.document, seq, "ignoring malformed logged update");
            return;
        }
        // Sent under the lock like local updates, so broadcast order follows apply order.
        let _ = self.tx.send(Frame::Data {
            origin: REMOTE_ORIGIN,
            bytes: protocol::update(update.to_vec()).into(),
        });
        drop(state);
    }

    /// Merges a snapshot (idempotent for what the room already has) and moves the cursor past it.
    async fn apply_snapshot(&self, upto: i64, snapshot: &[u8]) {
        let mut state = self.state.lock().await;
        if state.sync.apply_update(snapshot).is_err() {
            tracing::warn!(document = %self.document, "ignoring malformed snapshot");
            return;
        }
        state.cursor = state.cursor.max(upto);
        let _ = self.tx.send(Frame::Data {
            origin: REMOTE_ORIGIN,
            bytes: protocol::update(snapshot.to_vec()).into(),
        });
        drop(state);
    }

    /// Applies log rows this room has not seen (e.g. because a bus message was lost).
    async fn catch_up(&self) {
        let cursor = self.state.lock().await.cursor;
        // Rows up to the snapshot may have been folded away by another instance's compaction.
        if let Ok(Some((upto, state))) = self.env.store.snapshot_after(self.document, cursor).await
        {
            self.apply_snapshot(upto, &state).await;
        }
        let after = self
            .state
            .lock()
            .await
            .cursor
            .saturating_sub(CATCH_UP_OVERLAP);
        let Ok(rows) = self.env.store.updates_after(self.document, after).await else {
            tracing::warn!(document = %self.document, "catch-up read failed");
            return;
        };
        let newest = rows.last().map(|(seq, _)| *seq);
        for (seq, payload) in &rows {
            self.apply_logged(*seq, payload).await;
        }
        if let Some(newest) = newest {
            let mut state = self.state.lock().await;
            state.cursor = state.cursor.max(newest);
        }
    }
}

struct Silent;

#[async_trait::async_trait]
impl BusStream for Silent {
    async fn next(&mut self) -> Option<BusMessage> {
        std::future::pending().await
    }
}

/// Follows other instances until the room is dropped: bus messages for latency, periodic log
/// reads for correctness.
async fn follow_others(room: Weak<Room>, mut stream: Box<dyn BusStream>, every: Duration) {
    let mut ticker = tokio::time::interval(every);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await; // the first tick completes immediately
    loop {
        tokio::select! {
            message = stream.next() => {
                let Some(room) = room.upgrade() else { return };
                match message {
                    Some(message) => room.apply_remote(message).await,
                    // Bus gone: keep going on the catch-up alone.
                    None => stream = Box::new(Silent),
                }
            }
            _ = ticker.tick() => {
                let Some(room) = room.upgrade() else { return };
                if room.is_closed() {
                    return;
                }
                room.catch_up().await;
            }
        }
    }
}
