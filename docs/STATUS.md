# Status / handoff

## Product goal (from the owner, 2026-10-25)

The Docs, Sheets, Slides and PDF products must carry the features of the reference repos `storytold/wordcraft`,
`gridcraft`, `deckcraft` and `pdfcraft` (vendored engines, ADR 0001). Only gridcraft's xlsx adapter is used so far;
wordcraft, deckcraft and pdfcraft are not used yet. User-visible feature parity comes before more infrastructure.

## Update (2026-10-26)

Branch `feat/xlsx-io`, all pushed, `just check` green.

- Docs: `.docx` import/export (`modules/documents/engine`, wordcraft doc+docx; `POST /api/v1/documents/import-docx`, `/export-docx`; web: Upload .docx card, File > Download .docx). Images/shapes are dropped on import (no storage module yet).
- Slides: new `presentations` module (deck JSON, versioned content, sharing via workspace `ResourceKind::Presentation`), `modules/presentations/engine` (deckcraft model+pptx; text/shapes/background/notes only), web Slides editor (SVG canvas, drag/resize, text, shapes, thumbnails, notes, present mode, pptx up/download). Not browser-verified: pointer interactions.
- Next: Docs pdf export + pagination (wordcraft layout/render/pdf need fonts), Slides pictures/tables/charts/transitions/themes, PDF tool (pdfcraft not started), Sheets charts etc.

## RESUME HERE (2026-10-26, end of session)

Branch `feat/xlsx-io`, pushed (`903584a`), `just check` green. `target/` was deleted to free 34 GB: the first cargo
command and `scripts/build-embeds.sh` rebuild from scratch (long). `apps/web/public/apps/*` (built editors) is kept.

Architecture now (ADR 0003, runbook `docs/runbooks/upstream-updates.md`):

- Docs, Sheets, Slides and PDF open the reference editors (wordcraft, gridcraft, deckcraft, pdfcraft web builds) in an iframe,
  rebranded and patched (`scripts/embed-patches/*.patch`): no Discord/ArtCraft/GitHub, no template galleries, no AutoSave switch,
  New/Open/Close/Save As/Share go to the page (`host_command`). Pins in `scripts/build-embeds.sh` (pdfcraft one commit behind: its
  newest commit needs an unpublished `craft-libs` rev).
- Ports and adapters in `apps/web/src/editors/embed` (`EditorSession`, `PostMessageEngine`, `createCloudStore`, `browserDownloader`);
  `entities/package` talks to the new `storage` module (`/api/v1/packages/{id}`); `pdfs` module is the PDF resource.
- Old Vue editors stay behind `?legacy` (Docs ribbon/pagination/find work lives there; no collaboration in the embeds).
- Engine crates in `vendor/` (gridcraft, wordcraft, deckcraft) back the adapters (`modules/*/engine`): docx/xlsx/pptx import and export.

Open items, in order:

1. DEFERRED (2026-10-26, user decision): Repsy publishing of the vendored crates. `vendor/` works and the editors would not benefit. Revisit if repo size, other consumers or a team make it worthwhile.
2. DONE: `scripts/check-upstream.sh [--issue]` compares pins with upstream HEAD (exit 1 when behind). Not yet scheduled in CI.
3. DONE: `scripts/rebrand.sh` (run by build-embeds.sh) renames WordCraft/GridCraft/DeckCraft/PdfCraft to Docs/Sheets/Slides/PDF in string literals and forces the editors to English (navigator.language override). Left: theme named "DeckCraft", sample deck text, file metadata (Application/generator), non-UI crates.
4. Excel "Share" menu is still the app's own export menu; PowerPoint has no dirty signal (no autosave); old imports with Yjs content open blank.
5. UI polish list: dark theme and keyboard pass, Sheets/Slides features from the reference READMEs (charts, pivot, themes, transitions).
6. Language/theme sync between our app and the editors (they follow the browser language).

Dev notes: run `just dev-infra`, `just dev-api` (restart after pulling: new modules `pdfs`, `storage`), `just dev-web`.
New SQL migration needs the manual sqlx bootstrap (docker postgres, apply schema SQL, `just db-prepare`).

