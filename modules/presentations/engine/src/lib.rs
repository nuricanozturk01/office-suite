//! Adapter between the deck JSON stored by `presentations` and the vendored deckcraft engine:
//! `.pptx` import and export. Pure functions over byte slices and JSON; no I/O.
//!
//! The vendored crates are never modified (ADR 0001); the deck vocabulary of the web slide
//! editor lives here.

#![forbid(unsafe_code)]

mod export;
mod import;
mod merge;

pub use export::{ExportError, export_pptx};
pub use import::{ImportError, Imported, MAX_PPTX_BYTES, import_pptx};
pub use merge::PptxMerger;
