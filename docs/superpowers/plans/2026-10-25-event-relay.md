# Event Bus, Outbox Relay and Redis Broadcast Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver outbox events to subscribers exactly-once-in-effect across N API instances, and broadcast selected events to every instance via Redis.

**Architecture:** `platform` gains `event_bus` (envelope + handler traits), `outbox` (idempotency claim + `Relay` polling `<schema>.outbox` with `FOR UPDATE SKIP LOCKED`; one event per transaction) and `broadcast` (Redis pub/sub publisher + listener). Durable handlers run in their own transaction with a `platform.processed_events` claim; the relay's row lock only guards publishing, so no cross-module transaction exists (CLAUDE.md §4.2.4). `apps/api` spawns relay + listener.

**Tech Stack:** Rust 1.96, sqlx 0.9, `redis` 1.8.0 (stable; 2.x is RC only), `async-trait`, tokio, testcontainers-modules (postgres, redis).

**Spec:** `docs/superpowers/specs/2026-10-25-sharing-foundation-design.md` §2.1, §3, §5 and parent spec §3a.

## Global Constraints

- English code/commits; Conventional Commits with scope `platform` or `api`; `just check` green before each commit; no `unwrap`/`expect`/`println!` in non-test code; `#![forbid(unsafe_code)]`.
- Dependencies added through `[workspace.dependencies]` with versions verified on crates.io (redis `1.8.0`).
- Relay is at-least-once; handlers must be idempotent via `claim`. Redis broadcast is best-effort and never the source of truth.
- Multi-instance: no in-memory authoritative state; two relays on one DB must never produce a duplicate effect.
- Poll interval default 500 ms; batch = 1 event per transaction (poison events are logged, retried; dead-lettering is out of scope and noted in STATUS).

## Review Focus

- Handler fails → event stays unpublished and is retried next tick (no loss).
- Crash between handler success and `published_at` update → redelivery does not repeat the effect (`claim` returns false).
- Two relays ticking concurrently over the same outbox → each event handled once.
- Redis down at publish time → relay still marks the event published only if broadcast is not required to succeed (best-effort), and logs.
- Invalid schema identifier passed to `OutboxTable::new` → error, never reaches SQL.

---

### Task 1: Event envelope, handler traits, registry

**Files:**
- Create: `crates/platform/src/event_bus.rs`
- Modify: `crates/platform/src/lib.rs`, `crates/platform/Cargo.toml` (`async-trait`, `time`, `tracing` workspace deps), `Cargo.toml` if a dep is missing
- Test: unit tests in `event_bus.rs`

**Interfaces:**
- Produces:
  - `pub struct EventEnvelope { pub id: uuid::Uuid, pub event_type: String, pub payload: serde_json::Value }` (`Debug, Clone, PartialEq, Serialize, Deserialize`)
  - `pub enum HandlerError { Transient(String), Permanent(String) }` (`thiserror`)
  - `#[async_trait] pub trait EventHandler: Send + Sync { fn consumer(&self) -> &'static str; fn event_type(&self) -> &'static str; async fn handle(&self, event: &EventEnvelope) -> Result<(), HandlerError>; }`
  - `pub struct Subscriptions` with `new()`, `add(Arc<dyn EventHandler>)`, `for_type(&self, event_type: &str) -> Vec<Arc<dyn EventHandler>>`

- [ ] **Step 1: Write failing tests:** `for_type_returns_only_matching_handlers` (two handlers, two types, assert counts and `consumer()` names), `envelope_json_roundtrip` (serialize/deserialize equality).
- [ ] **Step 2:** `cargo test -p platform event_bus` — Expected: FAIL (compile error, items missing).
- [ ] **Step 3:** Implement the items above in `event_bus.rs`; export `pub mod event_bus;`.
- [ ] **Step 4:** `cargo test -p platform event_bus && cargo clippy -p platform --all-targets -- -D warnings` — Expected: PASS, no warnings.
- [ ] **Step 5: Commit** `feat(platform): add event envelope and handler registry`

