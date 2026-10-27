# Workspace Sharing (person-to-person ACL) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Owners can share a document or spreadsheet with another user by email as viewer, commenter or editor; shared users get matching access over REST; a "shared with me" list exists.

**Architecture:** New `workspace` module (contracts + core) owns shares and a read-model of resources fed by durable event handlers (`DocumentCreated/Renamed/Deleted`, `SpreadsheetCreated/Renamed/Deleted`). `documents` and `spreadsheets` consume the `workspace_contracts::PermissionChecker` port in their access policy (owner check stays local, so owners never depend on event lag). `identity` exposes `UserDirectory` for email lookup. Everything is multi-instance safe by construction (events through the outbox relay from plan `2026-10-25-event-relay.md`).

**Tech Stack:** Rust 1.96, axum, sqlx 0.9 (`query!` + `.sqlx` cache), utoipa, existing `platform` event runtime.

**Spec:** `docs/superpowers/specs/2026-10-25-sharing-foundation-design.md` §2.2–2.4, §4.

## Global Constraints

- CLAUDE.md applies in full: contracts-first, module boundaries (only `-contracts` dependencies across modules, no cross-schema SQL), newtype IDs, `thiserror` per module, HTTP mapping only in `api/`, explicit authz in every handler (default deny), IDOR tests, `utoipa` on every endpoint, `just gen-api` after handler changes, outbox writes in the same transaction as business data.
- Roles are ordered `Viewer < Commenter < Editor < Owner`; Owner can never be granted via a share; sharing/revoking is Owner-only; owner cannot share with self.
- Link sharing, folders, groups and expiring grants are out of scope here (follow-up plans); anonymous access does not exist.
- `just check` green before every commit; commits are Conventional Commits (scope `workspace`, `documents`, `spreadsheets`, `identity`, `api`).
- Shares appear after relay lag (≤ ~0.5 s); owner access has no lag. API tests that depend on the read-model drive the relay with `Relay::tick()` explicitly instead of sleeping.

## Review Focus

- Non-owner (including an editor/viewer) calling any share-management endpoint → 403/404, never data.
- Resource ids owned by someone else must not be probeable: `GET /files/{id}/shares` for a stranger returns the same status as for a nonexistent id.
- Sharing an unknown email → `user_not_found` (documented assumption), sharing with the owner's own email → 422.
- Revoke then access → denied immediately on next request (the policy reads the share live).
- A downgrade (editor → viewer) takes effect on the next write attempt.
- Event replay (same `DocumentCreated` twice, `Renamed` before `Created`) leaves the read-model correct.
- Soft-deleted resources vanish from "shared with me" and lose access.

---

### Task 1: `workspace-contracts` crate

**Files:**
- Create: `modules/workspace/contracts/{Cargo.toml,src/lib.rs}`
- Modify: root `Cargo.toml` (member + `[workspace.dependencies] workspace-contracts`)

**Interfaces:**
- Produces (all `missing_docs`-documented, `Serialize/Deserialize` where noted):
  - `typed_id!(ResourceId)` (via `shared_kernel`)
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum Role { Viewer, Commenter, Editor, Owner }` with `pub const fn can_write(self) -> bool` (≥ Editor) and `pub const fn can_read(self) -> bool` (always true)
  - `pub enum ResourceKind { Document, Spreadsheet }` (serde snake_case)
  - `#[async_trait] pub trait PermissionChecker: Send + Sync + Debug { async fn role(&self, user: identity_contracts::UserId, resource: ResourceId) -> Option<Role>; }`
  - events (serde, RFC 3339 time like existing contracts): `ResourceShared { resource_id, grantee: UserId, role: Role, shared_by: UserId, at }`, `PermissionRevoked { resource_id, user_id: UserId, at }` (emitted on revoke and on role downgrade)
- Dependencies: `identity-contracts`, `shared-kernel`, `serde`, `time`, `async-trait` only (no sqlx/axum/tokio).

- [ ] **Step 1: Write failing tests** in `src/lib.rs`: `roles_are_ordered` (`Viewer < Commenter < Editor < Owner`), `only_editor_and_owner_can_write`, `role_serializes_as_snake_case` (`"editor"`), `permission_revoked_json_roundtrip`.
- [ ] **Step 2:** `cargo test -p workspace-contracts` — Expected: FAIL (crate/items missing).
- [ ] **Step 3:** Implement per the interfaces.
- [ ] **Step 4:** `cargo test -p workspace-contracts && just arch` — Expected: PASS, no violations.
- [ ] **Step 5: Commit** `feat(workspace): add workspace contracts`

### Task 2: `UserDirectory` port in identity

**Files:**
- Modify: `modules/identity/contracts/src/lib.rs`, `modules/identity/core/src/{lib.rs,application/service.rs}` (+ a small adapter file `directory.rs`), `modules/identity/core/tests/` (extend existing API test file or add `directory.rs`)

