# Sharing Foundation (outbox relay, workspace ACL, multi-instance collaboration) — Design

Status: draft, decided under standing authorization to continue. Date: 2026-10-25.
Parent: `2026-10-25-vendored-engines-cloud-suite-design.md` (sub-project 2, first slice).

## 1. Intent

Google Drive-style sharing for Docs and Sheets: owners share a file with a person (by email) or
via a link with a role; viewers/commenters/editors get the matching access in REST and in the
real-time WebSocket; revoking access drops open sessions immediately. Must run as multiple
stateless instances. Chosen model: **simple role-based ACL** (owner / editor / commenter / viewer),
per file, plus link sharing. Out of scope here (later slices): folders and inheritance, groups,
expiring grants, comments, suggestions, version history, refresh tokens.

## 2. Components

### 2.1 `platform`: events and outbox (multi-instance safe)
- `platform::event_bus`: `EventEnvelope { id: Uuid, event_type: String, payload: serde_json::Value }`,
  traits `EventPublisher` and `EventSubscriber` (CLAUDE.md §4.2.5), `Subscriptions` registry.
- `platform::outbox::Relay`: every instance polls each registered schema's `<schema>.outbox`; a per-table
  advisory lock (`pg_try_advisory_xact_lock`) lets one instance work at a time, which keeps delivery
  roughly ordered and concurrent relays safe. Delivers one row at a time, sets `published_at`.
  At-least-once; failing events are retried, then parked in `platform.outbox_failures`.
  Poll interval plus Postgres `LISTEN/NOTIFY` wake-up is NOT used (keeps it simple); interval 500 ms.
- Two delivery kinds per subscription:
  - **Durable** (handler runs once cluster-wide): consumer inserts `event_id` into its own
    `<schema>.processed_events` in the same transaction as its effect (`ON CONFLICT DO NOTHING`).
  - **Broadcast** (runs on every instance): relay publishes the envelope to Redis channel
    `events.<event_type>`; each instance's listener invokes local handlers. Used for session drop.
- Redis loss does not lose durable events (Postgres is the source); broadcast is best-effort and
  additionally enforced by per-message permission re-checks (see 2.4).

### 2.2 `workspace-contracts` (new)
`Role { Viewer, Commenter, Editor, Owner }` (ordered), `ResourceKind { Document, Spreadsheet }`,
typed `ResourceId`, `PermissionChecker` port (`async fn role(user, kind, resource) -> Option<Role>`),
events `ResourceShared`, `ShareRevoked` and `PermissionRevoked { resource_id, user_id: UserId }`
(link-share revocation will get its own event type when link sharing lands). All additive and versionable (CLAUDE.md §4.2.8).

### 2.3 `workspace` core (new module, own schema)
- Tables: `workspace.resources` (read-model: id, kind, owner_id, title, deleted_at, filled from
  `DocumentCreated/Renamed/Deleted` and `SpreadsheetCreated/Renamed/Deleted`), `workspace.shares`
  (resource_id, grantee_user_id, role, created_by; unique per resource+grantee),
  `workspace.link_shares` (resource_id, token hash, role, revoked_at), `workspace.processed_events`,
  `workspace.outbox`.
- `PermissionChecker` impl: owner → Owner; else max(user share, valid link share) → role; else None.
  Link tokens are 256-bit random, only the SHA-256 is stored, revocable.
- HTTP (all authz-explicit, only Owner may manage): `GET/POST /api/v1/files/{id}/shares`,
  `DELETE /api/v1/files/{id}/shares/{user_id}`, `PUT/DELETE /api/v1/files/{id}/link`,
  `GET /api/v1/shared-with-me` (cursor pagination). Share target is an email resolved through the new
  port `identity_contracts::UserDirectory::find_by_email`; unknown email → 404 without leaking
  whether the account exists (same response shape as success is NOT used; a clear `user_not_found` is
  acceptable because the owner already knows the email — recorded as an assumption).
- Domain stays pure (`Share`, `Role` rules: cannot grant Owner, cannot share to the owner).

### 2.4 Consumers
- `documents`/`spreadsheets` `AccessPolicy`: owner OR `PermissionChecker.role` mapped to
  Read (Viewer, Commenter) / Write (Editor); delete and sharing stay Owner-only. `DocumentAccess`
  (existing port) keeps its signature and is backed by that policy.
- `collaboration`: broadcast subscriber on `PermissionRevoked`; every instance closes matching local
  sessions (close code 1008). Defence in depth: on each incoming update from a session the room
  re-validates the cached access level older than 30 s (bounded staleness if Redis drops a message).
- `collaboration` multi-instance: Yrs updates persisted append-only (exists) and published to Redis
  channel `collab.<document_id>`; instances hosting that room apply remote updates (origin-tagged to
  prevent echo). Compaction takes a per-document Postgres advisory lock.

### 2.5 Frontend
Share dialog (people + role selector, link toggle with role, copy link), "Shared with me" list on
Home, read-only/comment state in editors from the effective role, i18n `tr`/`en`, a11y per CLAUDE.md.

## 3. Data flow
Owner POST share → `workspace` writes share + outbox `ResourceShared` (one tx) → any instance relay
delivers → durable consumers update read-models; revoke → `PermissionRevoked` → relay → Redis
broadcast → every instance drops matching sessions.

## 4. Security
Explicit authz on every handler, IDOR tests (non-owner cannot list/modify shares, cannot enumerate
other users' resources), link tokens hashed + revocable + constant-time compare, rate limit on share
and link-resolve endpoints, audit rows for share/revoke (outbox events double as audit trail).

## 5. Testing
Domain unit tests; application tests with in-memory fakes; Postgres+Redis testcontainers
integration tests; **two-instance test** (two apps, one DB, one Redis): share on A visible on B,
revoke on A drops a WebSocket held on B, update typed on A reaches a client on B; each outbox
event is handled exactly once with 2 relays running (property: no duplicate effects); Playwright
two-user share flow later.

## 6. Order of work
(1) platform event bus + relay + Redis broadcast, (2) `workspace-contracts` + `identity UserDirectory`
+ documents/spreadsheets rename/delete events, (3) `workspace` core + API, (4) policy wiring in
documents/spreadsheets, (5) collaboration multi-instance + session drop, (6) frontend sharing UI.
Each is a separate plan; plan 1 is written first.

## 7. Assumptions / risks
- Redis is available in every environment (already in compose); broadcast is best-effort by design.
- Read-model lag: a share is usable only after the relay applied it; owner actions are
  read-your-writes because `resources` also gets the owner synchronously via the event only after
  creation, so owner access is resolved by the module's own `owner_id` (no lag for owners).
- Open: folder inheritance (next slice), groups, expiring grants.
