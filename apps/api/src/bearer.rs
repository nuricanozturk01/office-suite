//! Adapts the identity module's token port to the platform's HTTP extractor.

use std::sync::Arc;

use identity_contracts::TokenVerifier;
use platform::http::BearerVerifier;
use shared_kernel::Uuid;

#[derive(Debug)]
pub(crate) struct Adapter(pub(crate) Arc<dyn TokenVerifier>);

impl BearerVerifier for Adapter {
    fn verify(&self, token: &str) -> Option<Uuid> {
        self.0.verify(token).map(|id| id.as_uuid())
    }
}