**Interfaces:**
- Produces in `identity-contracts`: `#[async_trait] pub trait UserDirectory: Send + Sync + Debug { async fn find_by_email(&self, email: &str) -> Option<UserId>; }` (invalid or unknown email → `None`; lookup is case-insensitive as the existing `Email` value object defines).
- `identity::Module` gains `pub user_directory: Arc<dyn UserDirectory>`.

- [ ] **Step 1: Write failing test** (testcontainers like existing identity tests): register a user, `directory.find_by_email(email)` returns their id; uppercase variant also resolves; unknown and malformed addresses return `None`.
- [ ] **Step 2:** `cargo test -p identity directory` — Expected: FAIL.
- [ ] **Step 3:** Implement the adapter over the existing `UserRepository::find_by_email` and `Email::parse`; no new SQL.
- [ ] **Step 4:** `cargo test -p identity && just check` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(identity): expose user directory port`

### Task 3: Rename and delete events in documents and spreadsheets

**Files:**
- Modify: `modules/{documents,spreadsheets}/contracts/src/lib.rs`, `modules/{documents,spreadsheets}/core/src/{domain,application,infrastructure}/*` (event enum, outbox writer), their tests

**Interfaces:**
- Produces additive contract events: `DocumentRenamed { document_id, title, at }`, `DocumentDeleted { document_id, at }` and the spreadsheet equivalents (`SpreadsheetRenamed`, `SpreadsheetDeleted`), RFC 3339 `at`.
- Aggregates collect an internal event enum (e.g. `DocumentEvent::{Created, Renamed, Deleted}`); the repository writes one outbox row per event with `event_type` = the variant's contract type name, in the same transaction as the state change. Delete must also write its event transactionally (load the aggregate or write the row inside `repo.delete`).

- [ ] **Step 1: Write failing tests:** domain `rename_collects_renamed_event`, `delete_collects_deleted_event`; integration `rename_and_delete_write_outbox_rows` (assert rows with the exact `event_type` and payload fields after PUT/DELETE through the API), for both modules.
- [ ] **Step 2:** `cargo test -p documents -p spreadsheets` — Expected: FAIL.
- [ ] **Step 3:** Implement events, enum, repository changes; keep existing `DocumentCreated` payload unchanged (contract compatibility, CLAUDE.md §4.2.8).
- [ ] **Step 4:** `just db-prepare` (live DB) then `cargo test -p documents -p spreadsheets` — Expected: PASS; `.sqlx` updated.
- [ ] **Step 5: Commit** `feat(documents): emit rename and delete events` and `feat(spreadsheets): emit rename and delete events` (two commits).

### Task 4: `workspace` core module

**Files:**
- Create: `modules/workspace/core/{Cargo.toml,migrations/20260101000500_workspace_init.sql}`, `src/{lib.rs,domain/{mod,share}.rs,application/{mod,ports,service,projections}.rs,infrastructure/{mod,repository,permission}.rs,api/{mod,routes}.rs}`, `tests/workspace_api.rs`
- Modify: root `Cargo.toml` (members + deps), `apps/api/src/{main.rs,openapi.rs}`, `xtask/src/main.rs` (migrator), `openapi.json` + `packages/api-client` via `just gen-api`

**Interfaces:**
- Consumes: Task 1 contracts, Task 2 `UserDirectory`, Task 3 events, `platform::{event_bus::EventHandler, outbox::claim}`, `documents_contracts`/`spreadsheets_contracts` event types.
- Migration (`workspace` schema, rollback plan comment): `resources(id uuid pk, kind text, owner_id uuid, title text, created_at, updated_at, deleted_at)`; `shares(resource_id uuid, grantee_id uuid, role text CHECK (role IN ('viewer','commenter','editor')), shared_by uuid, created_at, updated_at, PRIMARY KEY (resource_id, grantee_id))` + index `(grantee_id, resource_id DESC)` justified in a comment; `outbox` like other modules. No FK to other schemas.
- Domain (pure): `Share::new(resource_owner, grantee, role, shared_by) -> Result<Share, ShareError>` rejecting `role == Owner` (`OwnerRoleNotGrantable`) and `grantee == owner` (`CannotShareWithOwner`).
- Application: `WorkspaceService` with `share(actor, resource, email, role)`, `revoke(actor, resource, user)`, `list_shares(actor, resource)`, `shared_with_me(user, cursor, limit)`; all return `Result<_, WorkspaceError>`; non-owner or unknown resource → `NotFound` (same variant, anti-probing); unknown email → `UserNotFound`; each write also inserts the contract event into `workspace.outbox` in the same transaction (`ResourceShared`, and `PermissionRevoked` for revoke or downgrade).
- Projection handlers (`EventHandler`, consumer names `workspace.resources`): one per event type from Task 3 + created events; each runs `claim` and an upsert guarded by `updated_at`/tombstone so replays and reordering are harmless (`Renamed`/`Deleted` before `Created` creates a tombstone/stub that `Created` completes).
- `PermissionChecker` impl (`infrastructure/permission.rs`): live query joining `shares` and `resources` (not deleted) → `Option<Role>`; owner → `Role::Owner`.
- HTTP under `/api/v1/files` (bearer required, `AuthUser`): `GET /{id}/shares`, `POST /{id}/shares` body `{ "email": string, "role": "viewer"|"commenter"|"editor" }` → 201/200, `DELETE /{id}/shares/{user_id}` → 204, `GET /shared-with-me?cursor&limit` cursor-paginated `{ items: [{ id, kind, title, role, owner_id, shared_at }], next_cursor }`. Problem+json codes: `not_found`, `user_not_found`, `invalid_role`, `cannot_share_with_owner`.
- `workspace::module(pool, user_directory) -> Module { router, permission_checker: Arc<dyn PermissionChecker>, event_handlers: Vec<Arc<dyn EventHandler>> }`; `apps/api` registers the handlers in `Subscriptions`, adds `"workspace"` to `outbox_schemas`, mounts the router, and passes `permission_checker` onward (Task 5).

- [ ] **Step 1: Write failing tests:** domain unit tests (`owner_role_is_not_grantable`, `cannot_share_with_owner`); application tests with in-memory fakes (`non_owner_gets_not_found`, `share_twice_updates_role`, `downgrade_emits_permission_revoked`); projection tests (`replayed_created_event_is_applied_once`, `renamed_before_created_is_not_lost`, `deleted_resource_is_hidden`); integration API tests with Postgres (`owner_shares_and_grantee_lists_it`, `stranger_cannot_list_or_modify_shares` (IDOR: same status for foreign and nonexistent id), `viewer_cannot_share`, `unknown_email_is_user_not_found`, `revoke_removes_access`, `pagination_cursor_is_stable`). Integration tests drive projections by inserting events and calling `Relay::tick()`.
- [ ] **Step 2:** `cargo test -p workspace` — Expected: FAIL.
- [ ] **Step 3:** Implement domain → application → infrastructure → api in that order, committing the test-green slice each time (domain+application first, then repository+projections, then API).
- [ ] **Step 4:** `just db-migrate && just db-prepare && just gen-api && just check` — Expected: all green, `openapi.json` and client regenerated, `xtask arch` clean.
- [ ] **Step 5: Commit** in slices: `feat(workspace): add share domain and service`, `feat(workspace): add repository and projections`, `feat(workspace): add sharing endpoints`.

### Task 5: Permission checks in documents and spreadsheets

**Files:**
- Modify: `modules/{documents,spreadsheets}/core/src/{application/ports.rs,application/service.rs,lib.rs,access.rs}`, `Cargo.toml` (dep `workspace-contracts`), `apps/api/src/main.rs`, existing API tests + new tests

**Interfaces:**
- Consumes: `workspace_contracts::{PermissionChecker, Role}`.
- Change: `AccessPolicy::allows` becomes `async fn allows(&self, actor, doc, action) -> bool` (via `async_trait`); new `OwnerOrShared { checker: Arc<dyn PermissionChecker> }`: owner → all actions; else `checker.role` → `Read` allowed for any role, `Write` for `Editor`+, `Delete` never. `documents::module(pool, checker)` and `spreadsheets::module(pool, checker)` take the checker; `DocumentAccess::access_level` (used by collaboration) is backed by the same policy (Viewer/Commenter → `Read`, Editor → `Write`).
- Reads of a foreign document by a user without any role keep returning the current not-found behaviour.

- [ ] **Step 1: Write failing tests** (API level, fake checker injected): `editor_can_update_content`, `viewer_can_read_but_not_update`, `shared_user_cannot_delete` (editors may rename, as in Google Drive; viewers may not), `stranger_still_gets_not_found`, and for collaboration's port `access_level_maps_roles` (`Viewer→Read`, `Editor→Write`, none→`None`).
- [ ] **Step 2:** `cargo test -p documents -p spreadsheets` — Expected: FAIL.
- [ ] **Step 3:** Implement the async policy and wiring; update `apps/api` composition: `workspace` module first, then pass `permission_checker` into documents/spreadsheets.
- [ ] **Step 4:** `cargo test --workspace && just check` — Expected: PASS. Add one end-to-end API test in `apps/api` or `workspace` tests: owner creates a document, shares by email, grantee `GET`s it (200), viewer `PUT`s (403/404 per policy), revoke → grantee `GET` fails.
- [ ] **Step 5: Commit** `feat(documents): honor workspace permissions` and `feat(spreadsheets): honor workspace permissions`.
