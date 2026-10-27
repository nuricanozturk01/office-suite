# Updating the reference repositories

The product builds on four open-source repositories (`storytold/gridcraft`, `wordcraft`,
`deckcraft`, `pdfcraft`, MIT OR Apache-2.0). Nothing follows them automatically: every
use is pinned to a commit, and an update is a deliberate change reviewed like any other.
Background: ADR 0001 (vendored engines) and ADR 0003 (embedded editors).

## What is pinned where

| Channel                                  | Used for                                                                                 | Pin                                      | Update command             |
| ---------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------- |
| `vendor/<repo>/` (engine crates)         | `.docx`, `.xlsx`, `.pptx` import/export, formula engine (adapters in `modules/*/engine`) | `sha` in `vendor/<repo>/VENDOR.toml`     | `scripts/vendor-import.sh` |
| `apps/web/public/apps/<repo>/` (editors) | Word, Excel, PowerPoint and PDF editors in the browser                                   | `SHA` table in `scripts/build-embeds.sh` | `scripts/build-embeds.sh`  |

The pins are independent. Updating an editor does not update the engine crates, and the
other way round.

## Quick path: one command

```sh
scripts/check-upstream.sh                          # what moved?
scripts/upgrade-upstream.sh gridcraft --dry-run    # upstream log since our pin, no changes
scripts/upgrade-upstream.sh gridcraft              # upgrade to upstream HEAD (or pass a full sha)
```

`upgrade-upstream.sh <repo> [sha]` does steps 2 and 3 below for one repo: re-imports `vendor/<repo>`
(crate list from `VENDOR.toml`) and updates the root `NOTICE`, moves the editor pin in
`build-embeds.sh`, re-applies `scripts/embed-patches/<repo>.patch` (rebasing it with a 3-way merge if
it no longer applies), builds the editor with trunk, then runs `just vendor-check` and the adapter
tests of that engine (`--full` runs `just check`). Options: `--skip-vendor`, `--skip-embed`,
`--no-verify`. It never commits.

What can go wrong, and what the script does:

| Situation                                  | Result                                                                                                         |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Everything applies and builds              | Pins moved, files updated. Do the manual check below, then commit.                                             |
| Vendor import, trunk build or tests fail   | The script puts `build-embeds.sh`, `NOTICE`, the patch and `vendor/<repo>` back; a copy is in `target/upgrade/`. |
| The patch conflicts                        | Pins stay moved, the conflicted tree is left in `target/embeds/<repo>`. Resolve it, regenerate the patch (the script prints the command), run `scripts/build-embeds.sh <repo>`. |
| Adapter tests fail (`modules/*/engine`)    | Upstream changed its model. Fix the adapter (and the mergers in the same crates), never `vendor/`.             |
| pdfcraft does not build at upstream HEAD   | Its newest commits can need an unpublished `craft-libs`. Pass an older sha, or wait for a release.            |

Riskiest patches when upstream refactors: the keep-the-view code (Docs `canvas.rs`, Sheets `Place`,
Slides `opened_once`), the rename report (`on_renamed`) and the PDF `hosted_hidden` filters. The
rebrand and the host bridge are shallow. Keep the patch surface small.

Manual check after every editor upgrade (nothing replaces it): open a file; edit and see "Saved";
rename in the editor; Share opens our dialog; Save/Export download; save from a second browser and
see the first one refresh without losing its place; File > Home returns to the list. After
`build-embeds.sh`, restart `just dev-web` (a stale file list leaves the editor on "Loading…").

## 1. Check for changes

```sh
scripts/check-upstream.sh          # prints pins vs upstream HEAD, exit 1 when behind
scripts/check-upstream.sh --issue  # same, and opens a GitHub issue (needs gh); for a weekly CI job
```

Read the upstream
commit list (`git log <old>..<new>`) before moving a pin: look for breaking changes to file
formats, the `?host=parent` bridge messages and crate APIs.

## 2. Update an engine crate set (`vendor/`)

```sh
git switch -c chore/vendor-<repo>-<short-sha>
scripts/vendor-import.sh <repo> <new-full-sha> <crate>...   # same crate list as VENDOR.toml
just vendor-check arch
just check
```

