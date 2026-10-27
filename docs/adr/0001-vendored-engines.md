# 0001. Vendored engine crates

Status: accepted. Date: 2026-10-25.

## Context

The suite needs docx/xlsx/pptx/pdf import-export, a formula engine, charts and PDF editing. The
pure-Rust engines in `storytold/{wordcraft,gridcraft,deckcraft,pdfcraft}` (MIT OR Apache-2.0)
already implement these. Their egui UIs and desktop apps are irrelevant to a cloud product with a
Vue UI, Yjs collaboration, i18n and a11y requirements.

## Decision

- Selected engine crates are copied unmodified into `vendor/<repo>/crates/<name>`. Each `<repo>`
  directory is its own nested Cargo workspace, excluded from the root workspace (`exclude = ["vendor"]`),
  so root `workspace.lints` do not apply and upstream code stays byte-identical.
- Only `modules/<m>/engine` adapter crates depend on vendor crates (by path). Adapters translate
  vendor models to our domain/Yjs models behind ports defined in `-contracts`. Vendor crates never
  depend on our crates. `xtask arch` enforces both.
- The import script drops `[dev-dependencies]` sections from the copied crate manifests (test-only crates such as `deckcraft-engine` pull git dependencies and are never built here); sources stay byte-identical.
- Provenance: `vendor/<repo>/VENDOR.toml` records upstream URL, 40-hex SHA and the copied crates;
  `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE` are kept. `xtask vendor-check` enforces it.
- Not vendored: `ui-egui`, `apps/*`, ArtCraft brand files (trademarked, not open source), fonts.
- `cargo-deny` gates licenses of all transitive dependencies (permissive only).
- Updates are deliberate re-imports at a new SHA via `scripts/vendor-import.sh`.

## Alternatives

- Rewrite from scratch: far slower, no benefit for solved problems.
- Embed the egui WASM apps: contradicts the Vue UI, Yjs collaboration, i18n and a11y rules.
- Git submodules / crates.io dependencies: upstream publishes no stable crates; submodules hide the
  exact code from review and tooling.

## Consequences

- Repo grows by the vendored crates; each sub-project vendors only what it needs, with
  `default-features = false`.
- Adapter layers carry the real integration cost.
- Upstream bug fixes require a re-import; local patches are forbidden (fix upstream or in the adapter).
