# Profile Page, GDPR/KVKK & UI Enrichment — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a full `/profile` page with display name/bio/avatar, password change, GDPR/KVKK data export and account deletion with 30-day grace period, plus app-wide UI polish (header avatar dropdown, empty states, 404 redesign).

**Architecture:** Extends `modules/identity` with new domain operations and five HTTP endpoints; adds a `WorkspaceExportPort` to `workspace-contracts` for GDPR export; adds a `pages/profile` feature to the Vue SPA following the existing FSD layer pattern (feature → `index.ts` public API, sub-components for each tab).

**Tech Stack:** Rust/axum/sqlx/argon2/thiserror (backend), Vue 3 / TypeScript / Tailwind / Reka UI / TanStack Query / vue-i18n / zod (frontend).

**Spec:** `docs/superpowers/specs/2026-10-27-profile-gdpr-ui-design.md`

---

## Global Constraints

- Edition 2024, `#![forbid(unsafe_code)]` on every crate; `[lints] workspace = true`.
- No `unwrap()`/`panic!`/`todo!` in production code.
- All timestamps UTC; IDs UUID v7.
- `avatar_url` must start with `https://`, max 500 chars; `bio` max 300 chars — validated at domain layer.
- `change_password` success revokes ALL refresh tokens for the user.
- GDPR export endpoint uses the authenticated caller's id only (no id query parameter).
- `DELETE /me` requires `confirm_display_name` matching the stored value exactly (case-sensitive).
- No billing surface; app is open-source.
- All user-visible strings use `t('key')` — no hardcoded strings.
- Avatar URL input is `https://` only (no `data:`, `javascript:`).
- `just check` must stay green after every task.

## Review Focus

- `change_password` with correct old password but too-short new password should return `validation_failed`, not `invalid_credentials`. (Task 5, service test)
- `schedule_deletion` called twice should return `409 deletion_already_scheduled`, not silently succeed. (Task 5, service test + Task 6, API test)
- `GET /me/export` must return the authenticated user's data, not accept a query param for id. (Task 6, API test)
- `avatar_url` with `http://` (not https) must be rejected at domain layer with `422`. (Task 2, domain unit test)
- Confirm dialog "Sil" button must remain disabled when the typed name doesn't exactly match the display name (case-sensitive). (Task 13, component test)

---

## File Map

### Backend — new/modified
| Path | Change |
|------|--------|
| `modules/identity/core/migrations/20260101000102_profile_gdpr.sql` | NEW — adds 3 columns |
| `modules/identity/core/src/domain/values.rs` | MODIFY — add `Bio`, `AvatarUrl` value objects |
| `modules/identity/core/src/domain/user.rs` | MODIFY — new fields + 4 new operations |
| `modules/identity/contracts/src/lib.rs` | MODIFY — add `ProfileUpdated`, `UserDeletionScheduled` events |
| `modules/workspace/contracts/src/lib.rs` | MODIFY — add `WorkspaceExportPort`, `FileExportRow`, `ExportError` |
| `modules/identity/core/src/application/ports.rs` | MODIFY — extend `UserRepository` trait + `RepositoryError` |
| `modules/identity/core/src/application/service.rs` | MODIFY — add `IdentityError` variants + 5 new service methods |
| `modules/identity/core/src/infrastructure/repository.rs` | MODIFY — `UserRow` + 4 new impl methods |
| `modules/workspace/core/src/infrastructure/export.rs` | NEW — `PgWorkspaceExportAdapter` |
| `modules/workspace/core/src/lib.rs` | MODIFY — expose export adapter |
| `modules/identity/core/src/api/routes.rs` | MODIFY — 5 new endpoints, extend `UserResponse` |
| `apps/api/src/main.rs` (or lib.rs) | MODIFY — inject `WorkspaceExportPort` into identity state |

### Frontend — new/modified
| Path | Change |
|------|--------|
| `apps/web/src/shared/i18n/locales/en.ts` | MODIFY — add `profile.*`, error codes |
| `apps/web/src/shared/i18n/locales/tr.ts` | MODIFY — same keys in Turkish |
| `apps/web/src/shared/config/routes.ts` | MODIFY — add `Profile` to `RouteName` |
| `apps/web/src/entities/session/model/session-store.ts` | MODIFY — `SessionUser` gains `avatarUrl`, `bio`, `deletionScheduledAt` |
| `apps/web/src/features/profile/api/keys.ts` | NEW |
| `apps/web/src/features/profile/api/use-profile-mutations.ts` | NEW |
| `apps/web/src/features/profile/ui/ProfileTab.vue` | NEW |
| `apps/web/src/features/profile/ui/SecurityTab.vue` | NEW |
| `apps/web/src/features/profile/ui/AppearanceTab.vue` | NEW |
| `apps/web/src/features/profile/ui/DataPrivacyTab.vue` | NEW |
| `apps/web/src/features/profile/index.ts` | NEW |
| `apps/web/src/pages/profile/ui/ProfilePage.vue` | NEW |
| `apps/web/src/pages/profile/index.ts` | NEW |
| `apps/web/src/app/router.ts` | MODIFY — add `/profile` route |
| `apps/web/src/features/auth/ui/UserMenu.vue` | MODIFY — dropdown with "Profile" link |
| `apps/web/src/widgets/recent-files/ui/RecentFiles.vue` | MODIFY — empty state |
| `apps/web/src/widgets/shared-files/ui/SharedFiles.vue` | MODIFY — empty state |
| `apps/web/src/pages/not-found/ui/NotFoundPage.vue` | MODIFY — redesign |

---

## Task 1: Database Migration

**Files:**
- Create: `modules/identity/core/migrations/20260101000102_profile_gdpr.sql`

**Interfaces:**
- Produces: `identity.users` has columns `avatar_url text`, `bio text`, `deletion_scheduled_at timestamptz` (all nullable)

