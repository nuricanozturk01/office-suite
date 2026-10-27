# Vendor Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put in place the layout, rules and tooling that let pure Rust engine crates from `storytold/{wordcraft,gridcraft,deckcraft,pdfcraft}` be vendored unmodified, enforced by `xtask`, with provenance and size budgets checked in `just check`.

**Architecture:** Each upstream repo becomes its own nested Cargo workspace under `vendor/<repo>/` (excluded from the root workspace, so root `workspace.lints` do not apply and upstream crates stay byte-identical). Only `modules/<m>/engine` adapter crates may depend on vendor crates by path. `xtask` gains `arch` rules, a `vendor-check` provenance task and a `budget` task.

**Tech Stack:** Rust 1.96 / edition 2024 (xtask, existing), `cargo_metadata`, `flate2` (gzip size), `toml` (parse `VENDOR.toml`/`budgets.toml`), `cargo-deny`, bash.

**Spec:** `docs/superpowers/specs/2026-10-25-vendored-engines-cloud-suite-design.md` (sections 2, 4).

## Global Constraints

- CLAUDE.md and code comments/commits in English; commits are Conventional Commits, scope `infra`.
- Vendor code is never edited. The only permitted deviation is the nested root `vendor/<repo>/Cargo.toml` (members list), recorded in `VENDOR.toml`.
- Copy only: selected `crates/<name>`, root `Cargo.toml`, `Cargo.lock` not copied, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`. Never copy `crates/ui-egui`, `apps/*`, `docs/brand/*`, fonts.
- Upstream SHAs (shallow clones are in the scratchpad `ref/` directory; re-clone if gone):
  wordcraft `ea5780d2ab02d284436a952d16a07788f90f03b3`, gridcraft `4eb2fb5c8ed30a580acf24234fffb2cb6256e335`,
  deckcraft `14f40c408c67d2cb43e30a47dd179aada6dba53a`, pdfcraft `71b47b519d2c295c2068dc89a8d75690fc7dd7e5`.
- Allowed licenses (`deny.toml`): MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib, CC0-1.0. Copyleft (GPL/LGPL/MPL) is not allowed.
- `xtask` must keep `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`println!` outside tests; `just check` must stay green.
- Performance budgets are placeholders until first measurement: initial JS ≤ 150 KB gzip, formula WASM ≤ 400 KB gzip.

## Review Focus

- `vendor/` contains a crate with no `VENDOR.toml` entry (or a mismatching SHA) → `vendor-check` fails naming the crate.
- A `core`/`contracts`/`domain` crate depends on a vendor crate directly → `arch` fails.
- A vendor crate depends on one of our workspace crates → `arch` fails.
- Missing `LICENSE-*`/`NOTICE` in a vendored repo directory → `vendor-check` fails.
- `budgets.toml` lists a file that does not exist → `budget` fails (not silently passes).

---

### Task 1: ADR, CLAUDE.md amendment, workspace exclusion, cargo-deny

**Files:**
- Create: `docs/adr/0001-vendored-engines.md`, `deny.toml`, `vendor/README.md`
- Modify: `CLAUDE.md` (§3 repo tree, §4.2 rule 7 list, §12), `Cargo.toml` (`[workspace] exclude = ["vendor"]`), `.gitignore` if needed

**Interfaces:**
- Produces: directory convention `vendor/<repo>/{Cargo.toml,LICENSE-*,NOTICE,VENDOR.toml,crates/*}`; ADR number 0001.

- [ ] **Step 1: Write ADR 0001** (Context, Decision, Alternatives, Consequences) with: nested workspaces per upstream repo, unmodified crates, adapter-only dependency rule, lints exemption, SHA pinning, brand/fonts excluded, `cargo-deny` license gate. Alternatives: rewrite from scratch; embed egui WASM apps (rejected: no Vue UI/Yjs/i18n/a11y).
- [ ] **Step 2: Amend CLAUDE.md first** (it states changes to rules precede code): add `vendor/` to §3 tree, add the vendor rules to §4.2 rule 7, add `just vendor-check` and `just budget` to §12.
- [ ] **Step 3:** Add `exclude = ["vendor"]` to root `[workspace]`. Write `deny.toml` with the license allowlist above, `[advisories] vulnerability = "deny"` equivalent for the installed cargo-deny schema, `[bans] multiple-versions = "warn"`, `[sources]` crates.io only. Write `vendor/README.md` describing layout and the re-import procedure.
- [ ] **Step 4:** Run `cargo metadata --format-version 1 >/dev/null` — Expected: success. Run `cargo install cargo-deny --locked` if `cargo deny --version` fails, then `cargo deny check licenses` — Expected: `licenses ok`.
- [ ] **Step 5: Commit** `chore(infra): add vendoring ADR, cargo-deny config and workspace exclusion`

### Task 2: `xtask arch` vendor rules (TDD)

**Files:**
- Modify: `xtask/src/main.rs` (split rule logic into `xtask/src/arch.rs`, keep `main.rs` as dispatcher), `xtask/Cargo.toml` (no new deps)
- Test: unit tests in `xtask/src/arch.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces: `pub(crate) struct PackageInfo { name: String, manifest_path: String, deps: Vec<String> }` and `pub(crate) fn vendor_violations(pkgs: &[PackageInfo]) -> Vec<String>`; `check()` builds `PackageInfo`s from `cargo_metadata` for workspace members **plus** path packages under `/vendor/`.
- Rules: a package under `/vendor/` is "vendor"; a package under `/modules/<m>/engine/` is "adapter". (1) Non-adapter, non-vendor package depending on a vendor package → violation `"{pkg}: depends on vendor crate `{dep}` (only modules/*/engine may)"`. (2) Vendor package depending on a non-vendor workspace package → violation `"{vendor}: depends on workspace crate `{dep}`"`.

