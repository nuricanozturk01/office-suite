//! Limits of the workbook document (mirrors `spreadsheets` core's `WorkbookContent`).

#[cfg(feature = "xlsx")]
pub(crate) const MAX_SHEETS: usize = 20;
#[cfg(feature = "xlsx")]
pub(crate) const MAX_CELLS: usize = 20_000;
pub(crate) const MAX_ROW: u32 = 5_000;
pub(crate) const MAX_COL: u32 = 200;
#[cfg(feature = "xlsx")]
pub(crate) const MAX_NAME_CHARS: usize = 100;
pub(crate) const MAX_INPUT_CHARS: usize = 8_000;
#[cfg(feature = "xlsx")]
pub(crate) const MIN_COL_WIDTH: f32 = 32.0;
#[cfg(feature = "xlsx")]
pub(crate) const MAX_COL_WIDTH: f32 = 800.0;
#[cfg(feature = "xlsx")]
/// Upload size accepted by [`crate::import_xlsx`].
pub const MAX_XLSX_BYTES: usize = 10 * 1024 * 1024;