- [ ] **Step 1: Write migration SQL**

```sql
-- Rollback: ALTER TABLE identity.users DROP COLUMN avatar_url, DROP COLUMN bio, DROP COLUMN deletion_scheduled_at.
ALTER TABLE identity.users ADD COLUMN avatar_url            text;
ALTER TABLE identity.users ADD COLUMN bio                   text;
ALTER TABLE identity.users ADD COLUMN deletion_scheduled_at timestamptz;
```

- [ ] **Step 2: Apply migration to dev DB**

```bash
just dev-infra   # ensure postgres is running
just db-migrate
```

Expected: migration applies cleanly; `just db-migrate` exits 0.

- [ ] **Step 3: Update sqlx offline cache**

```bash
just db-prepare
```

- [ ] **Step 4: Commit**

```bash
git add modules/identity/core/migrations/20260101000102_profile_gdpr.sql
git add .sqlx/
git commit -m "feat(identity): add profile and deletion columns"
```

---

## Task 2: Domain Value Objects + User Operations

**Files:**
- Modify: `modules/identity/core/src/domain/values.rs`
- Modify: `modules/identity/core/src/domain/user.rs`
- Test: `modules/identity/core/src/domain/` (inline `#[cfg(test)]` blocks)

**Interfaces:**
- Consumes: existing `DisplayName`, `ValueError`
- Produces:
  - `Bio::new(s: &str) -> Result<Self, ValueError>` — rejects > 300 chars
  - `AvatarUrl::parse(s: &str) -> Result<Self, ValueError>` — rejects non-`https://`, > 500 chars
  - `AvatarUrl::as_str(&self) -> &str`
  - `Bio::as_str(&self) -> &str`
  - `User::avatar_url(&self) -> Option<&AvatarUrl>`
  - `User::bio(&self) -> Option<&Bio>`
  - `User::deletion_scheduled_at(&self) -> Option<OffsetDateTime>`
  - `User::update_profile(display_name, bio: Option<Bio>, avatar_url: Option<AvatarUrl>, now) -> ProfileUpdated` — updates self, returns event
  - `User::change_password(old_hash_matches: bool, new_hash: String) -> Result<(), ValueError>` — `Err` when `!old_hash_matches`; domain does not call hasher
  - `User::schedule_deletion(now: OffsetDateTime) -> Result<UserDeletionScheduled, ValueError>` — `Err` (`ValueError` variant `DeletionAlreadyScheduled`) when `deletion_scheduled_at.is_some()`; sets `deletion_scheduled_at = now + 30 days`
  - `User::cancel_deletion() -> Result<(), ValueError>` — `Err` (`ValueError` variant `NoPendingDeletion`) when field is `None`; clears field

- [ ] **Step 1: Add `Bio` value object with failing test**

In `values.rs`, add to the test module:
```rust
#[test]
fn bio_rejects_over_300_chars() {
    let long = "a".repeat(301);
    assert!(Bio::new(&long).is_err());
}
#[test]
fn bio_accepts_empty_and_300_chars() {
    assert!(Bio::new("").is_ok());
    assert!(Bio::new(&"a".repeat(300)).is_ok());
}
```
Run: `cargo test -p identity domain::values` — expected FAIL (type not defined).

- [ ] **Step 2: Implement `Bio`**

Newtype `Bio(String)`, `Bio::new(s: &str) -> Result<Self, ValueError>`, `Bio::as_str(&self) -> &str`.
Add a `ValueError::BioTooLong` variant if needed (or reuse the existing validation variant — check how `DisplayName` does it).

- [ ] **Step 3: Add `AvatarUrl` value object with failing test**

```rust
#[test]
fn avatar_url_rejects_http() {
    assert!(AvatarUrl::parse("http://example.com/pic.jpg").is_err());
}
#[test]
fn avatar_url_rejects_data_uri() {
    assert!(AvatarUrl::parse("data:image/png;base64,abc").is_err());
}
#[test]
fn avatar_url_rejects_over_500_chars() {
    let long = format!("https://x.com/{}", "a".repeat(490));
    assert!(AvatarUrl::parse(&long).is_err());
}
#[test]
fn avatar_url_accepts_valid_https() {
    assert!(AvatarUrl::parse("https://example.com/pic.jpg").is_ok());
}
```
Run: `cargo test -p identity domain::values` — expected FAIL.

- [ ] **Step 4: Implement `AvatarUrl`**

Newtype `AvatarUrl(String)`, `AvatarUrl::parse(s: &str) -> Result<Self, ValueError>`, `AvatarUrl::as_str(&self) -> &str`.

- [ ] **Step 5: Verify value object tests pass**

Run: `cargo test -p identity domain::values`
Expected: all new tests PASS.

- [ ] **Step 6: Extend `User` aggregate with new fields and failing tests**

Add test module in `user.rs`:
```rust
#[test]
fn update_profile_changes_display_name_and_bio() {
    let mut user = make_test_user();
    let new_name = DisplayName::parse("Updated Name").unwrap();
    let bio = Bio::new("Hello").unwrap();
    let _event = user.update_profile(new_name.clone(), Some(bio), None, OffsetDateTime::now_utc());
    assert_eq!(user.display_name().as_str(), "Updated Name");
    assert!(user.bio().is_some());
}
#[test]
fn schedule_deletion_sets_date_30_days_out() {
    let mut user = make_test_user();
    let now = OffsetDateTime::now_utc();
    let event = user.schedule_deletion(now).unwrap();
    assert!(user.deletion_scheduled_at().is_some());
    // 29–31 days to account for rounding
    let diff = event.execute_after - now;
    assert!(diff.whole_days() >= 29 && diff.whole_days() <= 31);
}
#[test]
fn schedule_deletion_twice_returns_err() {
    let mut user = make_test_user();
    let now = OffsetDateTime::now_utc();
    user.schedule_deletion(now).unwrap();
    assert!(user.schedule_deletion(now).is_err());
}
#[test]
fn cancel_deletion_clears_date() {
    let mut user = make_test_user();
    let now = OffsetDateTime::now_utc();
    user.schedule_deletion(now).unwrap();
    user.cancel_deletion().unwrap();
    assert!(user.deletion_scheduled_at().is_none());
}
#[test]
fn cancel_deletion_without_pending_returns_err() {
    let mut user = make_test_user();
    assert!(user.cancel_deletion().is_err());
}
```
Run: `cargo test -p identity domain::user` — expected FAIL.