- The script replaces `vendor/<repo>`, checks licences and sources with `cargo deny`, and rewrites
  `VENDOR.toml`. Vendored sources are never edited by hand (ADR 0001).
- It removes `[dev-dependencies]` from the copied manifests (test-only crates pull git
  dependencies we do not build).
- `just check` runs the adapter tests (`modules/*/engine/tests`: docx, xlsx and pptx round trips).
  A failure there is how an upstream model change shows up; fix it in the adapter, not in `vendor/`.
- Update `NOTICE` with the new SHA (`just vendor-check` fails otherwise).

## 3. Update an embedded editor (`scripts/build-embeds.sh`)

```sh
git switch -c chore/embed-<repo>-<short-sha>
# edit the SHA table in scripts/build-embeds.sh
scripts/build-embeds.sh <repo>
```

The script clones the repository into `target/embeds/<repo>`, checks out the pinned commit,
applies `scripts/embed-patches/<repo>.patch`, runs `trunk build --release` and writes the result
to `apps/web/public/apps/<repo>` (git-ignored). Needs `trunk` and the `wasm32-unknown-unknown`
target.

Our patches change the upstream editors in these ways (see ADR 0003; the list grew, keep it short):

1. **Rebrand**: no Discord, ArtCraft or GitHub links, logos or contributor links; product names.
2. **Hosted mode** (`?host=parent`): a `host_command` hook so New, Open, Close, Save As and Share go
   to the page, no AutoSave switch, no Home/New template galleries. DeckCraft also gets its whole
   `postMessage` bridge from the patch.
3. **Host features**: theme from the app, rename and dirty reports, keep-view-on-refresh, save/export
   routing, Share buttons, hidden desktop-only commands.
4. Nothing else. Keep patches small: every extra line is a future merge conflict.

### When `git apply` fails

The upstream code around a patched line changed. Rebase the patch:

```sh
cd target/embeds/<repo>
git checkout -f <new-sha> && git clean -fd -e target
git apply --3way ../../../scripts/embed-patches/<repo>.patch   # resolve conflicts by hand
(cd apps/<repo>-web && cargo check --target wasm32-unknown-unknown)
# Always from the repository root of the clone: from a subfolder the apps/ hunks drop out.
git add -N . && git diff HEAD -- . ':!target' ':!Cargo.lock' > ../../../scripts/embed-patches/<repo>.patch
```

### Verify an editor update

Start `just dev-api` and `just dev-web`, then for each product:

1. Open a file from "Recent files": the saved content appears.
2. Edit, wait 3 s: status shows "Saved" (autosave). Press "Save to cloud": the package version rises.
3. "Download to computer" downloads the file; it opens in the desktop program.
4. File > New creates a new cloud document; File > Open and Close go back to the file list.
5. No Discord, ArtCraft or GitHub link anywhere (File menu, About, Help, title bar).

Unit tests for the host protocol: `pnpm --filter @office/web test` (`editors/embed`).

### pdfcraft

pdfcraft at its newest commit needs a `craft-libs` revision that is not published, so its pin is
the last commit before that dependency. Try the newest `main` after each upstream release:
`cd target/embeds/pdfcraft && git checkout origin/main && (cd apps/pdfcraft-web && cargo check --target wasm32-unknown-unknown)`.
If it builds, move the pin and rebuild the patch.

## 4. Release notes

Commit as `chore(vendor): update <repo> to <short-sha>` or `chore(web): update <repo> editor to
<short-sha>`. Put the upstream range (`<old>..<new>`) and anything the adapters or patches had to
change in the commit body. Deployments run `scripts/build-embeds.sh` as a build step; the built
editors are not committed.

## Later: a registry for the engine crates

Publishing the vendored crates under our own names to a private cargo registry (Repsy) would
replace the `vendor/` directory with versioned dependencies; step 2 then becomes "publish a new
version, bump it in `[workspace.dependencies]`". The patched editors stay a build-script concern.
Needs: registry URL, a publish token (stored in `~/.cargo/credentials.toml`, never in the repo) and
a crate name prefix.