- [ ] **Step 1:** Write failing tests: `adapter_may_use_vendor`, `core_using_vendor_is_violation` (asserts the exact message above), `vendor_using_workspace_crate_is_violation`, `non_vendor_without_vendor_deps_is_clean`.
- [ ] **Step 2:** Run `cargo test -p xtask` — Expected: FAIL (items undefined).
- [ ] **Step 3:** Move existing `module_of`/`check` into `arch.rs`, implement `vendor_violations` as a pure function over `PackageInfo`, call it from `check()` and merge messages with existing ones. Keep existing behavior byte-identical.
- [ ] **Step 4:** Run `cargo test -p xtask && cargo run -q -p xtask -- arch` — Expected: tests PASS, arch exits 0.
- [ ] **Step 5: Commit** `feat(infra): enforce vendor dependency rules in xtask arch`

### Task 3: `vendor-check` provenance task and `vendor-import` script (TDD)

**Files:**
- Create: `xtask/src/vendor.rs`, `scripts/vendor-import.sh`
- Modify: `xtask/src/main.rs` (dispatch `vendor-check`), `xtask/Cargo.toml` (add `toml`, `serde` via workspace deps), `Cargo.toml` (`[workspace.dependencies] toml = ...` — verify current version with `cargo search toml`), `justfile` (`vendor-check`, and add it to `check`)
- Test: unit tests in `xtask/src/vendor.rs`

**Interfaces:**
- `VENDOR.toml` schema (per repo dir): `upstream = "https://github.com/storytold/gridcraft"`, `sha = "<40 hex>"`, `crates = ["core", ...]`, `note = "root Cargo.toml members rewritten to crates/*"`.
- Produces: `pub(crate) fn vendor_check(root: &Path) -> Vec<String>` returning violations: missing `VENDOR.toml`, bad SHA (not 40 hex), missing `LICENSE-MIT`/`LICENSE-APACHE`/`NOTICE`, `crates/<x>` present on disk but not listed (or listed but absent), forbidden dir present (`crates/ui-egui`, `apps`, `docs/brand`).
- `scripts/vendor-import.sh <repo> <sha> <crate>...`: clones `https://github.com/storytold/<repo>` at `<sha>` into a temp dir (refuses if HEAD != sha), copies root `Cargo.toml`, licenses, `NOTICE`, listed crates plus their intra-repo path dependencies reported (script fails listing any missing transitive internal crate), rewrites root `members` to `["crates/*"]` and drops `default-members`, writes `VENDOR.toml`.

- [ ] **Step 1:** Write failing tests building temp dirs: `clean_vendor_dir_passes`, `missing_notice_fails`, `unlisted_crate_fails`, `ui_egui_present_fails`, `bad_sha_fails` (each asserts the message names the crate/file).
- [ ] **Step 2:** Run `cargo test -p xtask vendor` — Expected: FAIL.
- [ ] **Step 3:** Implement `vendor_check` and dispatch; empty/nonexistent `vendor/` → no violations.
- [ ] **Step 4:** Implement `scripts/vendor-import.sh` (`set -euo pipefail`, `shellcheck`-clean). Dry-run it with `scripts/vendor-import.sh gridcraft 4eb2fb5c8ed30a580acf24234fffb2cb6256e335 core` into a temp `VENDOR_DEST` override (env var, default `vendor`) — Expected: copies `core` and fails nothing.
- [ ] **Step 5:** Run `cargo test -p xtask && just vendor-check` — Expected: PASS.
- [ ] **Step 6: Commit** `feat(infra): add vendor provenance check and import script`

