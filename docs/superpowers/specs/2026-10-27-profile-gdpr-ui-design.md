# Profile Page, GDPR/KVKK Compliance & UI Enrichment

**Date:** 2026-10-27  
**Status:** Approved  
**Scope:** `modules/identity` backend extensions + `apps/web` profile feature + app-wide UI polish

---

## 1. Goals

1. Full-featured `/profile` page: display name, bio, avatar, password change, appearance prefs.
2. GDPR / KVKK compliance: data export (JSON ZIP), account deletion with 30-day grace period, cancellation.
3. App-wide UI enrichment: header avatar menu, empty states, 404 redesign.
4. Open-source product — no billing or subscription surface.

---

## 2. Backend

### 2.1 Database Migration

Single migration: `modules/identity/core/migrations/20260101000102_profile_gdpr.sql`

```sql
-- Rollback: ALTER TABLE identity.users DROP COLUMN avatar_url, bio, deletion_scheduled_at.
ALTER TABLE identity.users ADD COLUMN avatar_url           text;
ALTER TABLE identity.users ADD COLUMN bio                  text;
ALTER TABLE identity.users ADD COLUMN deletion_scheduled_at timestamptz;
```

Constraints: `bio` max 300 chars enforced at domain layer. `avatar_url` must be a valid `https://` URL or null; max 500 chars.

### 2.2 Domain Changes (`modules/identity/core/src/domain/user.rs`)

New value objects:
- `Bio` — newtype `String`, 0–300 chars, `Bio::new(s)?`
- `AvatarUrl` — newtype `String`, must start with `https://`, max 500 chars, `AvatarUrl::parse(s)?`

New domain operations on `User`:
- `update_profile(display_name: DisplayName, bio: Option<Bio>, avatar_url: Option<AvatarUrl>) -> ProfileUpdated`
- `change_password(old_hash_matches: bool, new_hash: String) -> Result<(), ValueError>` — caller verifies old password; domain just updates hash
- `schedule_deletion(now: OffsetDateTime) -> UserDeletionScheduled` — sets `deletion_scheduled_at = now + 30 days`; returns event
- `cancel_deletion() -> ()` — clears `deletion_scheduled_at`; only callable when field is Some

### 2.3 New Events (`modules/identity/contracts/src/lib.rs`)

```rust
pub struct ProfileUpdated {
    pub user_id: UserId,
    pub display_name: String,
    pub updated_at: OffsetDateTime,
}

pub struct UserDeletionScheduled {
    pub user_id: UserId,
    pub scheduled_at: OffsetDateTime,        // when the request was made
    pub execute_after: OffsetDateTime,        // scheduled_at + 30 days
}
```

### 2.4 Repository Changes

`UserRepository` trait gains:
- `update_profile(&self, id: UserId, display_name: &DisplayName, bio: Option<&Bio>, avatar_url: Option<&AvatarUrl>, now: OffsetDateTime) -> Result<(), RepositoryError>`
- `update_password(&self, id: UserId, new_hash: &str, now: OffsetDateTime) -> Result<(), RepositoryError>`
- `schedule_deletion(&self, id: UserId, execute_after: OffsetDateTime, now: OffsetDateTime) -> Result<(), RepositoryError>`
- `cancel_deletion(&self, id: UserId) -> Result<(), RepositoryError>`

`UserResponse` DTO gains: `avatar_url: Option<String>`, `bio: Option<String>`, `deletion_scheduled_at: Option<String>` (RFC 3339).

### 2.5 New Endpoints

All require `Authorization: Bearer <token>` and live under `/api/v1/auth`.

| Method | Path | Body | Success |
|--------|------|------|---------|
| `PATCH` | `/me` | `{ display_name?, bio?, avatar_url? }` | `200 UserResponse` |
| `POST` | `/me/change-password` | `{ current_password, new_password }` | `204` |
| `GET` | `/me/export` | — | `200 application/json`, `Content-Disposition: attachment; filename="my-data.json"` |
| `DELETE` | `/me` | `{ confirm_display_name }` — must match exactly | `200 { execute_after: string }` |
| `POST` | `/me/cancel-deletion` | — | `204` |

**PATCH /me rules:**
- All fields optional; at least one must be present (422 otherwise).
- `avatar_url` null clears the avatar.
- Writes `ProfileUpdated` to `identity.outbox` in the same transaction.

**POST /me/change-password rules:**
- Verify `current_password` against stored hash (Argon2id). Return `401 invalid_credentials` on mismatch.
- Hash `new_password` with Argon2id. `new_password` same rules as registration (12–256 bytes).
- Revoke all refresh tokens for the user after success.

