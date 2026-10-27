# vendor/

Unmodified engine crates from `storytold/{wordcraft,gridcraft,deckcraft,pdfcraft}` (see
`docs/adr/0001-vendored-engines.md`). Each subdirectory is a nested Cargo workspace excluded from the
root workspace.

Rules: never edit files under `crates/`; only `modules/<m>/engine` may depend on these crates; every
subdirectory has `VENDOR.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`.

Import or update: `scripts/vendor-import.sh <repo> <sha> <crate>...`, then `just vendor-check arch`.
