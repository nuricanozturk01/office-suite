//! Pure domain: no I/O.

mod document;
mod title;

pub use document::{Document, DocumentEvent};
pub use title::{DocumentTitle, TitleError};
