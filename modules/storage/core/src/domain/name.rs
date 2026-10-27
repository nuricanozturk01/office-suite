use thiserror::Error;

const MAX_CHARS: usize = 200;

/// Rejected file name.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("file name is invalid")]
pub(crate) struct NameError;

/// A file name without path separators or control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageName(String);

impl PackageName {
    /// Trims and validates `raw`.
    ///
    /// # Errors
    /// Fails when the name is empty, too long, or contains a path separator or control character.
    pub(crate) fn new(raw: &str) -> Result<Self, NameError> {
        let name = raw.trim();
        let bad = |c: char| c.is_control() || matches!(c, '/' | '\\');
        if name.is_empty() || name.chars().count() > MAX_CHARS || name.chars().any(bad) {
            return Err(NameError);
        }
        Ok(Self(name.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_names_and_rejects_paths() {
        assert!(PackageName::new(" Rapor 1.docx ").is_ok());
        assert_eq!(PackageName::new("../etc/passwd"), Err(NameError));
        assert_eq!(PackageName::new("a\\b"), Err(NameError));
        assert_eq!(PackageName::new("  "), Err(NameError));
        assert_eq!(PackageName::new("a\nb"), Err(NameError));
    }
}
