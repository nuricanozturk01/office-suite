//! Presentation aggregate and value objects.

mod content;
mod presentation;
mod title;

pub(crate) use content::{ContentError, DeckContent};
pub(crate) use presentation::{Presentation, PresentationEvent};
pub(crate) use title::{PresentationTitle, TitleError};
