//! Typed, fail-fast configuration read from environment variables (12-factor).

use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

const MIN_JWT_SECRET_BYTES: usize = 32;

/// Invalid or missing configuration.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// A required variable is not set.
    #[error("missing environment variable {0}")]
    Missing(&'static str),
    /// A variable has an unusable value.
    #[error("invalid value for {name}: {reason}")]
    Invalid {
        /// Variable name.
        name: &'static str,
        /// Why it is invalid.
        reason: &'static str,
    },
}

/// Process-wide settings.
#[derive(Debug)]
pub struct Settings {
    /// Socket address the API binds to.
    pub api_addr: String,
    /// Postgres connection URL.
    pub database_url: SecretString,
    /// Redis connection URL (event broadcast, realtime fan-out).
    pub redis_url: SecretString,
    /// HMAC secret for access tokens.
    pub jwt_secret: SecretString,
    /// Access token lifetime in seconds.
    pub access_token_ttl_secs: u64,
    /// Refresh token lifetime in seconds.
    pub refresh_token_ttl_secs: u64,
    /// Whether cookies carry the `Secure` flag (`COOKIE_SECURE=true` behind HTTPS).
    pub cookie_secure: bool,
    /// Browser origins allowed to call the API cross-origin (`CORS_ALLOWED_ORIGINS`, comma
    /// separated, e.g. `https://myoffice.example.com`). Empty means same-origin only.
    pub cors_allowed_origins: Vec<String>,
}

impl Settings {
    /// Reads settings from the environment.
    ///
    /// # Errors
    /// Returns [`SettingsError`] when a variable is missing or invalid.
    pub fn from_env() -> Result<Self, SettingsError> {
        let jwt_secret = jwt_secret()?;
        if jwt_secret.expose_secret().len() < MIN_JWT_SECRET_BYTES {
            return Err(SettingsError::Invalid {
                name: "JWT_SECRET",
                reason: "must be at least 32 bytes",
            });
        }
        let access_token_ttl_secs = match std::env::var("ACCESS_TOKEN_TTL_SECS") {
            Ok(raw) => raw.parse().map_err(|_| SettingsError::Invalid {
                name: "ACCESS_TOKEN_TTL_SECS",
                reason: "must be an integer",
            })?,
            Err(_) => 900,
        };
        let refresh_token_ttl_secs = match std::env::var("REFRESH_TOKEN_TTL_SECS") {
            Ok(raw) => raw.parse().map_err(|_| SettingsError::Invalid {
                name: "REFRESH_TOKEN_TTL_SECS",
                reason: "must be an integer",
            })?,
            Err(_) => 2_592_000,
        };
        let cookie_secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "true" || v == "1");
        let cors_allowed_origins =
            parse_origins(&std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default())?;
        Ok(Self {
            api_addr: std::env::var("API_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned()),
            database_url: required("DATABASE_URL")?,
            redis_url: std::env::var("REDIS_URL").map_or_else(
                |_| SecretString::from("redis://127.0.0.1:6379"),
                SecretString::from,
            ),
            jwt_secret,
            access_token_ttl_secs,
            refresh_token_ttl_secs,
            cookie_secure,
            cors_allowed_origins,
        })
    }
}

/// Splits a comma-separated list of browser origins. Each entry must be `scheme://host[:port]`
/// with no path, query or trailing slash, because browsers send exactly that as `Origin`.
fn parse_origins(raw: &str) -> Result<Vec<String>, SettingsError> {
    raw.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            let rest = origin
                .strip_prefix("https://")
                .or_else(|| origin.strip_prefix("http://"))
                .ok_or(SettingsError::Invalid {
                    name: "CORS_ALLOWED_ORIGINS",
                    reason: "each origin must start with http:// or https://",
                })?;
            if rest.is_empty() || rest.contains(['/', '?', '#']) {
                return Err(SettingsError::Invalid {
                    name: "CORS_ALLOWED_ORIGINS",
                    reason: "each origin must be scheme://host[:port] without a path",
                });
            }
            Ok(origin.to_owned())
        })
        .collect()
}

fn required(name: &'static str) -> Result<SecretString, SettingsError> {
    std::env::var(name)
        .map(SecretString::from)
        .map_err(|_| SettingsError::Missing(name))
}

/// `JWT_SECRET`, or the file named by `JWT_SECRET_FILE` (Docker secret-file convention) when a
/// container entrypoint generates and persists it at first start. Trailing whitespace is trimmed.
fn jwt_secret() -> Result<SecretString, SettingsError> {
    if let Ok(value) = std::env::var("JWT_SECRET") {
        return Ok(SecretString::from(value));
    }
    let path =
        std::env::var("JWT_SECRET_FILE").map_err(|_| SettingsError::Missing("JWT_SECRET"))?;
    let contents = std::fs::read_to_string(&path).map_err(|_| SettingsError::Invalid {
        name: "JWT_SECRET_FILE",
        reason: "could not read the file it points to",
    })?;
    Ok(SecretString::from(contents.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::parse_origins;

    #[test]
    fn parses_comma_separated_origins() {
        let origins = parse_origins(" https://myoffice.example.com , http://localhost:5173,, ");
        assert_eq!(
            origins.ok(),
            Some(vec![
                "https://myoffice.example.com".to_owned(),
                "http://localhost:5173".to_owned(),
            ])
        );
    }

    #[test]
    fn empty_value_means_no_origins() {
        assert_eq!(parse_origins("").ok(), Some(Vec::new()));
    }

    #[test]
    fn rejects_origin_with_path_or_scheme_problems() {
        assert!(parse_origins("https://myoffice.example.com/").is_err());
        assert!(parse_origins("myoffice.example.com").is_err());
        assert!(parse_origins("https://").is_err());
    }
}