### Task 2: Idempotency claim and outbox Relay

**Files:**
- Create: `crates/platform/src/outbox.rs`, `crates/platform/migrations/20260101000000_platform_init.sql`
- Modify: `crates/platform/src/lib.rs` (`pub mod outbox;`, `pub fn migrator()`), `crates/platform/Cargo.toml` (`tokio`, `tracing`, dev-deps `testcontainers-modules`, `tokio` macros), `apps/api/src/main.rs` + `xtask/src/main.rs` (apply `platform::migrator()` first), `.sqlx/` via `just db-prepare`
- Test: `crates/platform/tests/outbox.rs` (testcontainers Postgres)

**Interfaces:**
- Consumes: Task 1 `EventEnvelope`, `EventHandler`, `Subscriptions`, `HandlerError`.
- Migration creates schema `platform`, table `platform.processed_events(consumer text, event_id uuid, processed_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY (consumer, event_id))` with a rollback-plan comment.
- Produces:
  - `pub async fn claim(tx: &mut sqlx::PgConnection, consumer: &str, event_id: Uuid) -> Result<bool, sqlx::Error>` — `INSERT ... ON CONFLICT DO NOTHING`, true when this call inserted the row.
  - `pub struct OutboxTable` with `pub fn new(schema: &str) -> Result<Self, OutboxError>` (accepts `^[a-z][a-z0-9_]{0,62}$` only).
  - `pub trait Broadcaster: Send + Sync { async fn publish(&self, event: &EventEnvelope) -> Result<(), BroadcastError>; }` (via `async_trait`; defined here, implemented in Task 3) and `pub struct NoBroadcast` (no-op impl).
  - `pub struct Relay` with `new(pool: PgPool, tables: Vec<OutboxTable>, subscriptions: Arc<Subscriptions>, broadcaster: Arc<dyn Broadcaster>)`, `pub async fn tick(&self) -> Result<usize, sqlx::Error>` (events published this tick), and `pub async fn run(self, interval: Duration, shutdown: impl Future<Output = ()>)`.
- Algorithm per table in `tick`: `BEGIN; SELECT id,event_type,payload FROM {schema}.outbox WHERE published_at IS NULL ORDER BY created_at, id LIMIT 1 FOR UPDATE SKIP LOCKED;` run every handler for `event_type` (error → rollback, log with `event_id`/`consumer`, leave unpublished, continue with next table); then best-effort `broadcaster.publish` (error → `tracing::warn!`, still proceed); `UPDATE ... SET published_at = now() WHERE id = $1; COMMIT`. Loop per table until no row.

- [ ] **Step 1: Write failing integration tests** (`delivers_and_marks_published`, `failing_handler_leaves_event_unpublished_and_retries`, `redelivery_after_crash_does_not_repeat_effect` — handler uses `claim` then increments a counter table; simulate crash by calling handler directly once before `tick`; `two_relays_handle_each_event_once` — 20 events, two `Relay`s ticking concurrently via `tokio::join!` loops, assert counter == 20; `invalid_schema_is_rejected` — `OutboxTable::new("a;drop")` is `Err`). Test outbox schema is created by the test itself with the same columns as the modules (`id, event_type, payload, created_at, published_at`).
- [ ] **Step 2:** `cargo test -p platform --test outbox` — Expected: FAIL (compile).
- [ ] **Step 3:** Implement migration, `claim`, `OutboxTable`, `Relay` per the algorithm. SQL for the dynamic schema uses `sqlx::query` with the validated identifier (justify in a comment: identifiers cannot be bound).
- [ ] **Step 4:** `cargo test -p platform --test outbox` — Expected: all PASS. Run it 5× (`for i in 1 2 3 4 5`) to expose races — Expected: always PASS.
- [ ] **Step 5:** Run `just db-prepare` (needs `just dev-infra` + `just db-migrate`), then `just check` — Expected: green; commit regenerated `.sqlx`.
- [ ] **Step 6: Commit** `feat(platform): add idempotent outbox relay`