- [ ] **Step 7: Extend `User` aggregate**

Add fields `avatar_url: Option<AvatarUrl>`, `bio: Option<Bio>`, `deletion_scheduled_at: Option<OffsetDateTime>` to the struct.
Update `restore(...)` to accept these three new `Option` parameters.
Implement the four operations per spec §2.2.
`update_profile` returns `identity_contracts::ProfileUpdated` (add this event to contracts in Task 3 first — or use a placeholder struct if doing this before Task 3).

> **Note:** Task 3 adds the event types to contracts. Implement `update_profile` and `schedule_deletion` to return those events. If doing tasks in order, do Task 3 before completing this step.

- [ ] **Step 8: Verify all domain tests pass**

Run: `cargo test -p identity`
Expected: all tests PASS.

- [ ] **Step 9: Commit**

```bash
git add modules/identity/core/src/domain/
git commit -m "feat(identity): add Bio/AvatarUrl value objects and profile/deletion user operations"
```

---

## Task 3: New Contract Events + WorkspaceExportPort

**Files:**
- Modify: `modules/identity/contracts/src/lib.rs`
- Modify: `modules/workspace/contracts/src/lib.rs`

**Interfaces:**
- Produces:
  - `identity_contracts::ProfileUpdated { user_id: UserId, display_name: String, updated_at: OffsetDateTime }`
  - `identity_contracts::UserDeletionScheduled { user_id: UserId, scheduled_at: OffsetDateTime, execute_after: OffsetDateTime }`
  - `workspace_contracts::ExportError` (thiserror enum, single variant `Storage`)
  - `workspace_contracts::FileExportRow { id: String, name: String, kind: String, created_at: OffsetDateTime, updated_at: OffsetDateTime }`
  - `workspace_contracts::WorkspaceExportPort: Send + Sync + Debug` — `async fn list_owned_files(&self, owner: UserId) -> Result<Vec<FileExportRow>, ExportError>`

- [ ] **Step 1: Add `ProfileUpdated` and `UserDeletionScheduled` to `identity-contracts`**

Both derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. Timestamps use `#[serde(with = "time::serde::rfc3339")]`.

- [ ] **Step 2: Add `WorkspaceExportPort` to `workspace-contracts`**

Add `ExportError`, `FileExportRow` structs and the trait. Use `#[async_trait::async_trait]`.
`workspace-contracts/Cargo.toml` already has `async-trait`; add `thiserror` if missing.

- [ ] **Step 3: Verify compile**

Run: `cargo check -p identity-contracts -p workspace-contracts`
Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add modules/identity/contracts/src/lib.rs modules/workspace/contracts/src/lib.rs
git commit -m "feat(contracts): add ProfileUpdated, UserDeletionScheduled events and WorkspaceExportPort"
```

---

## Task 4: Repository Trait + PgUserRepository Extensions

**Files:**
- Modify: `modules/identity/core/src/application/ports.rs`
- Modify: `modules/identity/core/src/infrastructure/repository.rs`
- Create: `modules/workspace/core/src/infrastructure/export.rs`
- Modify: `modules/workspace/core/src/lib.rs`

**Interfaces:**
- Consumes: `Bio`, `AvatarUrl`, `ProfileUpdated`, `UserDeletionScheduled` from Tasks 2–3
- Produces (new `UserRepository` methods):
  - `async fn update_profile(&self, id: UserId, display_name: &DisplayName, bio: Option<&Bio>, avatar_url: Option<&AvatarUrl>, now: OffsetDateTime) -> Result<(), RepositoryError>`
  - `async fn update_password(&self, id: UserId, new_hash: &str, now: OffsetDateTime) -> Result<(), RepositoryError>`
  - `async fn revoke_all_refresh_tokens(&self, user_id: UserId, now: OffsetDateTime) -> Result<(), RepositoryError>`
  - `async fn schedule_deletion(&self, id: UserId, execute_after: OffsetDateTime, now: OffsetDateTime) -> Result<(), RepositoryError>`
  - `async fn cancel_deletion(&self, id: UserId) -> Result<(), RepositoryError>`
- `UserRow` gains `avatar_url: Option<String>`, `bio: Option<String>`, `deletion_scheduled_at: Option<OffsetDateTime>`
- `to_user` passes those three through `User::restore`
- `PgWorkspaceExportAdapter` implements `WorkspaceExportPort`; reads `workspace.resources` table (columns `id uuid, name text, kind text, created_at timestamptz, updated_at timestamptz, owner_id uuid`)

- [ ] **Step 1: Extend `UserRepository` trait in `ports.rs`**

Add the five method signatures above. No default implementations.

- [ ] **Step 2: Extend `UserRow` and `to_user` in `repository.rs`**

`UserRow` gains the three new Option fields.
`find_by_email` and `find_by_id` SELECT queries gain `, avatar_url, bio, deletion_scheduled_at`.
`to_user` parses `Bio::new` and `AvatarUrl::parse` from the Option strings, mapping errors to `RepositoryError::Storage`.
`User::restore(...)` call gains the three new arguments.

- [ ] **Step 3: Implement the five new repository methods**

`update_profile`: `UPDATE identity.users SET display_name=$2, bio=$3, avatar_url=$4, updated_at=$5 WHERE id=$1`. Writes `ProfileUpdated` event to outbox in same transaction.

`update_password`: `UPDATE identity.users SET password_hash=$2, updated_at=$3 WHERE id=$1`.

`revoke_all_refresh_tokens`: `UPDATE identity.refresh_tokens SET revoked_at=$2 WHERE user_id=$1 AND revoked_at IS NULL AND expires_at > $2`. (Check actual column names in `refresh` table migration; adapt if needed.)

`schedule_deletion`: `UPDATE identity.users SET deletion_scheduled_at=$2, updated_at=$3 WHERE id=$1`. Writes `UserDeletionScheduled` event to outbox in same transaction.

`cancel_deletion`: `UPDATE identity.users SET deletion_scheduled_at=NULL, updated_at=NOW() WHERE id=$1`.

- [ ] **Step 4: Write `PgWorkspaceExportAdapter`**

Create `modules/workspace/core/src/infrastructure/export.rs`:
```rust
pub struct PgWorkspaceExportAdapter { pool: PgPool }
impl PgWorkspaceExportAdapter {
    pub const fn new(pool: PgPool) -> Self { Self { pool } }
}
```
Implement `WorkspaceExportPort`: query `workspace.resources WHERE owner_id = $1 AND deleted_at IS NULL ORDER BY updated_at DESC`.
Map rows to `FileExportRow`; map sqlx errors to `ExportError::Storage`.

- [ ] **Step 5: Expose from `workspace/core/src/lib.rs`**

`pub use infrastructure::export::PgWorkspaceExportAdapter;`

- [ ] **Step 6: Verify compile**

Run: `cargo check -p identity -p workspace`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add modules/identity/core/src/application/ports.rs \
        modules/identity/core/src/infrastructure/repository.rs \
        modules/workspace/core/src/infrastructure/export.rs \
        modules/workspace/core/src/lib.rs
git commit -m "feat(identity,workspace): extend repository with profile/deletion ops and add WorkspaceExportPort"
```

