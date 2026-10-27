//! Pure domain: no I/O.

mod pdf;
mod title;

pub use pdf::{Pdf, PdfEvent};
pub use title::{PdfTitle, TitleError};
