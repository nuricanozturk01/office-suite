# Cloud Office Suite: Vendored Engines and Roadmap — Design

Status: draft for review. Date: 2026-10-25.

## 1. Intent

A Google Workspace-style **cloud** office suite (Docs, Sheets, Slides, plus a PDF tool) in the
browser with real-time collaboration. Source of truth is the server; the client is thin.

Success criteria:
- Open, edit and save `.docx`, `.xlsx`, `.pptx`, `.pdf`; most Google Docs/Sheets/Slides features.
- Professional, readable code that is an architectural reference (CLAUDE.md rules hold).
- Ultra performant, minimal resource use, enforced by budgets measured in CI.

Reference engines (pure Rust, `MIT OR Apache-2.0`): `storytold/wordcraft`, `gridcraft`,
`deckcraft`, `pdfcraft`. Their egui UIs and desktop apps are **not** used.

## 2. Vendoring rules

- Pure engine crates only are copied into `vendor/<repo>/<crate>`, unmodified, with the
  upstream `LICENSE-*`/`NOTICE`. The commit message records the upstream commit SHA.
- Not copied: `ui-egui`, `apps/*`, ArtCraft brand files (trademarked, not open source), fonts
  (upstream keeps them in a separate repo; we source fonts separately, license-checked).
- Each crate is taken with `default-features = false`; only crates a sub-project needs are added.
- `vendor/*` is excluded from `workspace.lints`. Only a `modules/<m>/engine` adapter crate may
  depend on a vendor crate. `xtask arch` enforces this; recorded in ADR 0001.
- `cargo-deny` license allowlist covers all vendor transitive deps; a failing license blocks the import.
- Our code never edits vendor code. Updates are re-imports at a new upstream SHA.

## 3. Architecture

- Existing modular monolith stays: `<module>-contracts` + `<module>` core; new
  `modules/<m>/engine` adapter crates wrap vendor types behind ports defined in contracts.
  Domain stays pure; adapters translate vendor models to our domain/Yjs models.
- Heavy work (import/export of docx/xlsx/pptx/pdf) runs only in `apps/worker`, fed by a Postgres
  job queue (`FOR UPDATE SKIP LOCKED`; no extra broker). Each job has CPU/memory/time limits,
  zip-bomb and XXE protection, and runs on `spawn_blocking`/a bounded pool.
- Files live in S3-compatible storage; downloads use short-lived signed URLs. Redis is used only
  for update fan-out and presence.
- Formula engine (`formula`, `functions`, `numfmt`, `core`, `model` from gridcraft) compiles to
  WASM, runs in a Web Worker on the client and is reused server-side, replacing the TS engine
  in `apps/web/src/editors/sheets/core`.

## 3a. Multi-instance requirement (binding for every sub-project)

The API and worker run as N identical, stateless instances behind a load balancer; no sticky
sessions. Consequences: no per-instance authoritative state (in-memory Yrs rooms are caches fed
by Postgres + Redis fan-out), outbox relaying and background jobs use `FOR UPDATE SKIP LOCKED`
or advisory locks, event consumers are idempotent (dedupe table), cross-instance reactions
(session drop on `PermissionRevoked`, update fan-out, presence) go through Redis pub/sub, and
migrations are safe to run concurrently. Every sub-project that adds runtime state ships an
integration test with two app instances sharing one Postgres and one Redis.

## 4. Performance and resource budgets

Targets, to be fixed by ADR after the first measurement, and checked in CI:
- Initial JS ≤ 150 KB gzip; editor chunks and WASM lazy-loaded per route.
- Sheets formula WASM ≤ 400 KB gzip.
- Server memory per open WS session is constant and small; load test (1000 concurrent sessions)
  reports RSS per session; `criterion` benchmarks for hot paths (sync, formula recalculation,
  import).
- Zero-copy where practical (`Bytes`, `Arc<str>`), batched delta Yjs updates, virtualized grids.

## 5. Sub-projects (in order)

Each gets its own plan, ADR(s), small PRs and `just check` green.

1. **Vendor infrastructure:** `vendor/` layout, ADR 0001, `xtask arch` rule, license/NOTICE checks,
   size/benchmark harness.
2. **Workspace:** sharing/ACL, `PermissionChecker`, `PermissionRevoked` + session drop, comments,
   suggestion mode, version history, storage module, refresh token + CSRF, outbox relay in worker,
   Redis fan-out.
3. **Sheets:** formula WASM, then `xlsx` import/export (worker), then charts, freeze panes,
   insert row/column, filters/sort, realtime collaboration (ADR on the CRDT model).
4. **Docs:** `docx` import/export and PDF export via `doc`/`docx`/`layout`/`pdf`; pagination;
   adapter between vendor doc model and the ProseMirror schema.
5. **Slides:** new `presentations` module, Yjs scene model, `pptx` import/export, present mode.
6. **PDF tool:** `cos`/`render`/`edit`/`forms`/`annot`/`organize`/`redact`; WASM viewer, server-side edits.

## 6. Security

Import files are untrusted (limits, sandboxed worker, no macros/OLE execution). Every endpoint
checks authorization explicitly (IDOR tests). Audit log for share/permission/delete/export.
Formula functions with network access stay disabled unless allowlisted with SSRF protection.

## 7. Testing

Domain unit tests; adapter round-trip tests (vendor ↔ our model, `insta` snapshots);
`proptest` for formula and CRDT logic; testcontainers integration; Playwright two-user
editing E2E and axe checks; benchmark regression in CI.

## 8. Risks and open decisions

- Adapter layers (Yjs ↔ vendor models) are the real effort, not copying.
- Upstream APIs may change; we pin SHAs and re-import deliberately.
- Open ADRs: Sheets CRDT model, Slides scene model (SVG vs Canvas), tenancy model, final
  performance budgets, font sourcing.