**GET /me/export:**
- Collects from identity schema: account row (id, email, display_name, bio, avatar_url, created_at).
- Collects from workspace schema via `WorkspaceExportPort` (new port in `workspace-contracts`): list of files owned by the user with id, name, kind, created_at, updated_at.
- Returns single JSON object (not a ZIP — browser downloads it directly).
- Structurally:
```json
{
  "schema_version": 1,
  "generated_at": "<RFC3339>",
  "account": { "id", "email", "display_name", "bio", "avatar_url", "created_at" },
  "files": [ { "id", "name", "kind", "created_at", "updated_at" } ]
}
```

**DELETE /me rules:**
- `confirm_display_name` must equal the user's `display_name` (case-sensitive). Return `422 confirmation_mismatch` otherwise.
- If `deletion_scheduled_at` already set: return `409 deletion_already_scheduled`.
- Sets `deletion_scheduled_at`; writes `UserDeletionScheduled` to outbox.
- Returns `{ execute_after: "<RFC3339>" }`.
- **Does NOT immediately delete anything.** Real erasure is a future worker job.

**POST /me/cancel-deletion rules:**
- If no pending deletion: return `409 no_pending_deletion`.
- Clears `deletion_scheduled_at`.

### 2.6 WorkspaceExportPort (new, minimal)

