use thiserror::Error;

const MAX_CHARS: usize = 255;

/// Invalid document title.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TitleError {
    /// Title is empty after trimming.
    #[error("title must not be empty")]
    Empty,
    /// Title exceeds the maximum length.
    #[error("title must be at most {MAX_CHARS} characters")]
    TooLong,
}

/// Validated document title (parse, don't validate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTitle(String);

impl DocumentTitle {
    /// Parses and normalizes a title.
    ///
    /// # Errors
    /// Returns [`TitleError`] when empty or too long.
    pub fn new(raw: &str) -> Result<Self, TitleError> {
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
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_whitespace() {
        assert_eq!(DocumentTitle::new("  Plan ").unwrap().as_str(), "Plan");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(DocumentTitle::new("   "), Err(TitleError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        assert_eq!(
            DocumentTitle::new(&"a".repeat(256)),
            Err(TitleError::TooLong)
        );
    }
}
