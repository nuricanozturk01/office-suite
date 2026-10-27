use thiserror::Error;

const MAX_CHARS: usize = 255;

/// Invalid presentation title.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum TitleError {
    /// Title is empty after trimming.
    #[error("title must not be empty")]
    Empty,
    /// Title exceeds the maximum length.
    #[error("title must be at most {MAX_CHARS} characters")]
    TooLong,
}

/// Validated presentation title (parse, don't validate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresentationTitle(String);

impl PresentationTitle {
    /// Parses and normalizes a title.
    ///
    /// # Errors
    /// Returns [`TitleError`] when empty or too long.
    pub(crate) fn new(raw: &str) -> Result<Self, TitleError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(TitleError::Empty);
        }
        if trimmed.chars().count() > MAX_CHARS {
            return Err(TitleError::TooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Title as a string slice.
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_rejects_bounds() {
        assert_eq!(PresentationTitle::new("  Q3 ").unwrap().as_str(), "Q3");
        assert_eq!(PresentationTitle::new("  "), Err(TitleError::Empty));
        assert_eq!(
            PresentationTitle::new(&"a".repeat(256)),
            Err(TitleError::TooLong)
        );
    }
}