### Task 3: Redis broadcast publisher and listener

**Files:**
- Create: `crates/platform/src/broadcast.rs`
- Modify: `crates/platform/src/lib.rs`, `crates/platform/Cargo.toml` (`redis`, `futures-util`), `Cargo.toml` (`redis = { version = "1.8.0", default-features = false, features = ["tokio-comp", "aio"] }` — verify feature names against crate docs before use), dev-dep `testcontainers-modules` feature `redis`
- Test: `crates/platform/tests/broadcast.rs`

**Interfaces:**
- Consumes: Task 2 `Broadcaster`, `BroadcastError`; Task 1 `EventEnvelope`.
- Produces:
  - `pub struct RedisBroadcaster` implementing `Broadcaster`; `RedisBroadcaster::connect(url: &SecretString) -> Result<Self, BroadcastError>`; channel name `events.<event_type>`; payload = JSON of the envelope.
  - `#[async_trait] pub trait BroadcastHandler: Send + Sync { fn event_type(&self) -> &'static str; async fn handle(&self, event: &EventEnvelope); }`
  - `pub async fn listen(url: &SecretString, handlers: Vec<Arc<dyn BroadcastHandler>>, shutdown: impl Future<Output = ()>) -> Result<(), BroadcastError>` — subscribes to the handlers' channels, reconnects with capped backoff on connection loss, ignores malformed messages with a `warn!`.

- [ ] **Step 1: Write failing tests:** `broadcast_reaches_every_listener` (two `listen` tasks = two instances, one publish, both handlers receive the envelope), `unrelated_event_type_is_ignored`, `malformed_message_does_not_stop_listener`.
- [ ] **Step 2:** `cargo test -p platform --test broadcast` — Expected: FAIL.
- [ ] **Step 3:** Implement publisher and listener (pub/sub connection per listener; no state shared across instances).
- [ ] **Step 4:** `cargo test -p platform --test broadcast` ×3 — Expected: PASS each time.
- [ ] **Step 5:** `just check` and `cargo deny check licenses sources` — Expected: green (redis is BSD-3-Clause, allowed).
- [ ] **Step 6: Commit** `feat(platform): add redis event broadcast`

### Task 4: Wire into the API composition root

**Files:**
- Modify: `apps/api/src/main.rs`, `crates/platform/src/settings.rs` (`redis_url: SecretString`, env `REDIS_URL`, default `redis://127.0.0.1:6379`), `.env.example`, `justfile` (`env` recipe writes `REDIS_URL`), `docs/STATUS.md`
- Test: `apps/api/tests/relay_wiring.rs` (or a unit test on a `spawn_background` helper)

**Interfaces:**
- Consumes: Tasks 1–3 types. No module registers handlers yet; the wiring builds `Subscriptions::new()`, `OutboxTable::new` for `identity`, `documents`, `spreadsheets` (and `collaboration` once it has an outbox), `RedisBroadcaster`, spawns `Relay::run` and `broadcast::listen` with the process shutdown signal, and awaits them on graceful shutdown.
- Produces: `fn spawn_background(...) -> JoinSet<()>` in `apps/api` used by `main`.

- [ ] **Step 1: Write failing test:** with Postgres + Redis containers, start the background tasks, insert a row into `documents.outbox`, register a test handler through the same builder, assert it is called once and the row's `published_at` is set within 5 s; second instance of `spawn_background` on the same DB does not cause a second call.
- [ ] **Step 2:** `cargo test -p api --test relay_wiring` — Expected: FAIL.
- [ ] **Step 3:** Implement settings field, `spawn_background`, call from `main`, shutdown handling (abort-free: tasks end on the shutdown future).
- [ ] **Step 4:** `just check` — Expected: green. Manual: `just dev-infra`, run two `just dev-api` instances is not possible on one port; covered by the test.
- [ ] **Step 5:** Update `docs/STATUS.md` (relay, broadcast, known gap: poison events, no dead letter).
- [ ] **Step 6: Commit** `feat(api): run outbox relay and redis listener`
