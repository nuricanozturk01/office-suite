# 0003. The reference apps are the editors

Status: accepted. Date: 2026-10-26.

## Context

The owner wants Docs, Sheets and Slides (and later PDF) to be the same products as
`storytold/{wordcraft,gridcraft,deckcraft,pdfcraft}`, with every feature. Their browser builds
(`apps/*-web`, eframe + wgpu) already run the full app and ship a `?host=parent` `postMessage`
bridge for pages that embed them and keep the files. Re-implementing their ribbons, layout and
engines in Vue cannot reach parity.

## Decision

- `scripts/build-embeds.sh` clones each repo at a pinned commit, applies
  `scripts/embed-patches/<repo>.patch` if there is one, runs `trunk build --release` and writes
  the static site to `apps/web/public/apps/<repo>` (git-ignored; a deploy step runs the script).
  DeckCraft has no bridge upstream, so `deckcraft.patch` adds the same one (`deckcraft:ready`,
  `open`, `save`).
- `editors/embed` hosts an app in an `<iframe>` (`?host=parent`). It sends the stored file on
  `:ready`, stores every `:save` of a document format and downloads exports (PDF, PNG, ...).
  A resource without a file gets a blank one made by the existing backend converters.
- New `storage` module: `GET|PUT /api/v1/packages/{resource_id}` keeps the native file
  (`.docx`, `.xlsx`, `.pptx`, `.pdf`) in `storage.packages` (optimistic `base_version`). Access
  follows the workspace role (`PermissionChecker`); the resource itself (title, listing, sharing)
  stays in `documents`, `spreadsheets` and `presentations`.
- In host mode the editors also hide their template and sample galleries and their Home page.
- `scripts/embed-patches/*.patch` rebrand the apps for this product: no Discord, ArtCraft or GitHub links or logos (contributor names stay as plain text; the licence notice stays in About), product names "Docs" and "Sheets"/"Slides" in the page title. The host header offers "Save to cloud" (presses Ctrl+S in the app), "Download to computer" (saves, then downloads the same file) and autosaves 2.5 s after the last change (apps that report `:dirty`).
- The pages open the embedded app by default; `?legacy` opens the earlier Vue editors.

## Architecture (ports and adapters)

The editors are libraries; the product never reaches into them. `apps/web/src/editors/embed`:

- `core/ports.ts`: `EditorEngine` (open a file, request a save, events `ready`, `save`, `dirty`,
  `command`, `failed`), `FileStore` (load, blank, save with a base version), `Downloader`.
- `core/session.ts`: `EditorSession`, the use case: loads the document into the editor, stores
  saves, downloads exports and copies, autosaves, routes file commands. No DOM, no HTTP; covered by
  `session.test.ts` with fake ports.
- `adapters/`: `PostMessageEngine` (the editors' `?host=parent` bridge), `createCloudStore`
  (`/api/v1/packages` plus the backend converters for a first file), `browserDownloader`.
- `vue/EmbeddedApp.vue` only wires adapters to a session. `entities/package` owns the HTTP calls.
- The patches in `scripts/embed-patches` add one thing to each editor: a `host_command` hook, so
  New, Close, Save As and Share go to the page instead of acting on a disk.

## Consequences

- Full feature parity with the reference apps, including their layout engines and file formats.
- No real-time co-editing in the embedded editors (their documents are not Yjs documents). Sharing,
  roles and version conflicts work per file. The Vue/Yjs Docs editor remains behind `?legacy`.
- The apps need WebGPU or WebGL2 and download a large WASM bundle on first open.
- Remaining upstream names in menus, translations and the Help dialogs should be reviewed before a public launch (trademarks are not covered by the MIT/Apache licence).
- PDF: `pdfcraft` is pinned one commit before its `craft-menu` switch (that `craft-libs` revision is not published). A PDF is a resource of its own (`pdfs` module: title, owner, sharing; the file lives in `storage`); there is no blank PDF, files come from an upload.
