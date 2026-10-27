use documents_engine::{ExportError, ImportError};
use serde_json::Value;

use super::DocumentError;

/// Parses an untrusted `.docx` upload into a `ProseMirror` document plus import warnings. The
/// parse is CPU bound, so it runs on the blocking pool.
pub(crate) async fn import_docx(bytes: Vec<u8>) -> Result<(Value, Vec<String>), DocumentError> {
    let imported = tokio::task::spawn_blocking(move || documents_engine::import_docx(&bytes))
        .await
        .map_err(|_| DocumentError::Internal)?
        .map_err(|err| match err {
            ImportError::TooLarge => DocumentError::TooLarge,
            ImportError::Invalid(_) => DocumentError::Unreadable,
        })?;
    Ok((imported.content, imported.warnings))
}

/// Renders the editor's `ProseMirror` JSON as `.docx` bytes.
pub(crate) async fn export_docx(content: Value, title: String) -> Result<Vec<u8>, DocumentError> {
    tokio::task::spawn_blocking(move || documents_engine::export_docx(&content, &title))
        .await
        .map_err(|_| DocumentError::Internal)?
        .map_err(|err| match err {
            ExportError::NotADocument => DocumentError::Unreadable,
            ExportError::Write(_) => DocumentError::Internal,
        })
}
