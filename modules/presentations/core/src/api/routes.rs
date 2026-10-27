use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use platform::{
    http::{AuthUser, ProblemDetails},
    pagination::UpdatedCursor,
};
use presentations_contracts::{OwnerId, PresentationId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_kernel::Uuid;
use time::OffsetDateTime;
use utoipa::{
    IntoParams, OpenApi, ToSchema,
    openapi::schema::{AdditionalProperties, Object, ObjectBuilder},
};

use crate::{
    application::{PresentationError, PresentationService, PresentationSummary},
    domain::Presentation,
};

type Service = Arc<PresentationService>;

fn deck_schema() -> Object {
    ObjectBuilder::new()
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .description(Some("Slide deck"))
        .build()
}

#[derive(Deserialize, ToSchema)]
struct PresentationCreateBody {
    title: String,
}

#[derive(Deserialize, ToSchema)]
struct PresentationTitleBody {
    title: String,
}

#[derive(Deserialize, ToSchema)]
struct PresentationContentBody {
    /// Version the client last observed.
    version: i64,
    /// Slide deck. Validated before it is stored.
    #[schema(schema_with = deck_schema)]
    content: Value,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct PresentationListQuery {
    /// `next_cursor` of the previous page; opaque.
    cursor: Option<String>,
    /// Page size.
    #[param(minimum = 1)]
    limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
struct PresentationResponse {
    #[schema(format = Uuid)]
    id: String,
    title: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
struct PresentationListResponse {
    items: Vec<PresentationResponse>,
    next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct PresentationDetailResponse {
    #[schema(format = Uuid)]
    id: String,
    title: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
    version: i64,
    #[schema(schema_with = deck_schema)]
    content: Value,
}

#[derive(Serialize, ToSchema)]
struct PresentationContentResponse {
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

/// Raw `.pptx` bytes; documents the binary body in the `OpenAPI` schema only.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[expect(
    dead_code,
    reason = "schema marker for the binary request and response bodies"
)]
struct PptxFile(Vec<u8>);

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ImportQuery {
    /// Title of the new presentation (usually the file name without extension).
    name: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct PresentationImportResponse {
    presentation: PresentationDetailResponse,
    /// What the importer dropped or approximated (limits, dates, unsupported parts).
    warnings: Vec<String>,
}

impl From<&PresentationSummary> for PresentationResponse {
    fn from(summary: &PresentationSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            title: summary.title.as_str().to_owned(),
            created_at: summary.created_at,
            updated_at: summary.updated_at,
        }
    }
}

impl From<&Presentation> for PresentationDetailResponse {
    fn from(presentation: &Presentation) -> Self {
        Self {
            id: presentation.id().to_string(),
            title: presentation.title().as_str().to_owned(),
            created_at: presentation.created_at(),
            updated_at: presentation.updated_at(),
            version: presentation.version(),
            content: presentation.content_value().clone(),
        }
    }
}

impl From<PresentationError> for ProblemDetails {
    fn from(err: PresentationError) -> Self {
        match err {
            PresentationError::Invalid => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            ),
            PresentationError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "not_found", "Not found")
            }
            PresentationError::Conflict => {
                Self::new(StatusCode::CONFLICT, "version_conflict", "Version conflict")
            }
            PresentationError::TooLarge => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File is too large",
            ),
            PresentationError::Unreadable => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unreadable_file",
                "File is not a readable .pptx deck",
            ),
            PresentationError::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            ),
        }
    }
}

const fn owner(user: AuthUser) -> OwnerId {
    OwnerId::from_uuid(user.0)
}

