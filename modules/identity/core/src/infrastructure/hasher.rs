use argon2::{Argon2, PasswordHash, PasswordHasher as _, PasswordVerifier as _};
use async_trait::async_trait;

use crate::application::{PasswordHasher, RepositoryError};

/// Argon2id hasher; hashing runs on the blocking pool.
#[derive(Debug)]
pub(crate) struct Argon2Hasher {
    dummy: String,
}

impl Argon2Hasher {
    /// Creates the hasher and precomputes the timing-equalization hash.
    ///
    /// # Errors
    /// [`RepositoryError::Storage`] when hashing fails.
    pub(crate) fn new() -> Result<Self, RepositoryError> {
        Ok(Self {
            dummy: hash_sync("timing-equalization-placeholder")?,
        })
    }
}

fn hash_sync(plaintext: &str) -> Result<String, RepositoryError> {
    Argon2::default()
        .hash_password(plaintext.as_bytes())
        .map(|h| h.to_string())
        .map_err(|_| RepositoryError::Storage)
}

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, plaintext: &str) -> Result<String, RepositoryError> {
        let plaintext = plaintext.to_owned();
        tokio::task::spawn_blocking(move || hash_sync(&plaintext))
            .await
            .map_err(|_| RepositoryError::Storage)?
    }

    async fn verify(&self, plaintext: &str, phc: &str) -> bool {
        let (plaintext, phc) = (plaintext.to_owned(), phc.to_owned());
        tokio::task::spawn_blocking(move || {
            PasswordHash::new(&phc).is_ok_and(|parsed| {
                Argon2::default()
                    .verify_password(plaintext.as_bytes(), &parsed)
                    .is_ok()
            })
        })
        .await
        .unwrap_or(false)
    }

    fn dummy_hash(&self) -> &str {
        &self.dummy
    }
}
