//! Merged `OpenAPI` document of every mounted module.

use utoipa::openapi::{
    InfoBuilder, OpenApi, OpenApiBuilder,
    security::{Http, HttpAuthScheme, SecurityScheme},
};

/// Builds the API description; mount points must match [`crate::app`].
pub(crate) fn document() -> OpenApi {
    let mut doc = OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("Office Suite API")
                .version(env!("CARGO_PKG_VERSION"))
                .build(),
        )
        .build()
        .nest("/api/v1/auth", identity::openapi())
        .nest("/api/v1/documents", documents::openapi())
        .nest("/api/v1/spreadsheets", spreadsheets::openapi())
        .nest("/api/v1/presentations", presentations::openapi())
        .nest("/api/v1/pdfs", pdfs::openapi())
        .nest("/api/v1/packages", storage::openapi())
        .nest("/api/v1/files", workspace::openapi());
    doc.components
        .get_or_insert_with(Default::default)
        .add_security_scheme(
            "bearer",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    doc
}

/// Canonical serialization, also used for the committed `openapi.json`.
pub(crate) fn to_json() -> Result<String, serde_json::Error> {
    document().to_pretty_json().map(|mut json| {
        json.push('\n');
        json
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn committed_spec_is_up_to_date() {
        let committed = include_str!("../../../openapi.json");
        assert!(
            super::to_json().unwrap() == committed,
            "openapi.json is stale; run `just gen-api`"
        );
    }
}