---

## Task 5: IdentityService New Methods

**Files:**
- Modify: `modules/identity/core/src/application/service.rs`
- Test: `modules/identity/core/tests/profile.rs` (new integration test file)

**Interfaces:**
- Consumes: all new repository methods from Task 4; `PasswordHasher::verify`
- Produces (new service methods, all `pub(crate) async fn`):
  - `update_profile(&self, id: UserId, display_name: &str, bio: Option<&str>, avatar_url: Option<&str>) -> Result<User, IdentityError>`
  - `change_password(&self, id: UserId, current: &str, new_password: &str) -> Result<(), IdentityError>`
  - `schedule_deletion(&self, id: UserId) -> Result<OffsetDateTime, IdentityError>` — returns `execute_after`
  - `cancel_deletion(&self, id: UserId) -> Result<(), IdentityError>`
  - `gdpr_export(&self, id: UserId, workspace: &dyn WorkspaceExportPort) -> Result<GdprExport, IdentityError>` where `GdprExport` is a new pub(crate) struct with `account` and `files` fields
- New `IdentityError` variants: `ConfirmationMismatch`, `DeletionAlreadyScheduled`, `NoPendingDeletion`, `AtLeastOneFieldRequired`

- [ ] **Step 1: Add new `IdentityError` variants**

In `service.rs`, add:
```rust
ConfirmationMismatch,
DeletionAlreadyScheduled,
NoPendingDeletion,
AtLeastOneFieldRequired,
```
Add `From` mappings in the `ProblemDetails` conversion in `routes.rs` (anticipating Task 6).

- [ ] **Step 2: Add `GdprExport` struct**

```rust
pub(crate) struct GdprExport {
    pub(crate) account: GdprAccount,
    pub(crate) files: Vec<workspace_contracts::FileExportRow>,
}
pub(crate) struct GdprAccount {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) display_name: String,
    pub(crate) bio: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) created_at: time::OffsetDateTime,
}
```

- [ ] **Step 3: Write failing integration tests**

Create `modules/identity/core/tests/profile.rs`. Tests use `testcontainers` (same pattern as `auth_flow.rs`):
```rust
#[tokio::test]
async fn update_profile_persists_bio_and_avatar() { /* ... */ }

#[tokio::test]
async fn change_password_wrong_current_returns_invalid_credentials() { /* ... */ }

#[tokio::test]
async fn change_password_short_new_returns_validation_failed() { /* ... */ }

#[tokio::test]
async fn schedule_deletion_twice_returns_already_scheduled() { /* ... */ }

#[tokio::test]
async fn cancel_deletion_without_pending_returns_no_pending() { /* ... */ }
```
Run: `cargo nextest run -p identity --test profile` — expected FAIL (methods not yet implemented).

- [ ] **Step 4: Implement `update_profile`**

Parse `DisplayName`, `Option<Bio>`, `Option<AvatarUrl>`. Call `users.update_profile(...)`.

- [ ] **Step 5: Implement `change_password`**

Load user. Call `hasher.verify(current, user.password_hash()).await`. If false → `IdentityError::InvalidCredentials`. Parse `Password::new(new_password)?` (reuses existing `ValueError` → `IdentityError::Invalid`). Hash. Call `users.update_password(...)`. Then call `users.revoke_all_refresh_tokens(...)`.

- [ ] **Step 6: Implement `schedule_deletion`**

Load user. Call `user.schedule_deletion(clock.now())` — map `ValueError::DeletionAlreadyScheduled` → `IdentityError::DeletionAlreadyScheduled`. Call `users.schedule_deletion(id, event.execute_after, now)`. Return `event.execute_after`.

- [ ] **Step 7: Implement `cancel_deletion`**

Load user. Call `user.cancel_deletion()` — map `ValueError::NoPendingDeletion` → `IdentityError::NoPendingDeletion`. Call `users.cancel_deletion(id)`.