### Task 4: First import — gridcraft formula stack (proves the pipeline)

**Files:**
- Create: `vendor/gridcraft/**` via the script: crates `core`, `formula`, `numfmt`, `functions`, `model` (+ any intra-repo crates the script reports as needed, e.g. `calc` only if `functions`/`model` require it)
- Modify: `NOTICE` (new root file listing bundled third-party code and its licenses), `docs/STATUS.md` (note vendor status)

**Interfaces:**
- Consumes: `scripts/vendor-import.sh`, `xtask vendor-check`, `xtask arch`.
- Produces: path-addressable crates `vendor/gridcraft/crates/{core,formula,numfmt,functions,model}` for the Sheets sub-project; nothing in our workspace depends on them yet.

- [ ] **Step 1:** Run the import with the pinned gridcraft SHA and the five crates; fix the crate list only by adding crates the script reports as missing internal dependencies.
- [ ] **Step 2:** Run `cargo check --manifest-path vendor/gridcraft/Cargo.toml` — Expected: success (proves nested workspace resolves with the trimmed members list; if `workspace.dependencies` path entries for absent crates break resolution, record the minimal root-manifest edit in `VENDOR.toml` `note`).
- [ ] **Step 3:** Run `cargo test --manifest-path vendor/gridcraft/Cargo.toml -p gridcraft-formula` — Expected: upstream tests PASS (confirms unmodified code behaves).
- [ ] **Step 4:** Run `cargo deny check licenses` and `just vendor-check arch` — Expected: both succeed. Check `git diff --stat` has no files under `crates/*/src` changed vs upstream (`diff -r` against the scratchpad clone).
- [ ] **Step 5:** Add `NOTICE` at repo root stating the vendored repos, SHAs and `MIT OR Apache-2.0`, no ArtCraft marks.
- [ ] **Step 6: Commit** `chore(infra): vendor gridcraft formula stack at 4eb2fb5`

### Task 5: Size budget task

**Files:**
- Create: `xtask/src/budget.rs`, `budgets.toml`
- Modify: `xtask/src/main.rs`, `xtask/Cargo.toml` (`flate2` — verify version with `cargo search flate2`, `default-features = false`, `rust_backend`), `Cargo.toml` workspace deps, `justfile` (`budget` recipe; NOT part of `check`, since it needs a web build; document in `docs/STATUS.md`)
- Test: unit tests in `xtask/src/budget.rs`

**Interfaces:**
- `budgets.toml`: `[[budget]] name = "initial-js"`, `glob = "apps/web/dist/assets/index-*.js"`, `max_gzip_bytes = 153600`; second entry `name = "formula-wasm"`, `glob = "apps/web/dist/assets/*formula*.wasm"`, `max_gzip_bytes = 409600`, `optional = true` (absent until the WASM exists).
- Produces: `pub(crate) fn budget_violations(root: &Path, budgets: &[Budget]) -> Vec<String>`; message format `"{name}: {file} is {actual} B gzip, limit {max} B"`; a non-optional budget matching no file → `"{name}: no file matches {glob}"`.

- [ ] **Step 1:** Write failing tests with temp files: `under_limit_passes`, `over_limit_fails_with_exact_message`, `missing_required_glob_fails`, `missing_optional_glob_is_ignored`.
- [ ] **Step 2:** Run `cargo test -p xtask budget` — Expected: FAIL.
- [ ] **Step 3:** Implement gzip-size measurement (`flate2::write::GzEncoder`, level 9, count output bytes) and glob expansion without a new crate if `std::fs` walk suffices; otherwise add `glob` with justification in the commit body.
- [ ] **Step 4:** Run `cargo test -p xtask` — Expected: PASS. Run `pnpm --filter web build && just budget` — Expected: initial JS measured and reported; adjust only the placeholder limit in `budgets.toml` if the baseline already exceeds it, and record the measured baseline in `docs/STATUS.md` (do not hide a failure by raising the limit without writing the number down).
- [ ] **Step 5:** Run full `just check` — Expected: all green.
- [ ] **Step 6: Commit** `feat(infra): add gzip size budget check`

---

## Out of scope (own plans later, in spec order)

2 Workspace/sharing, 3 Sheets (formula WASM + xlsx), 4 Docs (docx/pdf), 5 Slides, 6 PDF tool.
