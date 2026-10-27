# Collaboration: Multi-Instance Sync and Access Revocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Several API instances can serve the same document without losing or diverging edits, and revoking or downgrading a user's access closes their open WebSocket sessions on every instance.

**Architecture:** Postgres stays the source of truth (append-only `document_updates`). Rooms are per-instance caches: each instance publishes appended updates and awareness frames to a per-document Redis channel and applies the ones it receives; a periodic DB catch-up per room heals lost Redis messages. Compaction is rebuilt from the durable log under an advisory lock, never from a room's memory. A broadcast handler turns `PermissionRevoked` into a room-level `AccessChanged` frame, and sessions also re-check access periodically.

**Tech Stack:** Rust 1.96, axum WebSocket, yrs 0.28, redis 1.8 (`platform::docbus`), sqlx 0.9, testcontainers (Postgres + Redis), tokio-tungstenite (existing sync tests).

**Spec:** `docs/superpowers/specs/2026-10-25-sharing-foundation-design.md` §2.4, §5 and parent spec §3a.

## Global Constraints

- CLAUDE.md applies in full (contracts boundaries, no `unwrap` in production code, `tracing` without document content/PII, layering, `just check` green before each commit, Conventional Commits with scope `collaboration` or `platform`).
- A document must never lose an update because of compaction, a second instance, or Redis loss.
- Redis messages are an optimization; correctness never depends on them (DB catch-up interval default 5 s, injectable for tests).
- Access recheck interval default 30 s, injectable for tests. Close code for access loss: 1008 (`CLOSE_POLICY`), reason `access changed`.
- Updates are limited to 1 MiB per frame (existing); Redis payloads carry the raw frame plus a 17-byte header (16-byte instance id, 1-byte kind).
- No per-document state on an instance may be authoritative; a room dropped and reloaded must equal the log.

## Review Focus

- Instance A compacts while instance B has appended newer rows → B's rows survive and the next load equals the union.
- A Redis message lost between instances → the DB catch-up delivers the missing update within the catch-up interval.
- The same update received via Redis and via DB catch-up → applied once effectively (idempotent), broadcast to local sessions at most once.
- Own Redis echo is ignored (instance id check), so a single instance never double-broadcasts.
- A user downgraded from editor to viewer while connected can no longer write (session closed on revoke event; recheck closes it even if the event is lost).
- A revoke for user X must not disconnect user Y on the same document.
- Redis unavailable at publish time → the editing session continues (error logged), peers on other instances converge through the catch-up.

---

### Task 1: `platform::docbus` — raw per-channel pub/sub

**Files:**
- Create: `crates/platform/src/docbus.rs`, `crates/platform/tests/docbus.rs`
- Modify: `crates/platform/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct DocBus` with `pub async fn connect(url: &SecretString) -> Result<Arc<Self>, BroadcastError>`, `pub async fn publish(&self, channel: &str, payload: Vec<u8>) -> Result<(), BroadcastError>`, `pub async fn subscribe(self: &Arc<Self>, channel: &str) -> Result<Subscription, BroadcastError>`.
  - `pub struct Subscription { /* receiver + guard */ }` with `pub async fn recv(&mut self) -> Result<Arc<[u8]>, Lagged>`-style API (document the exact error enum: `Lagged(u64)`, `Closed`); dropping the last `Subscription` of a channel unsubscribes it from Redis.
- Behaviour: one Redis PubSub connection split into sink/stream; a reader task routes messages by channel name to in-process `tokio::sync::broadcast` senders (capacity 256); on connection loss it reconnects with capped backoff (200 ms → 5 s) and re-subscribes every live channel; messages during the gap are lost (documented).