## Live sync, new `livesync` module (2026-10-26)

- `modules/livesync` (+ `livesync-contracts`): a WebSocket per open file (`/api/v1/livesync/{resource_id}`, bearer subprotocol like `collaboration`). It tells every connection who else has the file open (name, role, "editing" flag) and pushes a `saved` message when `storage` publishes `PackageSaved` (now carries `by`). Access comes from `workspace_contracts::PermissionChecker`, names from the new `identity_contracts::UserProfiles` port, presence across instances over Redis (`livesync.<resource>` channel, heartbeat 5 s, TTL 15 s). Viewers and commenters cannot flag "editing". Tests: `modules/livesync/core/tests` (presence, saves, access, roles, activity, two instances).
- Web: `apps/web/src/editors/collab` is editor-independent. `LiveCoordinator` (core) talks to a `LiveChannel` (adapter: `WebSocketLiveChannel`) and a `SyncTarget` (`EditorSession` implements it); a new editor or transport only implements one of those. Rules: a clean copy is reloaded when somebody saves; a copy with unsaved edits is left alone and the user chooses "Use their version" or "Keep mine".
- The editor page has no top bar any more (rename, share, download, delete live in My files). Presence shows as a pill over the editor's status bar ("Ann Lee is editing", click for names, roles and "Editing" badges); the footer only appears for sync notices.
- Editor renames reach the file list: the Docs rename dialog and Info > Title (Docs, Sheets, Slides) post `<prefix>:renamed`, the page renames the resource. PDF has no rename in its UI.
- A refresh after somebody else's save keeps the user's place: Docs restores scroll offset, caret and view; Sheets restores sheet, selection and scroll; Slides keeps the slide. (The first open after start-up is not a refresh.)
- Not done yet, by design of this step: characters typed by two people at the same moment are not merged (the second save asks the user to choose), and collaborators' carets are not drawn inside the editors. The planned next steps are a server-side three-way merge on save conflicts (needs package history in `storage`, a merge port in `storage-contracts`, implementations in the `*/engine` crates) and caret positions in the protocol.

## Bulk actions, one theme switch, hosted File menus (2026-10-26)

