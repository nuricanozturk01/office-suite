//! HTTP plumbing: RFC 9457 problem details and the bearer-token extractor.

use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::FromRequestParts,
    http::{HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

/// Media type of [`ProblemDetails`] responses.
pub const PROBLEM_JSON: &str = "application/problem+json";

/// `application/problem+json` body.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProblemDetails {
    /// Short, stable type slug.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Human-readable summary.
    pub title: &'static str,
    /// HTTP status code.
    pub status: u16,
    /// Machine-readable error code.
    pub code: &'static str,
    /// Occurrence-specific detail (never contains secrets or PII).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ProblemDetails {
    /// Builds a problem for the given status.
    #[must_use]
    pub const fn new(status: StatusCode, code: &'static str, title: &'static str) -> Self {
        Self {
            kind: "about:blank",
            title,
            status: status.as_u16(),
            code,
            detail: None,
        }
    }

    /// Adds a detail message.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (status, Json(self)).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(PROBLEM_JSON));
        response
    }
}

/// Verifies a bearer token and returns the user id it was issued for.
pub trait BearerVerifier: Send + Sync {
    /// Returns the user id for a valid token.
    fn verify(&self, token: &str) -> Option<Uuid>;
}

/// Extractor for an authenticated request; rejects with 401 otherwise.
///
/// Requires `Extension<Arc<dyn BearerVerifier>>` to be installed by the composition root.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let unauthorized = || {
            ProblemDetails::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required",
            )
        };
        let Extension(verifier) =
            Extension::<Arc<dyn BearerVerifier>>::from_request_parts(parts, state)
                .await
                .map_err(|_| unauthorized())?;
        let token = bearer_token(&parts.headers).ok_or_else(unauthorized)?;
        verifier.verify(token).map(Self).ok_or_else(unauthorized)
    }
}

/// Token from `Authorization: Bearer <t>`, or from `Sec-WebSocket-Protocol: bearer, <t>`
/// (browsers cannot set headers on WebSocket handshakes and URLs end up in logs).
fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return Some(token);
    }
    let mut protocols = headers
        .get(header::SEC_WEBSOCKET_PROTOCOL)?
        .to_str()
        .ok()?
        .split(',')
        .map(str::trim);
    protocols.find(|p| *p == "bearer")?;
    protocols.next().filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    fn headers(name: header::HeaderName, value: &'static str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(name, HeaderValue::from_static(value));
        map
    }

    #[test]
    fn reads_authorization_header() {
        let h = headers(header::AUTHORIZATION, "Bearer abc");
        assert_eq!(bearer_token(&h), Some("abc"));
    }

    #[test]
    fn reads_websocket_subprotocol() {
        let h = headers(header::SEC_WEBSOCKET_PROTOCOL, "bearer, tok.en");
        assert_eq!(bearer_token(&h), Some("tok.en"));
    }

    #[test]
    fn rejects_missing_or_malformed() {
        assert_eq!(bearer_token(&HeaderMap::new()), None);
        assert_eq!(
            bearer_token(&headers(header::SEC_WEBSOCKET_PROTOCOL, "bearer")),
            None
        );
        assert_eq!(
            bearer_token(&headers(header::SEC_WEBSOCKET_PROTOCOL, "other, x")),
            None
        );
        assert_eq!(
            bearer_token(&headers(header::AUTHORIZATION, "Basic abc")),
            None
        );
    }
}
