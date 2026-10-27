//! Adapter between the workbook JSON stored by `spreadsheets` and the vendored gridcraft
//! engine: `.xlsx` import and export, and the calculation session behind the web grid. Pure
//! functions over byte slices and JSON; no I/O.
//!
//! The vendored crates are never modified (ADR 0001): everything specific to this product, such
//! as the document limits and the stored formula syntax, lives here.

#![forbid(unsafe_code)]

#[cfg(feature = "xlsx")]
mod export;
#[cfg(feature = "xlsx")]
mod import;
mod limits;
#[cfg(feature = "xlsx")]
mod merge;
mod model;
mod session;
mod syntax;

#[cfg(feature = "xlsx")]
pub use export::{ExportError, export_xlsx};
#[cfg(feature = "xlsx")]
pub use import::{ImportError, Imported, import_xlsx};
#[cfg(feature = "xlsx")]
pub use limits::MAX_XLSX_BYTES;
#[cfg(feature = "xlsx")]
pub use merge::XlsxMerger;
pub use model::{ModelError, workbook_from_json};
pub use session::{CellValue, Edit, EditError, Session};
