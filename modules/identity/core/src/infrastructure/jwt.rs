use identity_contracts::{TokenVerifier, UserId};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;

use crate::application::{AccessToken, RepositoryError, TokenIssuer};

const ISSUER: &str = "office-suite";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iss: String,
    exp: u64,
    iat: u64,
}

/// HS256 access tokens.
pub(crate) struct JwtTokens {
    encoding: EncodingKey,
    decoding: DecodingKey,
    ttl_secs: u64,
}

impl std::fmt::Debug for JwtTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtTokens")
            .field("ttl_secs", &self.ttl_secs)
            .finish_non_exhaustive()
    }
}

impl JwtTokens {
    /// Creates a token codec from the shared secret.
    #[must_use]
    pub(crate) fn new(secret: &SecretString, ttl_secs: u64) -> Self {
        let bytes = secret.expose_secret().as_bytes();
        Self {
            encoding: EncodingKey::from_secret(bytes),
            decoding: DecodingKey::from_secret(bytes),
            ttl_secs,
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

impl TokenIssuer for JwtTokens {
    fn issue(&self, user: UserId) -> Result<AccessToken, RepositoryError> {
        let iat = Self::now_secs();
        let claims = Claims {
            sub: user.as_uuid(),
            iss: ISSUER.to_owned(),
            exp: iat + self.ttl_secs,
            iat,
        };
        let value = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|_| RepositoryError::Storage)?;
        Ok(AccessToken {
            value,
            expires_in_secs: self.ttl_secs,
        })
    }
}

impl TokenVerifier for JwtTokens {
    fn verify(&self, token: &str) -> Option<UserId> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        decode::<Claims>(token, &self.decoding, &validation)
            .ok()
            .map(|data| UserId::from_uuid(data.claims.sub))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(secret: &str, ttl: u64) -> JwtTokens {
        JwtTokens::new(&SecretString::from(secret.to_owned()), ttl)
    }

    #[test]
    fn roundtrip() {
        let t = tokens("0123456789abcdef0123456789abcdef", 60);
        let id = UserId::from_uuid(Uuid::now_v7());
        let token = t.issue(id).unwrap();
        assert_eq!(t.verify(&token.value), Some(id));
    }

    #[test]
    fn rejects_other_secret_and_garbage() {
        let a = tokens("0123456789abcdef0123456789abcdef", 60);
        let b = tokens("fedcba9876543210fedcba9876543210", 60);
        let token = a.issue(UserId::from_uuid(Uuid::now_v7())).unwrap();
        assert_eq!(b.verify(&token.value), None);
        assert_eq!(a.verify("not-a-jwt"), None);
    }
}