- "My files": checkbox per row and "select all"; with a selection a bar offers Download, Delete (one confirmation) and Clear (`widgets/recent-files`, `features/file-actions`). There is no bulk endpoint; the web app calls the single-file endpoints. Unit test: `use-selection.test.ts`.
- One theme switch: `scripts/embed-patches/host-prelude.js` (injected into each editor's `index.html` by `scripts/rebrand.sh`) answers `prefers-color-scheme` from `localStorage["office.theme"]`, so the app's switch changes the editors live and survives a reload. The in-editor theme controls are hidden when hosted (Docs options theme picker, Sheets quick button / View tab / File > Options box, Slides preferences checkbox and top-right button and View "Dark Mode").
- Removed when hosted because they are risky or meaningless there: Docs "Encrypt with Password" and the Start-up option, Sheets AutoSave switch and Home button.
- File menus verified in headless Chrome against the real API (downloads and page messages recorded):
  - Docs: Save, Save As (downloads `.docx`), Export to PDF, XML, Word 2003 XML, ODT, RTF, HTML, Markdown, LaTeX, TXT, PNG (all download; ODT/RTF no longer replace the stored document: only `.docx` is stored), Print (builds the hidden PDF iframe that opens the browser print dialog).
  - Sheets: Save, Save As (downloads `.xlsx`), Export to PDF, CSV, HTML, Print.
  - Slides: Save, Save As, Print (downloads a PDF), Export PDF, Handouts (PDF), Change File Type TXT/PNG/deckcraft.
  - PDF: Open (goes to the file list), Save, Save as (downloads and stores), Export to text, image, Word, PowerPoint.
  - Hidden when hosted because they open or create other documents in the editor (the page keeps one file per editor): PDF "New blank PDF", "Create PDF from…", "Combine files", "Open Recent", "Pin folder", "Close all"; Docs "Word document (*.docx)" export (it is the file itself) and "Save these pages as PDF…" (no save dialog on the web); Slides "PowerPoint (*.pptx)" in Change File Type.
  - Not exercised: the browser's own print dialog (headless Chrome shows none), Sheets "Protect Workbook" with a password, the PDF editor's other tools (hundreds); upstream tests cover the engines.
- Patch gotcha: regenerate an editor patch from the repository root (`git diff -- . ':!target' ':!Cargo.lock'` inside `target/embeds/<app>`); from a subfolder the `apps/` hunks silently drop out.
- Dev note: tests drive Chrome on port 9333; a crashed test script leaves Chrome running there, kill it by port before the next run.

## Last edited on top, and verification notes (2026-10-26)

- Saving a file in an editor now moves it to the top of "My files": `storage` writes a `PackageSaved` event to `storage.outbox` in the save transaction (migration `20260101001000_storage_outbox.sql`); `documents`, `spreadsheets`, `presentations` and `pdfs` each consume it (`api/events.rs`) and raise `updated_at` through `platform::touch`. Idempotent (event ledger) and monotonic (`GREATEST`). Tests: `crates/platform/tests/touch.rs`, `modules/storage/core/tests/storage_api.rs`.
- The list endpoints page by id; the web app sorts the loaded pages by `updated_at`. With more than one page of files, a very old file that was just edited may sit on an unloaded page. A server-side `sort=updated` would fix it.
- Dev only: after `scripts/build-embeds.sh`, restart `just dev-web`. Trunk recreates `apps/web/public/apps/<app>` and Vite keeps a stale public-file list, so the editor stays on "Loading...".
- The Zotero tab is hidden in the Docs editor (desktop only; patch in `scripts/embed-patches/wordcraft.patch`).
- Slides: saving a new deck stored a `.deckcraft` copy that the page treated as an export, so nothing reached the cloud, and the editor never reported unsaved changes (no autosave). The deckcraft patch now saves `.pptx` when hosted and posts `deckcraft:dirty`; verified in the browser (2 slides stored, autosave fires). This closes open item 4's "no dirty signal".
- Browser checks done (headless Chrome, real API): Docs (typing, bold, italic, underline, heading, list, centre, page break, autosave, reload, `.docx` round trip), Sheets (values, bold, `=SUM(A1:B1)*2` stored with result 24), Slides (new slide, `.pptx` stored), PDF (upload, render, save). Not checked one by one: the remaining hundreds of ribbon commands; the upstream engine suites cover them (wordcraft 1071/1071, gridcraft 494/495 where the one failure is a Unix file-mode test, deckcraft 458/458, pdfcraft 370/370).
- Upstream engine tests are the evidence for editor commands: wordcraft-engine 1071/1071 pass (one test is flaky when run in parallel).

## Frontend redesign and known gaps (2026-10-26)

Done (web only, not committed: the folder's `.git` disappeared during the session, see the note below):
- New visual identity: cool paper surface, ink text, per-file-type colours (docs cobalt, sheets green, slides amber, pdf red), Bricolage Grotesque + Instrument Sans (`@fontsource-variable`). Tokens in `apps/web/src/app/styles.css`.
- Landing page at `/` (`pages/landing`); signed-in users are redirected to `/documents`. Split auth pages (`shared/ui/AuthShell.vue`).
- File library: left rail (New, Upload, My files / Shared with me), list rows with type, edited date and a menu: Open, Rename, Download, Share, Delete (`features/file-actions`).
- App language defaults to English (`createAppI18n('en')`); `tr` stays available. The product name is one i18n key (`app.name`), still "Office".
- Backend/frontend parity: every OpenAPI operation is called by the web app (checked by script). Missing UX was list-level Rename and Download.

Known gaps:
- No collaboration in the embedded editors (the old Vue editors behind `?legacy` have it).
- Slides (PowerPoint) has no dirty signal, so autosave may not trigger.
- Old imports with Yjs content open blank.
- Sheets "Share" menu is still our own export menu.
- Language and theme are not synced between the app and the editors.
- App rename pending (candidates: Bureau, Deskwork, Paperwork, Clerk, Folio); check trademark and domain first.
- Download of a never-saved blank file says to open and save it once first.
- Not redesigned yet: old Vue editor chrome, share dialog, 404 page, dark theme visual pass.

Repository note: at about 18:41 the old `.git` vanished. A new clone of `git@github.com:nuricanozturk01/office-suite.git` now sits there with one `Initial commit` (`85c45fa`, README only) on `master`; the old history (`main`, feature branches, last good commit `87664e5`) is not in it. The working tree is intact and untracked. Decide how to commit it (one baseline commit or restored history) before pushing.

## Earlier handoff (2026-10-25)

Branch `feat/xlsx-io` (stacked on feat/refresh-tokens <- feat/sharing-ui <- feat/collab-multi-instance <- feat/workspace-sharing
<- feat/event-relay <- feat/vendor-infra <- main; all pushed to origin; none merged to main).
In progress: Sheets `.xlsx` import/export.

- Done (committed as WIP): `vendor/gridcraft` now also has the `xlsx` crate (+ upstream `rustfmt.toml`, so `cargo fmt --all` no longer
  rewrites vendor code; the import script copies it). `modules/spreadsheets/engine` (crate `spreadsheets-engine`): `import_xlsx(bytes)`,
  `export_xlsx(json)`, formula syntax converter (stored `;`/Turkish names <-> canonical English), limits mirroring `WorkbookContent`.
  12 tests pass (`cargo test -p spreadsheets-engine`).
- Done: core `POST /api/v1/spreadsheets/import` (raw body <= 10 MiB, `?name=`) and `GET /{id}/export.xlsx`; web "Upload .xlsx" card on Home and
  File > "Download (.xlsx)". `just check` green. Known gap: import `warnings` are returned by the API but the UI does not show them yet.
- Done: formula engine in WASM (ADR 0002). `just wasm` builds `packages/formula-engine/pkg` (needs `rustup target add wasm32-unknown-unknown` and
  `cargo install wasm-bindgen-cli --version 0.2.129 --locked`). The grid evaluates with gridcraft's 500+ functions and dynamic arrays; verified in
  headless Chrome (errors, spills). Not done: number formats beyond number/percent/currency (use gridcraft numfmt), engine in a Web Worker.
- Left to do, in order (feature parity with the reference repos; gridcraft has 500+ functions, pivot tables, 17 chart types, conditional
  formats, tables, validation, comments, freeze panes, XLSX round-trip):
  1. Sheets features from gridcraft's README/ROADMAP: charts, conditional formatting, freeze panes, row/col insert, tables, pivot, validation.
  2. Docs: wordcraft docx import/export + pdf export + pagination. Slides: deckcraft module + pptx. PDF tool: pdfcraft.
  3. Roadmap in `docs/superpowers/specs/2026-10-25-vendored-engines-cloud-suite-design.md` §5; comments / version history / link sharing / folders.
- Dev notes: run tests via `just` (not with `.env` exported: it rebuilds sqlx crates and bloats `target/`); new SQL migrations need a manual
  bootstrap (apply the .sql to the dev DB with psql, `just db-prepare`, drop the object, `cargo run -p xtask -- migrate` with SQLX_OFFLINE=true);
  headless Chrome at /usr/bin/google-chrome works with the REA `capture_browser_scenario` tool for real-browser checks.

Last updated: 2026-10-24. Nothing is committed yet (`git init` done, no commits).

## Done (`just check` green: Rust + web + arch + OpenAPI drift)

- Phase 0: workspace, toolchain pin (Rust 1.96.0), lints, `xtask arch`, justfile, docker compose (Postgres 18, Redis 8).
- `platform`: settings, db pool + per-schema migrations, RFC 9457 `ProblemDetails`, `AuthUser` extractor
  (Authorization header or `Sec-WebSocket-Protocol: bearer, <token>`).
- `identity`: register / login / me, Argon2id, HS256 JWT, outbox write.
- `documents`: CRUD + cursor pagination, owner-only `AccessPolicy`, `DocumentAccess` contract port, outbox write.
- `collaboration`: Yrs sync over WebSocket at `/api/v1/collab/{document_id}`, append-only
  `document_updates` + snapshot compaction (every 200 updates), read-only vs write sessions.
- OpenAPI: `utoipa` 6 on identity, documents, and spreadsheets handlers; each module exposes `openapi()`,
  merged in `apps/api/src/openapi.rs`, served at `/api/v1/openapi.json`, committed as `/openapi.json`
  (drift test in `apps/api`). WebSocket endpoint is not in the spec.
- `packages/api-client`: `openapi-typescript` + `openapi-fetch`, generated `src/schema.d.ts` committed.
- `apps/web` (Vue 3.5, Vite 8, TS 6.0 strict, Tailwind 4, Pinia, TanStack Query, vue-i18n tr/en):
  login / register, document list (infinite cursor), create / rename / delete, Docs editor
  (Tiptap 3 + `@tiptap/extension-collaboration` + carets) on a custom y-protocols provider
  (`src/editors/docs/core/collab-provider.ts`, reconnect with backoff, stops on close 1008).
  FSD layers enforced with `no-restricted-imports` in `eslint.config.ts`.
  Verified manually in the browser: two sessions editing the same document live, persistence, rename.
- Google Docs-style UI (Reka UI primitives):
  - Home (`/documents`): search box (client-side title filter), blank document and blank spreadsheet,
    recent files (both types) with a per-card actions menu, avatar account menu.
  - Editor shell (`editors/docs/vue/DocsEditor.vue`): title + save status + menu bar (File/Edit/View/
    Insert/Format/Tools, data-driven in `vue/menus.ts`), roving-tabindex toolbar (undo/redo, print, zoom,
    paragraph style, font, size −/+, B/I/U/S, text/highlight color, link, image, alignment, lists,
    checklist, indent, clear formatting), presence avatars, outline sidebar, A4 page canvas, print CSS.
  - Dialogs: insert image (http(s) URL only), insert table, word count; HTML download (`core/export.ts`).
  - Shortcuts: Ctrl/⌘+K link, Ctrl/⌘+Shift+C word count, plus Tiptap's defaults.
- Spreadsheets (`modules/spreadsheets` + `apps/web` grid), verified in the browser:
  - CRUD, cursor list (metadata only), optimistic `version` on `PUT /content` (409 `version_conflict`).
    Rename does not bump `version`. Workbook is a jsonb snapshot (≤20 sheets), not Yrs.
  - Home lists documents and spreadsheets together. Blank spreadsheet card, rename, delete.
  - Grid: virtualized DOM sheet, formula bar, name box, sheet tabs, undo/redo, bold/italic/underline,
    align, text/fill color, number/percent/currency, fill handle, copy/paste, CSV download, selection stats.
  - Formula engine in `editors/sheets/core` (English and Turkish names, `;` argument separator in `tr`).
    Same engine in Rust/WASM is the follow-up; there is no wasm-pack pipeline yet.

## Vendored engines (ADR 0001)

- `vendor/gridcraft`: core, formula, numfmt, functions, model at upstream `4eb2fb5`. Nothing depends on
  them yet; the Sheets adapter (`modules/spreadsheets/engine`) is the next consumer.
- `xtask arch` enforces adapter-only dependencies; `just vendor-check` enforces SHA/licence/crate list;
  `scripts/vendor-import.sh` is the only way to add or update vendored code. Plan:
  `docs/superpowers/plans/2026-10-25-vendor-infrastructure.md`.

## Events and multi-instance (2026-10-25)

- `platform::outbox::Relay`: every instance polls `<schema>.outbox`; a per-table advisory lock lets one work
  at a time (ordered, never duplicated). Handlers dedupe with `platform::outbox::claim`
  (`platform.processed_events`). `platform::broadcast`: Redis pub/sub (`events.<type>`) so every
  instance reacts (e.g. session drop). `platform::background::EventRuntime` starts both; `apps/api`
  runs it with schemas identity, documents, spreadsheets (no handlers registered yet).
- Requires Redis (`REDIS_URL`, default `redis://127.0.0.1:6379`; `just dev-infra` starts it).
- Failing handlers are retried (default 20 attempts, 30 s handler timeout), then parked in
  `platform.outbox_failures` (`parked_at`) so later events keep flowing; there is no replay tool yet.
- Known gaps: broadcast is best-effort and not re-sent after a Redis reconnect; collaboration still has no outbox and no Redis update fan-out.
- Next: `workspace-contracts` + `workspace` ACL module (spec `2026-10-25-sharing-foundation-design.md`),
  then collaboration multi-instance (Redis update fan-out, session drop on `PermissionRevoked`).

## Sharing (2026-10-25)

- `workspace` module: per-file shares by email with roles viewer/commenter/editor, `shared-with-me`, owner-only
  management at `/api/v1/files`. Documents and spreadsheets authorize through `PermissionChecker` (owner full;
  viewer/commenter read; editor writes and renames; delete owner-only). Revocation applies on the next REST request.
- Known gaps: a file shared <0.5 s after creation returns 404 until the relay has
  projected it (frontend should retry once); no link sharing, folders, groups, rate limit on share; identity lookup
  errors surface as `user_not_found`; titles in the read-model are last-writer-wins by wall clock.

## Multi-instance collaboration (2026-10-25)

- Rooms are per-instance caches. Appended updates and awareness go to Redis `collab.<document_id>`
  (`platform::docbus`); a DB catch-up (default 5 s) heals lost messages; duplicates are skipped by seq.
- Compaction folds the durable log under an advisory lock (the old room-memory compaction could delete
  another instance's updates; regression test `compaction_by_one_instance_keeps_updates_of_the_other`).
- `PermissionRevoked` (Redis broadcast, every instance) closes the user's sessions with 1008; open sessions
  also re-check access every 30 s. The web client stops reconnecting on 1008 (reload/re-open to get the new level).
- Gaps: awareness is not persisted; a bus gap is only healed by the catch-up interval; the frontend has no
  sharing UI yet.

## Sharing UI (2026-10-25)

- Home: file card menu has "Share" (dialog: add by email with role, change role, remove) and a
  "Shared with me" section. A 404 right after creating a file is retried 3x (the sharing module learns
  about files from events). Verified by vitest component tests and by a headless-Chrome run against the real API
  (login, reload keeps the session, share by email, the grantee sees the file under "Shared with me" and edits it
  live). No committed Playwright suite yet.
- Not done: Share button inside the editors, role-aware read-only editors (REST responses do not carry the
  caller's role yet), showing the owner's name in "Shared with me", link sharing.

## Disk

- `target/` reached 32 GB (stale per-config copies: SQLX_OFFLINE flips, full debuginfo). `just db-prepare` now
  uses `target/prepare`, `[profile.dev] debug = "line-tables-only"`, and `just gc` drops incremental caches.

## Size baseline (2026-10-25, gzip, `just budget`)

- Initial `index-*.js`: 11.1 KB (limit 150 KB, passes). Lazy chunks: `document` (Tiptap) 199.8 KB,
  `vue-router` 59.8 KB, `routes` 42.6 KB, `auth` 30.0 KB, `spreadsheet` 15.5 KB.
- The Docs chunk is the largest cost; trimming Tiptap extensions is a candidate optimisation.

## Dev setup

```
pnpm install
just dev-infra         # postgres + redis (docker compose)
just dev-api           # .env on first run, migrate, listen on 127.0.0.1:8080
just dev-web           # http://localhost:5173, proxies /api (including WebSocket) to 127.0.0.1:8080
just check             # fmt, clippy, vue-tsc, tests, arch, OpenAPI drift
just gen-api           # after changing any HTTP handler or DTO
```

`just dev-api` writes `.env` when it is missing (`DATABASE_URL`, a random `JWT_SECRET` of 32 bytes,
`API_ADDR`, `SQLX_OFFLINE=true`). The file is gitignored. The API loads it at startup and does not
override variables already exported in the shell. `just jwt` only prints a new `export` line; it does
not edit `.env`. Passwords must be 12–256 bytes.

The justfile exports `SQLX_OFFLINE=true`, so `query!` compiles from the committed `.sqlx` cache and
migrations run when the process starts. Unset that variable and a fresh database fails the build
(`relation "…" does not exist`) before those migrations can create the tables. `just db-prepare`
is the command that checks queries against a live database (`SQLX_OFFLINE=false`).

Only one API can bind `127.0.0.1:8080`. A second `just dev-api` exits with "Address already in use".
If tables exist but `<schema>._sqlx_migrations` does not, startup fails with
`relation "…" already exists`. Recreate the volume
(`docker compose -f docker/compose.yml down -v`, deletes local data) or point `DATABASE_URL` at a new database.

## Decisions / deviations to be aware of

- TypeScript pinned to 6.0.3: TS 7 (native) is not supported by `typescript-eslint` (<6.1) yet.
- Forms: `vee-validate` 4.15 has no zod 4 / Standard Schema support, so `shared/lib/form.ts`
  (`useZodForm`) wraps zod directly. Revisit when vee-validate supports zod 4.
- Access token is kept in memory only; a rotating refresh token (HttpOnly, SameSite=Strict, Path=/api/v1/auth,
  `Secure` when `COOKIE_SECURE=true`) restores the session after a reload and renews the access token at 80% of
  its lifetime. `/auth/refresh` and `/auth/logout` require `x-requested-with: office` (CSRF guard); replaying a
  used refresh token revokes the whole family. Refresh calls are serialized across tabs with Web Locks.
- vue-i18n runs in composition mode via the `__VUE_I18N_LEGACY_API__: false` build flag.
- Reka's `MenubarContent` always refocuses its trigger on close, so menu actions are queued and run
  after that (`vue/chrome/menu-actions.ts`); otherwise dialogs/editor would lose focus.
- `editor.isEditable` is not reactive in `@tiptap/vue-3`; read-only UI state comes from the provider
  status instead. Editor state updates reach Vue after two animation frames (Tiptap's debounce).
- The page is always rendered as white paper (also in dark theme): text/highlight colors are user content.
- Spreadsheet cells are the same white paper. The formula engine runs in the browser; the server stores
  the workbook JSON and does not evaluate formulas.

## Known gaps

- identity: no rate limiting (login/refresh), OIDC, audit log, purge of expired refresh tokens.
- No outbox relay / worker; events only land in `<module>.outbox`.
- collaboration: single instance only (no Redis fan-out), awareness is relay-only,
  no `PermissionRevoked` session drop, no WS rate limit, no per-document size cap.
  Server rejects _any_ update from read-only sessions; the client skips empty step-2 replies, but a
  read-only client holding local deletions would still be disconnected (1008).
- web: no Playwright E2E / axe yet, no stylelint / knip / lefthook / commitlint, no msw,
  no theme toggle (follows `prefers-color-scheme`).
- Docs UI not yet built (need backend modules): comments, suggestion mode, sharing dialog, version
  history, server-side search, real pagination (page breaks), .docx/.pdf export, image upload
  (images are URL-only). Color swatches are announced by hex code, not by name.
- `workspace`, `storage`, `notifications`, `search`, `presentations` modules not started.
- Spreadsheets: no realtime collaboration, charts, freeze panes, row/column insert, xlsx import/export, or print. The grid is 500×40. CSV download only. Formula engine is TypeScript for now (Rust/WASM later). Active sheet tab is not persisted.

## Next steps (in order)

2. Playwright E2E for login + two-user concurrent editing; axe checks; lefthook + commitlint.
3. `workspace` module: sharing / ACL, `PermissionChecker`, `PermissionRevoked` event, then drop sessions.
4. Outbox relay (`apps/worker`), Redis fan-out.
5. Slides, import/export (xlsx/docx). Sheets formula engine as a pure Rust crate compiled to WASM so the client and server share one implementation.

## Paused

- PixPin rewrite (separate project): target `~/Downloads/pixpin.exe` is an Inno Setup 6.7.0 installer that
  innoextract 1.9 / master cannot read. Needs Wine (`sudo apt install wine`) or a portable zip.
  Plan was clean-room Rust + Tauri 2 + Vue in `~/Desktop/pixclone`, no license/activation logic.
