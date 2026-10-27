use serde_json::Value as Json;

use crate::model::workbook_from_json;

/// Why a workbook could not be exported.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The workbook JSON does not have the expected shape.
    #[error("workbook document is invalid")]
    Invalid,
    /// The file writer failed.
    #[error("could not write the xlsx file: {0}")]
    Write(String),
}

/// Writes the workbook document as an `.xlsx` package.
///
/// # Errors
/// [`ExportError`] when the document is malformed or the writer fails.
pub fn export_xlsx(workbook: &Json) -> Result<Vec<u8>, ExportError> {
    let book = workbook_from_json(workbook).map_err(|_| ExportError::Invalid)?;
    gridcraft_xlsx::write_xlsx(&book).map_err(|e| ExportError::Write(e.to_string()))
}
