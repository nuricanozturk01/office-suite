use thiserror::Error;

const MAX_EMAIL_CHARS: usize = 254;
const MAX_NAME_CHARS: usize = 100;
const MIN_PASSWORD_CHARS: usize = 12;
const MAX_PASSWORD_BYTES: usize = 256;
const MAX_BIO_CHARS: usize = 300;
const MAX_AVATAR_URL_CHARS: usize = 500;

/// Invalid value object input.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValueError {
    /// Email is malformed.
    #[error("email is invalid")]
    Email,
    /// Display name is empty or too long.
    #[error("display name must be 1-{MAX_NAME_CHARS} characters")]
    DisplayName,
    /// Password violates the policy.
    #[error("password must be {MIN_PASSWORD_CHARS}-{MAX_PASSWORD_BYTES} bytes")]
    Password,
    /// Bio exceeds max length.
    #[error("bio must be at most {MAX_BIO_CHARS} characters")]
    BioTooLong,
    /// Avatar URL is not a valid https:// URL or is too long.
    #[error("avatar URL must be a valid https:// URL of at most {MAX_AVATAR_URL_CHARS} characters")]
    AvatarUrl,
    /// Deletion is already scheduled.
    #[error("account deletion is already scheduled")]
    DeletionAlreadyScheduled,
    /// No pending deletion to cancel.
    #[error("no pending account deletion")]
    NoPendingDeletion,
    /// Current password mismatch.
    #[error("current password is incorrect")]
    WrongPassword,
}

/// Normalized (trimmed, lowercased) email address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    /// Parses an email address.
    ///
    /// # Errors
    /// [`ValueError::Email`] when malformed.
    pub fn parse(raw: &str) -> Result<Self, ValueError> {
        let value = raw.trim().to_lowercase();
        let valid = value.len() <= MAX_EMAIL_CHARS
            && value.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty()
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
                    && !value.contains(char::is_whitespace)
                    && domain.matches('@').count() == 0
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(ValueError::Email)
        }
    }

    /// Address as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    /// Parses a display name.
    ///
    /// # Errors
    /// [`ValueError::DisplayName`] when empty or too long.
    pub fn parse(raw: &str) -> Result<Self, ValueError> {
        let value = raw.trim();
        if value.is_empty() || value.chars().count() > MAX_NAME_CHARS {
            return Err(ValueError::DisplayName);
        }
        Ok(Self(value.to_owned()))
    }

    /// Name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Short user biography, max 300 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bio(String);

impl Bio {
    /// Creates a bio, rejecting strings over 300 characters.
    ///
    /// # Errors
    /// [`ValueError::BioTooLong`] when over 300 chars.
    pub fn new(raw: &str) -> Result<Self, ValueError> {
        if raw.chars().count() > MAX_BIO_CHARS {
            return Err(ValueError::BioTooLong);
        }
        Ok(Self(raw.to_owned()))
    }

    /// Bio as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated `https://` avatar URL, max 500 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarUrl(String);

impl AvatarUrl {
    /// Parses an avatar URL.
    ///
    /// # Errors
    /// [`ValueError::AvatarUrl`] when not `https://`, or over 500 characters.
    pub fn parse(raw: &str) -> Result<Self, ValueError> {
        let value = raw.trim();
        if !value.starts_with("https://") || value.chars().count() > MAX_AVATAR_URL_CHARS {
            return Err(ValueError::AvatarUrl);
        }
        Ok(Self(value.to_owned()))
    }

    /// URL as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Plaintext password that satisfies the policy. Never logged or serialized.
#[derive(Clone)]
pub struct Password(String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

impl Password {
    /// Validates a new password against the policy.
    ///
    /// # Errors
    /// [`ValueError::Password`] when too short or too long.
    pub fn new(raw: &str) -> Result<Self, ValueError> {
        if raw.chars().count() < MIN_PASSWORD_CHARS || raw.len() > MAX_PASSWORD_BYTES {
            return Err(ValueError::Password);
        }
        Ok(Self(raw.to_owned()))
    }

    /// Wraps a login attempt without enforcing the creation policy.
    #[must_use]
    pub fn attempt(raw: &str) -> Self {
        Self(raw.chars().take(MAX_PASSWORD_BYTES).collect())
    }

    /// Plaintext, for hashing only.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_normalized() {
        assert_eq!(
            Email::parse("  Ada@Example.COM ").unwrap().as_str(),
            "ada@example.com"
        );
    }

    #[test]
    fn email_rejects_garbage() {
        for bad in [
            "",
            "a",
            "a@b",
            "@x.com",
            "a b@x.com",
            "a@.com",
            "a@x.",
            "a@@x.com",
        ] {
            assert_eq!(Email::parse(bad), Err(ValueError::Email), "{bad}");
        }
    }

    #[test]
    fn name_bounds() {
        assert!(DisplayName::parse("  ").is_err());
        assert!(DisplayName::parse(&"x".repeat(101)).is_err());
        assert_eq!(DisplayName::parse(" Ada ").unwrap().as_str(), "Ada");
    }

    #[test]
    fn password_policy() {
        assert!(Password::new("short").is_err());
        assert!(Password::new("correct horse battery").is_ok());
        assert_eq!(
            format!("{:?}", Password::attempt("x")),
            "Password(<redacted>)"
        );
    }

    #[test]
    fn bio_rejects_over_300_chars() {
        let long = "a".repeat(301);
        assert_eq!(Bio::new(&long), Err(ValueError::BioTooLong));
    }

    #[test]
    fn bio_accepts_empty_and_300_chars() {
        assert!(Bio::new("").is_ok());
        assert!(Bio::new(&"a".repeat(300)).is_ok());
    }

    #[test]
    fn avatar_url_rejects_http() {
        assert_eq!(
            AvatarUrl::parse("http://example.com/pic.jpg"),
            Err(ValueError::AvatarUrl)
        );
    }

    #[test]
    fn avatar_url_rejects_data_uri() {
        assert_eq!(
            AvatarUrl::parse("data:image/png;base64,abc"),
            Err(ValueError::AvatarUrl)
        );
    }

    #[test]
    fn avatar_url_rejects_over_500_chars() {
        let long = format!("https://x.com/{}", "a".repeat(490));
        assert_eq!(AvatarUrl::parse(&long), Err(ValueError::AvatarUrl));
    }

    #[test]
    fn avatar_url_accepts_valid_https() {
        assert!(AvatarUrl::parse("https://example.com/pic.jpg").is_ok());
    }
}