- [ ] **Step 8: Implement `gdpr_export`**

Load user. Call `workspace.list_owned_files(id).await` — map `ExportError` → `IdentityError::Internal`. Build `GdprExport`.

- [ ] **Step 9: Run integration tests**

Run: `cargo nextest run -p identity --test profile`
Expected: all tests PASS.

- [ ] **Step 10: Commit**

```bash
git add modules/identity/core/src/application/service.rs \
        modules/identity/core/tests/profile.rs
git commit -m "feat(identity): add update_profile, change_password, schedule_deletion, cancel_deletion, gdpr_export service methods"
```

---

## Task 6: New HTTP Endpoints

**Files:**
- Modify: `modules/identity/core/src/api/routes.rs`
- Modify: `apps/api/src/main.rs` (or wherever identity `ApiState` is built)
- Test: `modules/identity/core/tests/profile_api.rs` (new)

**Interfaces:**
- Consumes: all new service methods from Task 5
- Produces:
  - `PATCH /api/v1/auth/me` — body `UpdateProfileRequest { display_name: Option<String>, bio: Option<String>, avatar_url: Option<String> }` → `200 UserResponse` or `422`
  - `POST /api/v1/auth/me/change-password` — body `ChangePasswordRequest { current_password: String, new_password: String }` → `204` or `401`/`422`
  - `GET /api/v1/auth/me/export` — `200 application/json` with `Content-Disposition: attachment; filename="my-data.json"`, body `GdprExportResponse`
  - `DELETE /api/v1/auth/me` — body `DeleteAccountRequest { confirm_display_name: String }` → `200 { execute_after: String }` or `409`/`422`
  - `POST /api/v1/auth/me/cancel-deletion` — `204` or `409`
- `UserResponse` gains `avatar_url: Option<String>`, `bio: Option<String>`, `deletion_scheduled_at: Option<String>`
- `ApiState` gains `workspace_export: Arc<dyn WorkspaceExportPort>`

- [ ] **Step 1: Extend `UserResponse` and `ApiState`**

Add fields to `UserResponse`. Add `workspace_export: Arc<dyn WorkspaceExportPort>` to `ApiState`.
Update `From<&User>` impl to include the new fields (RFC 3339 string for `deletion_scheduled_at`).

- [ ] **Step 2: Add new error → ProblemDetails mappings**

In the `From<IdentityError> for ProblemDetails` impl:
- `DeletionAlreadyScheduled` → `409 "deletion_already_scheduled"`
- `NoPendingDeletion` → `409 "no_pending_deletion"`
- `ConfirmationMismatch` → `422 "confirmation_mismatch"`
- `AtLeastOneFieldRequired` → `422 "at_least_one_field_required"`

- [ ] **Step 3: Write failing API tests**

Create `modules/identity/core/tests/profile_api.rs` using the existing `auth_flow.rs` pattern (spin up axum test app with testcontainers):
```rust
#[tokio::test]
async fn patch_me_updates_bio() { /* POST /register → login → PATCH /me {bio: "Hello"} → GET /me → assert bio */ }

#[tokio::test]
async fn patch_me_rejects_http_avatar_url() { /* PATCH /me {avatar_url: "http://..."} → 422 */ }

#[tokio::test]
async fn change_password_wrong_current_returns_401() { /* ... */ }

#[tokio::test]
async fn export_returns_account_and_files() { /* login → GET /me/export → assert schema_version=1 */ }

#[tokio::test]
async fn delete_me_requires_correct_display_name() { /* DELETE /me {confirm: "wrong"} → 422 */ }

#[tokio::test]
async fn delete_me_twice_returns_409() { /* DELETE /me once OK, second → 409 */ }

#[tokio::test]
async fn cancel_deletion_without_pending_returns_409() { /* POST /me/cancel-deletion without prior DELETE → 409 */ }
```
Run: `cargo nextest run -p identity --test profile_api` — expected FAIL.

- [ ] **Step 4: Implement `PATCH /me`**

Request struct `UpdateProfileRequest`. If all three fields are `None` → return `IdentityError::AtLeastOneFieldRequired`. Call `service.update_profile(...)`.

- [ ] **Step 5: Implement `POST /me/change-password`**

Request struct `ChangePasswordRequest`. Call `service.change_password(id, &body.current_password, &body.new_password).await?`. Return `StatusCode::NO_CONTENT`.

- [ ] **Step 6: Implement `GET /me/export`**

```rust
#[derive(Serialize)]
struct GdprExportResponse {
    schema_version: u8,   // always 1
    generated_at: String, // RFC 3339
    account: GdprAccountResponse,
    files: Vec<FileExportRow>,
}
```
Call `service.gdpr_export(id, &*state.workspace_export).await?`.
Return with headers `Content-Type: application/json` and `Content-Disposition: attachment; filename="my-data.json"`.

- [ ] **Step 7: Implement `DELETE /me`**

Request `DeleteAccountRequest { confirm_display_name: String }`. Load user via `service.get(id)`. Compare `confirm_display_name` with `user.display_name().as_str()` — exact case-sensitive match. Call `service.schedule_deletion(id).await?`. Return `200 { execute_after: <RFC3339> }`.

- [ ] **Step 8: Implement `POST /me/cancel-deletion`**

Call `service.cancel_deletion(id).await?`. Return `StatusCode::NO_CONTENT`.

- [ ] **Step 9: Add utoipa annotations to all five new handlers**

Each handler gets a `#[utoipa::path(...)]` annotation matching the endpoint spec.

- [ ] **Step 10: Register routes and inject `WorkspaceExportPort` in `apps/api`**

Add the five routes to the router in `routes.rs`.
In composition root (`apps/api`), construct `Arc<PgWorkspaceExportAdapter>` from the workspace pool and pass it into `ApiState`.

