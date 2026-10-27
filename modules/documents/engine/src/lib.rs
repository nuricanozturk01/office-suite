//! Adapter between the `ProseMirror` JSON the web editor produces and the vendored wordcraft
//! engine: `.docx` import and export. Pure functions over byte slices and JSON; no I/O.
//!
//! The vendored crates are never modified (ADR 0001); everything specific to this product, such
//! as the supported node and mark vocabulary, lives here.

#![forbid(unsafe_code)]

mod export;
mod import;
mod merge;

pub use export::{ExportError, export_docx};
pub use import::{ImportError, Imported, MAX_DOCX_BYTES, import_docx};
pub use merge::DocxMerger;