#[utoipa::path(
    post,
    path = "",
    operation_id = "createPresentation",
    tag = "presentations",
    security(("bearer" = [])),
    request_body = PresentationCreateBody,
    responses(
        (status = 201, description = "Created", body = PresentationDetailResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn create(
    State(service): State<Service>,
    user: AuthUser,
    Json(body): Json<PresentationCreateBody>,
) -> Result<(StatusCode, Json<PresentationDetailResponse>), ProblemDetails> {
    let presentation = service.create(owner(user), &body.title).await?;
    Ok((
        StatusCode::CREATED,
        Json(PresentationDetailResponse::from(&presentation)),
    ))
}

#[utoipa::path(
    get,
    path = "",
    operation_id = "listPresentations",
    tag = "presentations",
    security(("bearer" = [])),
    params(PresentationListQuery),
    responses(
        (status = 200, description = "OK", body = PresentationListResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn list(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<PresentationListQuery>,
) -> Result<Json<PresentationListResponse>, ProblemDetails> {
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<UpdatedCursor>)
        .transpose()?;
    let page = service.list(owner(user), cursor, query.limit).await?;
    Ok(Json(PresentationListResponse {
        next_cursor: page.next_cursor.map(|id| id.to_string()),
        items: page.items.iter().map(PresentationResponse::from).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getPresentation",
    tag = "presentations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = PresentationDetailResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn get_one(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PresentationDetailResponse>, ProblemDetails> {
    let presentation = service
        .get(owner(user), PresentationId::from_uuid(id))
        .await?;
    Ok(Json(PresentationDetailResponse::from(&presentation)))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "renamePresentation",
    tag = "presentations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = PresentationTitleBody,
    responses(
        (status = 200, description = "OK", body = PresentationResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn rename(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PresentationTitleBody>,
) -> Result<Json<PresentationResponse>, ProblemDetails> {
    let presentation = service
        .rename(owner(user), PresentationId::from_uuid(id), &body.title)
        .await?;
    Ok(Json(PresentationResponse {
        id: presentation.id().to_string(),
        title: presentation.title().as_str().to_owned(),
        created_at: presentation.created_at(),
        updated_at: presentation.updated_at(),
    }))
}

#[utoipa::path(
    put,
    path = "/{id}/content",
    operation_id = "savePresentationContent",
    tag = "presentations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = PresentationContentBody,
    responses(
        (status = 200, description = "Saved", body = PresentationContentResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "Stale version", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn save_content(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PresentationContentBody>,
) -> Result<Json<PresentationContentResponse>, ProblemDetails> {
    let presentation = service
        .save_content(
            owner(user),
            PresentationId::from_uuid(id),
            body.content,
            body.version,
        )
        .await?;
    Ok(Json(PresentationContentResponse {
        version: presentation.version(),
        updated_at: presentation.updated_at(),
    }))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deletePresentation",
    tag = "presentations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 204, description = "No content"),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn delete(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ProblemDetails> {
    service
        .delete(owner(user), PresentationId::from_uuid(id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/import",
    operation_id = "importPresentation",
    tag = "presentations",
    security(("bearer" = [])),
    params(ImportQuery),
    request_body(
        content = PptxFile,
        content_type = "application/octet-stream",
        description = "The .pptx file, at most 10 MiB",
    ),
    responses(
        (status = 201, description = "Imported", body = PresentationImportResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 413, description = "File too large", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Not a readable deck", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn import_pptx(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<ImportQuery>,
    body: Bytes,
) -> Result<(StatusCode, Json<PresentationImportResponse>), ProblemDetails> {
    let title = query.name.unwrap_or_else(|| "Imported".to_owned());
    let (presentation, warnings) = service
        .import_pptx(owner(user), &title, body.to_vec())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(PresentationImportResponse {
            presentation: PresentationDetailResponse::from(&presentation),
            warnings,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/{id}/export.pptx",
    operation_id = "exportPresentation",
    tag = "presentations",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (
            status = 200,
            description = "The deck as an .pptx attachment",
            body = PptxFile,
            content_type = "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn export_pptx(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response, ProblemDetails> {
    let (presentation, bytes) = service
        .export_pptx(owner(user), PresentationId::from_uuid(id))
        .await?;
    let disposition = content_disposition(presentation.title().as_str());
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).map_or('0', |c| c.to_ascii_uppercase())
}

/// `attachment` header with an ASCII fallback and an RFC 5987 UTF-8 name.
fn content_disposition(title: &str) -> String {
    let fallback: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for byte in title.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    format!("attachment; filename=\"{fallback}.pptx\"; filename*=UTF-8''{encoded}.pptx")
}

#[derive(OpenApi)]
#[openapi(paths(
    create,
    list,
    get_one,
    rename,
    save_content,
    delete,
    import_pptx,
    export_pptx
))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(rename).delete(delete))
        .route("/{id}/content", put(save_content))
        .route("/{id}/export.pptx", get(export_pptx))
        .route(
            "/import",
            post(import_pptx).layer(DefaultBodyLimit::max(presentations_engine::MAX_PPTX_BYTES)),
        )
        .with_state(service)
}