- [ ] **Step 11: Regenerate OpenAPI + client**

```bash
just gen-api
```

- [ ] **Step 12: Run all API tests**

Run: `cargo nextest run -p identity --test profile_api`
Expected: all tests PASS.

- [ ] **Step 13: Commit**

```bash
git add modules/identity/core/src/api/routes.rs \
        apps/api/src/ \
        openapi.json \
        packages/api-client/src/schema.d.ts
git commit -m "feat(identity): add profile, password, export and account deletion endpoints"
```

---

## Task 7: Frontend i18n Keys + Error Codes

**Files:**
- Modify: `apps/web/src/shared/i18n/locales/en.ts`
- Modify: `apps/web/src/shared/i18n/locales/tr.ts`

**Interfaces:**
- Produces: `profile.*` namespace and new error codes available to all components

- [ ] **Step 1: Add `profile` namespace to `en.ts`**

```typescript
profile: {
  tab: { profile: 'Profile', security: 'Security', appearance: 'Appearance', dataPrivacy: 'Data & Privacy' },
  avatar: { label: 'Profile picture URL', clear: 'Remove picture' },
  bio: { label: 'Bio', hint: 'Up to 300 characters' },
  emailReadonlyHint: 'Email cannot be changed yet.',
  save: 'Save',
  saved: 'Saved',
  password: {
    current: 'Current password', new: 'New password', confirm: 'Confirm new password',
    changed: 'Password changed.', sessionsRevoked: 'Other sessions have been signed out.',
    mismatch: 'Passwords do not match.', tooShort: 'Password must be at least {min} characters.',
  },
  theme: { label: 'Theme', light: 'Light', dark: 'Dark', system: 'System' },
  lang: { label: 'Language' },
  export: {
    title: 'Download my data',
    description: 'Download all your files and account data as a JSON file.',
    button: 'Download my data',
  },
  gdpr: {
    title: 'Privacy & Legal',
    bodyEn: '...',  // Full GDPR Art. 13 text in English (see spec §3.4)
    bodyTr: '...',  // Full KVKK text in Turkish (see spec §3.4)
  },
  delete: {
    title: 'Delete account',
    description: 'Your account will be permanently deleted 30 days from now. You can cancel during that period.',
    button: 'Delete my account',
    confirmTitle: 'Are you sure you want to delete your account?',
    confirmHint: 'To confirm, type your display name ({displayName}) below.',
    confirmButton: 'Delete',
    scheduledBanner: 'Your account will be deleted on {date}.',
    cancelButton: 'Cancel deletion',
    cancelled: 'Account deletion has been cancelled.',
  },
}
```

- [ ] **Step 2: Add new error codes to `errors` namespace in `en.ts`**

```typescript
deletion_already_scheduled: 'Your account is already scheduled for deletion.',
no_pending_deletion: 'No pending deletion to cancel.',
confirmation_mismatch: 'The name you entered does not match your display name.',
at_least_one_field_required: 'Provide at least one field to update.',
```

- [ ] **Step 3: Mirror all keys in `tr.ts` with Turkish translations**

The GDPR/KVKK body text goes here. For `profile.gdpr.bodyTr`, include the full KVKK Art. 10 notice (controller name placeholder `{appName}`, rights list, contact). For `profile.gdpr.bodyEn`, include GDPR Art. 13 compliant text.

- [ ] **Step 4: Verify TypeScript — `MessageSchema` must match**

Run: `pnpm -C apps/web vue-tsc --noEmit`
Expected: no type errors.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/shared/i18n/
git commit -m "feat(web): add profile and GDPR i18n keys"
```

---

## Task 8: Profile Feature API Layer

**Files:**
- Create: `apps/web/src/features/profile/api/keys.ts`
- Create: `apps/web/src/features/profile/api/use-profile-mutations.ts`
- Create: `apps/web/src/features/profile/index.ts`
- Test: inline in `use-profile-mutations.test.ts` (new, msw)

**Interfaces:**
- Consumes: generated API client `schema.d.ts` (paths `/api/v1/auth/me` PATCH, `/api/v1/auth/me/change-password` POST, etc.)
- Produces (exported from `features/profile/index.ts`):
  - `profileKeys` — query key factory `{ me: () => ['profile', 'me'] }`
  - `useUpdateProfile(): UseMutationReturn<...>` — `mutateAsync({ displayName?, bio?, avatarUrl? })`
  - `useChangePassword(): UseMutationReturn<...>` — `mutateAsync({ currentPassword, newPassword })`
  - `useExportData(): UseMutationReturn<...>` — triggers browser download of the JSON response
  - `useScheduleDeletion(): UseMutationReturn<...>` — `mutateAsync({ confirmDisplayName })` → `{ executeAfter: string }`
  - `useCancelDeletion(): UseMutationReturn<...>`
  - All mutations invalidate `['me']` on success.

- [ ] **Step 1: Write keys.ts**

```typescript
export const profileKeys = {
  me: () => ['profile', 'me'] as const,
}
```

- [ ] **Step 2: Write failing tests for `useUpdateProfile`**

`use-profile-mutations.test.ts` using `msw`:
```typescript
it('calls PATCH /me and invalidates me query', async () => { /* ... */ })
it('triggers download on useExportData success', async () => { /* ... */ })
it('returns execute_after on useScheduleDeletion success', async () => { /* ... */ })
```
Run: `pnpm -C apps/web vitest run features/profile` — expected FAIL.

- [ ] **Step 3: Implement all five mutations in `use-profile-mutations.ts`**

Each wraps the corresponding generated `$fetch` call via `useMutation` (TanStack Query).
`useExportData`: on success, creates `URL.createObjectURL(new Blob([JSON.stringify(data)], {type:'application/json'}))`, clicks a temporary `<a download="my-data.json">`.
All five call `queryClient.invalidateQueries({ queryKey: ['me'] })` on success.

- [ ] **Step 4: Export from `index.ts`**

```typescript
export { profileKeys, useUpdateProfile, useChangePassword, useExportData, useScheduleDeletion, useCancelDeletion } from './api/use-profile-mutations'
```

- [ ] **Step 5: Run tests**

Run: `pnpm -C apps/web vitest run features/profile`
Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/features/profile/
git commit -m "feat(web): add profile feature API layer"
```