- [ ] **Step 1: Write failing tests** (Redis container): `message_reaches_all_local_subscribers_of_a_channel`, `two_buses_exchange_messages` (separate `DocBus::connect` = two instances), `other_channels_are_not_delivered`, `dropping_last_subscription_unsubscribes` (assert via `PUBSUB NUMSUB` that the count is 0), `resubscribes_after_connection_loss` (kill the client connection with `CLIENT KILL TYPE pubsub`, publish again within 5 s, expect delivery).
- [ ] **Step 2:** `cargo test -p platform --test docbus` — Expected: FAIL (compile).
- [ ] **Step 3:** Implement per the interface (use the redis crate's PubSub split; no unbounded buffers).
- [ ] **Step 4:** `cargo test -p platform --test docbus` ×3 — Expected: PASS each time; `cargo clippy -p platform --all-targets -- -D warnings`.
- [ ] **Step 5: Commit** `feat(platform): add per-channel redis doc bus`

### Task 2: Compaction rebuilt from the log (fixes a data-loss bug)

**Files:**
- Modify: `modules/collaboration/core/src/application/{ports.rs,room.rs}`, `modules/collaboration/core/src/infrastructure/store.rs`, `.sqlx` via `just db-prepare`
- Test: `modules/collaboration/core/tests/multi_instance.rs` (new)

**Interfaces:**
- `UpdateStore::compact(&self, document) -> Result<(), StoreError>` loses its `state`/`upto_seq` arguments: the store takes `pg_try_advisory_xact_lock(<collab namespace const>, hashtext(document))` (returns `Ok(())` without work when another instance holds it), then in ONE transaction loads snapshot + all rows, folds them with yrs (`SyncState` from the domain, passed in as a closure or done by the application layer via `load` + `replace_snapshot(document, state, upto_seq)` inside the same lock — choose the variant that keeps yrs out of the infrastructure layer), upserts the snapshot only if `upto_seq` grows, and deletes rows `seq <= upto_seq`.
- `Room` triggers compaction after `COMPACT_AFTER` own appends but never passes its in-memory state.

- [ ] **Step 1: Write the failing test** `compaction_by_one_instance_keeps_updates_of_the_other`: two `Room`s over one Postgres (no Redis, no catch-up), each applies different edits, room A is forced to compact (call the room's compaction directly or lower `COMPACT_AFTER` via a constructor parameter), then a freshly loaded room C contains BOTH rooms' edits. (Before the fix this fails: A's snapshot lacks B's rows yet deletes them.)
- [ ] **Step 2:** `cargo test -p collaboration --test multi_instance` — Expected: FAIL with missing edits.
- [ ] **Step 3:** Implement the log-based compaction as specified; make `COMPACT_AFTER` a `Room::load` parameter with the old value as default constant.
- [ ] **Step 4:** `cargo test -p collaboration` — Expected: PASS (existing `sync_flow` tests included).
- [ ] **Step 5: Commit** `fix(collaboration): compact from the durable log under an advisory lock`

### Task 3: Cross-instance fan-out and DB catch-up

**Files:**
- Create: `modules/collaboration/core/src/infrastructure/bus.rs`
- Modify: `modules/collaboration/core/src/application/{ports.rs,room.rs,service.rs,mod.rs}`, `modules/collaboration/core/src/lib.rs`, `modules/collaboration/core/Cargo.toml`, `apps/api/src/main.rs`
- Test: `modules/collaboration/core/tests/multi_instance.rs`

**Interfaces:**
- New port in `application/ports.rs`:
  `#[async_trait] pub(crate) trait UpdateBus: Send + Sync { async fn publish(&self, document: DocumentId, message: BusMessage); async fn subscribe(&self, document: DocumentId) -> Result<BusStream, StoreError>; }` with `BusMessage { instance: Uuid, kind: BusKind /* Update{seq} | Awareness */, bytes: Arc<[u8]> }` and a `BusStream` yielding `BusMessage`.
- `NoBus` (no-op, single instance/tests) and `RedisBus` (over `platform::docbus`, channel `collab.<document_id>`, binary header per Global Constraints).
- `UpdateStore` gains `async fn updates_after(&self, document, after_seq: i64) -> Result<Vec<(i64, Vec<u8>)>, StoreError>` (served by the existing `(document_id, seq)` index).
- `Room`: after a successful local `append`, `bus.publish(Update{seq})`; awareness frames are also published. A per-room task (holds only a `Weak<Room>`, ends when the room is dropped) consumes `BusStream` and `updates_after(last_seq)` every `catch_up_interval` (constructor parameter, default 5 s); remote updates are applied to the `SyncState` and fanned out to local sessions as `Frame::Data` with a fresh non-session origin; messages whose `instance` equals the own id are ignored; `last_seq` becomes `max(last_seq, seq)`.
- `collaboration::module(pool, access, bus: Arc<dyn UpdateBus>)` plus `pub fn redis_bus(url) -> Result<Arc<dyn UpdateBus>, …>` helper for the composition root.

- [ ] **Step 1: Write failing tests** (Postgres + Redis containers, two `CollaborationService` instances): `update_on_a_reaches_session_on_b_via_redis`, `awareness_on_a_reaches_b`, `catch_up_delivers_update_when_redis_message_is_lost` (instance B uses `NoBus`, interval 100 ms; update applied via A's store; B's session receives it within 2 s), `own_echo_is_not_rebroadcast` (single instance with RedisBus: exactly one frame to a peer session), `duplicate_delivery_is_applied_once` (same update through bus and catch-up → peer session receives it once; state equals).
- [ ] **Step 2:** `cargo test -p collaboration --test multi_instance` — Expected: FAIL.
- [ ] **Step 3:** Implement the port, both buses, the per-room task and wiring. `apps/api` builds `RedisBus` from `settings.redis_url` (fail fast if unreachable).
- [ ] **Step 4:** `just db-prepare && cargo test -p collaboration` ×3 — Expected: PASS each time.
- [ ] **Step 5: Commit** `feat(collaboration): fan out updates across instances`

### Task 4: Access revocation and periodic recheck

**Files:**
- Create: `modules/collaboration/core/src/api/events.rs`
- Modify: `modules/collaboration/core/src/application/{room.rs,service.rs}`, `modules/collaboration/core/src/api/{mod.rs,ws.rs}`, `modules/collaboration/core/src/lib.rs` (+ `workspace-contracts` dependency), `apps/api/src/main.rs`
- Test: `modules/collaboration/core/tests/access_changes.rs`

**Interfaces:**
- `Frame::AccessChanged { user: OwnerId }` broadcast to a room's sessions; `Session` knows its `user`; `ws.rs` closes a session with 1008 `access changed` when the frame's user equals its own, and also when its periodic `recheck()` (interval param, default 30 s) finds `access.access_level(user, document) < session level at join`.
- `CollaborationService::access_changed(document, user)` finds the live room (if any) and sends `AccessChanged`; no room → no-op.
- `Module` gains `broadcast_handlers: Vec<Arc<dyn platform::broadcast::BroadcastHandler>>` containing a handler for event type `PermissionRevoked` (decodes `workspace_contracts::PermissionRevoked`, maps ids, calls `access_changed`). `apps/api` passes these to `EventRuntimeConfig.broadcast_handlers`.

- [ ] **Step 1: Write failing tests:** `revoke_event_closes_only_the_affected_users_session` (two sessions on one room, handler invoked with an envelope for user X → X's receiver gets the close signal, Y's does not), `recheck_closes_session_after_downgrade_even_without_event` (fake `DocumentAccess` that flips from Write to Read; interval 50 ms), `revoke_for_unloaded_room_is_a_noop`, and an end-to-end test with a real WebSocket (tokio-tungstenite like `sync_flow.rs`): editor connects, access flips to none, handler fires, client observes a close frame with code 1008.
- [ ] **Step 2:** `cargo test -p collaboration --test access_changes` — Expected: FAIL.
- [ ] **Step 3:** Implement frames, session handling, recheck timer in the `ws.rs` select loop, the event handler, and composition wiring.
- [ ] **Step 4:** `just check` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(collaboration): drop sessions when access changes`

### Task 5: Two-instance end-to-end and docs

**Files:**
- Create: `modules/collaboration/core/tests/two_instances_ws.rs`
- Modify: `docs/STATUS.md`

**Interfaces:** consumes everything above; no new production API.

- [ ] **Step 1: Write the test:** start two real axum servers (random ports) over one Postgres and one Redis, each with its own `collaboration::module`; client 1 connects to server A and client 2 to server B for the same document; client 1 sends a Yjs update frame; client 2 receives it (≤ 2 s) and a late third client on A that joins afterwards gets the state via sync step 2; kill nothing; assert final `document_updates` rows decode to the union.
- [ ] **Step 2:** `cargo test -p collaboration --test two_instances_ws` — Expected: PASS (all features exist by now; if it fails the failure is a real bug in Tasks 2–4: fix there, not by weakening the test).
- [ ] **Step 3:** Update `docs/STATUS.md` (multi-instance collaboration done; remaining gaps: awareness is not persisted, Redis gap relies on catch-up, frontend reconnect behaviour on 1008).
- [ ] **Step 4:** `just check` — Expected: PASS.
- [ ] **Step 5: Commit** `test(collaboration): cover two instances end to end`