In `workspace-contracts/src/lib.rs`:
```rust
#[async_trait::async_trait]
pub trait WorkspaceExportPort: Send + Sync + std::fmt::Debug {
    async fn list_owned_files(&self, owner: UserId) -> Result<Vec<FileExportRow>, ExportError>;
}

pub struct FileExportRow {
    pub id: String,
    pub name: String,
    pub kind: String,          // "document" | "spreadsheet" | "presentation" | "pdf"
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

Implemented in `modules/workspace/core` (reads `workspace.resources`). Injected in `apps/api`.

### 2.7 Error additions to `IdentityError`

```rust
ConfirmationMismatch,
DeletionAlreadyScheduled,
NoPendingDeletion,
AtLeastOneFieldRequired,
```

---

## 3. Frontend

### 3.1 New Route

`/profile` → `pages/profile/ui/ProfilePage.vue`

Router guard: requires auth (same as `/documents`). Redirect to `/login` if unauthenticated.

### 3.2 Feature Structure

```
features/profile/
├── api/
│   ├── keys.ts                  # TanStack Query keys
│   └── use-profile-mutations.ts # updateProfile, changePassword, exportData, deleteAccount, cancelDeletion
├── model/
│   └── use-profile-store.ts     # active tab state (Pinia, UI only)
├── ui/
│   ├── ProfileTab.vue
│   ├── SecurityTab.vue
│   ├── AppearanceTab.vue
│   └── DataPrivacyTab.vue
└── index.ts
```

### 3.3 ProfilePage layout

```
┌─────────────────────────────────────────────────────┐
│ AppHeader (existing)                                │
├─────────────────────────────────────────────────────┤
│  ┌─────┐  Display Name                             │
│  │ AVT │  email@example.com             [Kaydet]   │
│  └─────┘                                           │
│  ────────────────────────────────────────────────  │
│  Profil | Güvenlik | Görünüm | Veri & Gizlilik     │
├─────────────────────────────────────────────────────┤
│  [tab content]                                      │
└─────────────────────────────────────────────────────┘
```

Max width 720 px, centred. Mobile: tabs collapse to a select element or stacked list.

### 3.4 Tab Content

**Profil tab:**
- Avatar: circular img (48px) + URL input below. "Temizle" button sets null. Falls back to initials avatar (existing `user-color.ts` logic).
- Display name: text field (required, 1–100 chars).
- Bio: textarea (max 300 chars, char counter).
- Email: read-only field + "Email değiştirme yakında" note.
- Save button: `PATCH /me`. On success: invalidate `['me']` query + show inline success message.

**Güvenlik tab:**
- Three password fields: current, new, confirm.
- Validation: new ≥ 12 chars, new ≠ current, new === confirm.
- On success: `POST /me/change-password`. Show "Şifre değiştirildi. Diğer oturumlardan çıkış yapıldı."
- Revokes all refresh tokens server-side → web app triggers a re-login after 200.

**Görünüm tab:**
- Theme: three-way radio: Açık / Koyu / Sistem. Writes to `localStorage['office.theme']`; triggers `host-prelude.js` live update.
- Language: radio: Türkçe / English. Writes to `localStorage['office.lang']`; calls `i18n.locale = lang`.
- Both are purely client-side (no backend storage for now).

**Veri & Gizlilik tab:**

Three sections:

1. **Verilerimi İndir**  
   "Hesabınızdaki tüm dosyaları ve kişisel verileri JSON formatında indirin."  
   `[Verilerimi İndir]` button → `GET /me/export` → triggers browser download via `<a download>`.

2. **KVKK / GDPR Bilgilendirmesi**  
   Collapsible panel. Static text:  
   - TR: "6698 sayılı KVKK kapsamında kişisel verileriniz…" (full legal text, 200–400 words covering: data controller, data categories, retention, rights: access/correction/deletion/portability/objection, contact: [app name]@[domain]).  
   - EN: equivalent GDPR notice (Art. 13/14 compliant header).  
   Language follows app locale.

3. **Hesabı Sil (Danger Zone)**  
   Red-bordered box. Text: "Hesabınız 30 gün içinde kalıcı olarak silinecektir. Bu süre içinde giriş yaparak işlemi iptal edebilirsiniz."  
   `[Hesabımı Sil]` → opens confirm dialog:
   - Title: "Hesabınızı silmek istediğinizden emin misiniz?"
   - Body: "Bu işlemi onaylamak için görünen adınızı (**{displayName}**) aşağıya yazın."
   - Text field + `[Sil]` (red, disabled until field matches).
   - On success: show banner at top of page: "Hesabınız {date} tarihinde silinecektir. [İptal Et]"
   - "İptal Et" link: `POST /me/cancel-deletion` → clears banner, shows "Silme işlemi iptal edildi."

   If deletion already scheduled (page load shows `deletion_scheduled_at`): show banner instead of button.

### 3.5 AppHeader Changes

Avatar area (top-right) becomes a `DropdownMenu` (Reka UI):
- Avatar circle (initials or image).
- Menu items: "Profil" → `/profile`, separator, "Çıkış" → logout.

### 3.6 Empty States

`DocumentsPage` when no files:
- My Files empty: illustration + "Henüz belge yok. Yeni oluşturun veya yükleyin."
- Shared with me empty: "Henüz paylaşılan belge yok."

Simple SVG illustrations (inline, no external assets).

### 3.7 404 Page Redesign

`pages/not-found/ui/NotFoundPage.vue`:
- Large "404" number, short message, "Ana Sayfaya Dön" button.
- Follows app theme.

---

## 4. i18n Keys

All user-visible strings use `t('key')`. New namespaces:

```
profile.tab.profile
profile.tab.security
profile.tab.appearance
profile.tab.data_privacy
profile.avatar.label
profile.avatar.clear
profile.bio.label
profile.bio.hint          # "En fazla 300 karakter"
profile.email.readonly_hint
profile.save
profile.saved
profile.password.current
profile.password.new
profile.password.confirm
profile.password.changed
profile.password.sessions_revoked
profile.theme.label
profile.theme.light / dark / system
profile.lang.label
profile.export.title
profile.export.description
profile.export.button
profile.gdpr.title
profile.gdpr.body_tr
profile.gdpr.body_en
profile.delete.title
profile.delete.description
profile.delete.button
profile.delete.confirm_title
profile.delete.confirm_hint
profile.delete.confirm_button
profile.delete.scheduled_banner   # "Hesabınız {date} tarihinde silinecektir."
profile.delete.cancel_button
profile.delete.cancelled
```

---

## 5. Testing

**Backend:**
- Domain unit tests: `update_profile` with invalid bio length, invalid avatar URL, valid combinations.
- `change_password`: wrong current password returns error, valid changes hash.
- `schedule_deletion`: sets date, returns event; calling twice returns `DeletionAlreadyScheduled`.
- `cancel_deletion`: clears date; calling without pending returns `NoPendingDeletion`.
- API integration tests (testcontainers): PATCH /me roundtrip, DELETE /me confirm mismatch, export JSON structure.

**Frontend:**
- Vitest: `use-profile-mutations.ts` with `msw` mocks; confirm-dialog disable logic.
- Component: `DataPrivacyTab.vue` — button renders, banner renders after deletion scheduled.

---

## 6. Security Notes

- Avatar URL: `https://` only (no `data:`, `javascript:`, `http://`). Validated at domain layer AND in OpenAPI schema.
- GDPR export: only returns data for the authenticated user (no query parameter for ID — always uses `AuthUser`).
- Confirm display name check is case-sensitive to prevent trivial bypass.
- `change_password` success revokes all refresh tokens (prevents session fixation after credential change).
- `DELETE /me` does not immediately erase data — grace period allows account recovery and prevents abuse.

---

## 7. Out of Scope (future)

- Email change (requires verification flow).
- Avatar file upload (needs `storage` module).
- Two-factor authentication.
- Active session list.
- Worker job that executes the actual erasure after 30 days.
- OIDC connected accounts.
- Notification preferences.