---

## Task 9: ProfileTab and SecurityTab Components

**Files:**
- Create: `apps/web/src/features/profile/ui/ProfileTab.vue`
- Create: `apps/web/src/features/profile/ui/SecurityTab.vue`

**Interfaces:**
- Consumes: `useUpdateProfile`, `useChangePassword` from Task 8; `useSessionStore` (has `user.displayName`, `user.avatarUrl`, `user.bio`); `t()` from `vue-i18n`
- Produces: two standalone tab panel components, emitting no events (mutations are internal)

- [ ] **Step 1: Implement `ProfileTab.vue`**

Fields (bound with `ref`): `displayName` (pre-filled from session), `bio` (pre-filled), `avatarUrl` (pre-filled or `''`).
Validation (inline, no library needed): `displayName` non-empty.
Avatar input: `type="url"` + `pattern="https://.*"` for browser hint; also validate in the submit handler.
Submit calls `useUpdateProfile().mutateAsync(...)`. Shows `t('profile.saved')` for 2s on success.
"Remove picture" button sets `avatarUrl` to `null` in the mutation payload.

- [ ] **Step 2: Implement `SecurityTab.vue`**

Three password fields using `TextField` shared component.
Validate: `newPassword === confirmPassword`, `newPassword.length >= 12`.
Shows inline error if validation fails before calling the API.
On mutation success: show `t('profile.password.changed')` + `t('profile.password.sessionsRevoked')`.

- [ ] **Step 3: Export both from `features/profile/index.ts`**

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/features/profile/ui/ProfileTab.vue \
        apps/web/src/features/profile/ui/SecurityTab.vue \
        apps/web/src/features/profile/index.ts
git commit -m "feat(web): add ProfileTab and SecurityTab components"
```

---

## Task 10: AppearanceTab Component

**Files:**
- Create: `apps/web/src/features/profile/ui/AppearanceTab.vue`

**Interfaces:**
- Consumes: `shared/lib/theme.ts` (existing `useTheme` or equivalent); `useI18n` for locale switching; `localStorage['office.theme']`, `localStorage['office.lang']`
- Produces: standalone tab panel, no props needed

- [ ] **Step 1: Implement `AppearanceTab.vue`**

Theme section: three radio buttons (`light`, `dark`, `system`) bound to `localStorage['office.theme']`. On change, dispatch `storage` event or call the existing theme util so `host-prelude.js` reacts.

Language section: two radio buttons (`en`, `tr`). On change: `i18n.locale.value = lang`; `localStorage.setItem('office.lang', lang)`.

Both sections read their initial value from localStorage on mount.

- [ ] **Step 2: Export from index.ts**

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/features/profile/ui/AppearanceTab.vue
git commit -m "feat(web): add AppearanceTab component"
```

---

## Task 11: DataPrivacyTab Component

**Files:**
- Create: `apps/web/src/features/profile/ui/DataPrivacyTab.vue`
- Test: `apps/web/src/features/profile/ui/DataPrivacyTab.test.ts`

**Interfaces:**
- Consumes: `useExportData`, `useScheduleDeletion`, `useCancelDeletion` from Task 8; `useSessionStore` for `user.displayName` and `user.deletionScheduledAt`; `t()` for all strings
- Produces: standalone tab panel with three sections (export, GDPR text, danger zone)

- [ ] **Step 1: Write failing component test**

```typescript
it('keeps Delete button disabled until name matches exactly', async () => {
  // render DataPrivacyTab with user.displayName = "Test User"
  // open confirm dialog, type "test user" (wrong case) → button disabled
  // type "Test User" (exact) → button enabled
})
it('shows scheduled banner when deletionScheduledAt is set', async () => {
  // render with deletionScheduledAt set → assert banner text visible
  // assert "Delete my account" button not rendered
})
```
Run: `pnpm -C apps/web vitest run DataPrivacyTab` — expected FAIL.

- [ ] **Step 2: Implement `DataPrivacyTab.vue`**

**Export section:** button calls `useExportData().mutate()`. Loading state disables the button.

**GDPR section:** `<details>` / collapsible. Body text: `t('profile.gdpr.bodyTr')` when locale is `tr`, else `t('profile.gdpr.bodyEn')`.

**Danger zone:** 
- When `deletionScheduledAt` is null: show `[t('profile.delete.button')]` (red).
- Clicking opens a `AppDialog` with a text field. "Sil" button: `disabled` until `confirmInput === user.displayName` (exact, case-sensitive). On confirm: call `useScheduleDeletion().mutateAsync(...)`.
- When `deletionScheduledAt` is set: show banner `t('profile.delete.scheduledBanner', { date: formatDate(deletionScheduledAt) })` + `[t('profile.delete.cancelButton')]` inline link. Cancel calls `useCancelDeletion().mutate()`.

- [ ] **Step 3: Run component tests**

Run: `pnpm -C apps/web vitest run DataPrivacyTab`
Expected: all PASS.

- [ ] **Step 4: Export from index.ts**

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/features/profile/ui/DataPrivacyTab.vue \
        apps/web/src/features/profile/ui/DataPrivacyTab.test.ts
git commit -m "feat(web): add DataPrivacyTab with GDPR/KVKK compliance and account deletion flow"
```

---

## Task 12: ProfilePage + Route

**Files:**
- Create: `apps/web/src/pages/profile/ui/ProfilePage.vue`
- Create: `apps/web/src/pages/profile/index.ts`
- Modify: `apps/web/src/shared/config/routes.ts`
- Modify: `apps/web/src/app/router.ts`

**Interfaces:**
- Consumes: all four tab components via `features/profile/index.ts`; `useSessionStore` for user header display
- Produces: `/profile` route, auth-guarded

- [ ] **Step 1: Add `Profile` to `RouteName`**

```typescript
Profile: 'profile',
```

- [ ] **Step 2: Add route to `router.ts`**

```typescript
{
  path: '/profile',
  name: RouteName.Profile,
  component: () => import('@/pages/profile'),
},
```
No `meta` — falls into the default auth guard (no `public: true`).

- [ ] **Step 3: Implement `ProfilePage.vue`**

Layout:
- `AppHeader` at top (existing widget).
- Below header: max-w-2xl centred container with:
  - User header row: avatar circle (48 px, initials fallback) + display name + email.
  - Tab bar: four tabs (`Reka UI Tabs` component), labels from `t('profile.tab.*')`.
  - Tab panels render the four feature components.

Active tab stored in `useProfileStore` (Pinia, UI state only): `activeTab: ref<'profile'|'security'|'appearance'|'dataPrivacy'>('profile')`.

- [ ] **Step 4: Create `pages/profile/index.ts`**

```typescript
export { default } from './ui/ProfilePage.vue'
```

- [ ] **Step 5: Verify TypeScript**

Run: `pnpm -C apps/web vue-tsc --noEmit`
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/pages/profile/ \
        apps/web/src/shared/config/routes.ts \
        apps/web/src/app/router.ts
git commit -m "feat(web): add /profile page with four-tab layout"
```

---

## Task 13: AppHeader UserMenu Dropdown

**Files:**
- Modify: `apps/web/src/features/auth/ui/UserMenu.vue`

**Interfaces:**
- Consumes: `RouteName.Profile`; existing `@signed-out` emit; Reka UI `DropdownMenu`

- [ ] **Step 1: Replace the existing UserMenu with a Reka UI DropdownMenu**

Trigger: avatar circle (initials or `<img>` if `user.avatarUrl` set), accessible label `t('auth.accountMenu', { name: user.displayName })`.

Menu items:
1. `t('profile.tab.profile')` → `router.push({ name: RouteName.Profile })`
2. Separator
3. `t('auth.signOut')` → calls existing logout logic → emits `signed-out`

Close on item click. Focus returns to trigger on close.

- [ ] **Step 2: Verify keyboard navigation**

Manually: Tab to avatar → Enter opens menu → arrow keys navigate → Enter activates → Escape closes.

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/features/auth/ui/UserMenu.vue
git commit -m "feat(web): replace UserMenu with accessible dropdown including Profile link"
```

---

## Task 14: Empty States + 404 Redesign

**Files:**
- Modify: `apps/web/src/widgets/recent-files/ui/RecentFiles.vue`
- Modify: `apps/web/src/widgets/shared-files/ui/SharedFiles.vue`
- Modify: `apps/web/src/pages/not-found/ui/NotFoundPage.vue`

**Interfaces:**
- Consumes: existing `t('files.empty')`, `t('files.sharedEmpty')`, `t('notFound.*')` keys

- [ ] **Step 1: My Files empty state**

In `RecentFiles.vue`, when `files.length === 0` and not loading, render:
```html
<div class="flex flex-col items-center gap-4 py-16 text-ink-subtle">
  <!-- inline SVG: simple document stack illustration, ~80px -->
  <p class="text-sm">{{ t('files.empty') }}</p>
</div>
```
Inline SVG: two overlapping rectangles with a subtle fold (no external image).

- [ ] **Step 2: Shared with me empty state**

Same pattern in `SharedFiles.vue` using `t('files.sharedEmpty')`.
SVG: two outlines of a person with a share arrow.

- [ ] **Step 3: Redesign 404 page**

`NotFoundPage.vue`:
```html
<div class="flex min-h-screen flex-col items-center justify-center gap-6 text-center">
  <span class="font-display text-9xl font-bold text-ink-muted/20">404</span>
  <h1 class="text-xl font-semibold">{{ t('notFound.title') }}</h1>
  <AppButton :to="{ name: RouteName.Documents }">{{ t('notFound.home') }}</AppButton>
</div>
```
No new i18n keys needed — `notFound.title` and `notFound.home` already exist.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/widgets/recent-files/ui/RecentFiles.vue \
        apps/web/src/widgets/shared-files/ui/SharedFiles.vue \
        apps/web/src/pages/not-found/ui/NotFoundPage.vue
git commit -m "feat(web): add empty states for file lists and redesign 404 page"
```

---

## Task 15: Full Check + Push

**Files:** none new

- [ ] **Step 1: Run full check**

```bash
just check
```
Expected: `cargo fmt`, `clippy`, `vue-tsc`, `vitest`, `cargo nextest`, `arch`, `gen-api-check` all green.

- [ ] **Step 2: Fix any remaining warnings**

Resolve all `clippy` warnings; fix any `vue-tsc` errors.

- [ ] **Step 3: Verify profile page manually**

Start `just dev-infra && just dev-api && just dev-web`.
- Register, login, navigate to `/profile`.
- Change display name + bio → save → reload → values persist.
- Try setting `http://` avatar URL → expect form-level or API error.
- Change password → sign out → sign in with new password.
- Appearance tab: toggle theme → editor iframes update live.
- Download data → JSON file downloads with `account` and `files` keys.
- Delete account → type wrong name → button disabled; type correct name → confirm → banner appears.
- Cancel deletion → banner disappears.

- [ ] **Step 4: Final commit and push**

```bash
git add -A
git commit -m "chore: final check pass for profile/GDPR/UI feature"
git push origin master
```
