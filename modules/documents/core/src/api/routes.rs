use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use documents_contracts::{DocumentId, OwnerId};
use platform::{
    http::{AuthUser, ProblemDetails},
    pagination::UpdatedCursor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_kernel::Uuid;
use time::OffsetDateTime;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::{
    application::{self, DocumentError, DocumentService},
    domain::Document,
};

type Service = Arc<DocumentService>;

const EXPORT_BODY_LIMIT: usize = 10 * 1024 * 1024;

#[derive(Deserialize, ToSchema)]
struct TitleBody {
    title: String,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    /// `next_cursor` of the previous page; opaque.
    cursor: Option<String>,
    /// Page size.
    #[param(minimum = 1)]
    limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
struct DocumentResponse {
    #[schema(format = Uuid)]
    id: String,
    title: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
struct ListResponse {
    items: Vec<DocumentResponse>,
    next_cursor: Option<String>,
}

impl From<&Document> for DocumentResponse {
    fn from(doc: &Document) -> Self {
        Self {
            id: doc.id().to_string(),
            title: doc.title().as_str().to_owned(),
            created_at: doc.created_at(),
            updated_at: doc.updated_at(),
        }
    }
}

impl From<DocumentError> for ProblemDetails {
    fn from(err: DocumentError) -> Self {
        match err {
            DocumentError::Invalid(e) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            )
            .with_detail(e.to_string()),
            DocumentError::NotFound => Self::new(StatusCode::NOT_FOUND, "not_found", "Not found"),
            DocumentError::TooLarge => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File is too large",
            ),
            DocumentError::Unreadable => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unreadable_file",
                "File is not a readable .docx document",
            ),
            DocumentError::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            ),
        }
    }
}

/// Raw `.docx` bytes; documents the binary body in the `OpenAPI` schema only.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[expect(
    dead_code,
    reason = "schema marker for the binary request and response bodies"
)]
struct DocxFile(Vec<u8>);

#[derive(Serialize, ToSchema)]
struct DocxImportResponse {
    /// `ProseMirror` document in the editor schema.
    #[schema(value_type = Object)]
    content: Value,
    /// What the importer dropped or approximated.
    warnings: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
struct DocxExportBody {
    /// Title used for the file name and document properties.
    title: String,
    /// The editor's `getJSON()` output.
    #[schema(value_type = Object)]
    content: Value,
}

const fn owner(user: AuthUser) -> OwnerId {
    OwnerId::from_uuid(user.0)
}

#[utoipa::path(
    post,
    path = "",
    operation_id = "createDocument",
    tag = "documents",
    security(("bearer" = [])),
    request_body = TitleBody,
    responses(
        (status = 201, description = "Created", body = DocumentResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn create(
    State(service): State<Service>,
    user: AuthUser,
    Json(body): Json<TitleBody>,
) -> Result<(StatusCode, Json<DocumentResponse>), ProblemDetails> {
    let doc = service.create(owner(user), &body.title).await?;
    Ok((StatusCode::CREATED, Json(DocumentResponse::from(&doc))))
}

#[utoipa::path(
    get,
    path = "",
    operation_id = "listDocuments",
    tag = "documents",
    security(("bearer" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "OK", body = ListResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn list(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ProblemDetails> {
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<UpdatedCursor>)
        .transpose()?;
    let page = service.list(owner(user), cursor, query.limit).await?;
    Ok(Json(ListResponse {
        items: page.items.iter().map(DocumentResponse::from).collect(),
        next_cursor: page.next_cursor.map(|c| c.to_string()),
    }))
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getDocument",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DocumentResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn get_one(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentResponse>, ProblemDetails> {
    let doc = service.get(owner(user), DocumentId::from_uuid(id)).await?;
    Ok(Json(DocumentResponse::from(&doc)))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "renameDocument",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = TitleBody,
    responses(
        (status = 200, description = "OK", body = DocumentResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn rename(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<TitleBody>,
) -> Result<Json<DocumentResponse>, ProblemDetails> {
    let doc = service
        .rename(owner(user), DocumentId::from_uuid(id), &body.title)
        .await?;
    Ok(Json(DocumentResponse::from(&doc)))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deleteDocument",
    tag = "documents",
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
        .delete(owner(user), DocumentId::from_uuid(id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/import-docx",
    operation_id = "importDocx",
    tag = "documents",
    security(("bearer" = [])),
    request_body(
        content = DocxFile,
        content_type = "application/octet-stream",
        description = "The .docx file, at most 20 MiB",
    ),
    responses(
        (status = 200, description = "Converted", body = DocxImportResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 413, description = "File too large", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Not a readable document", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn import_docx(
    _user: AuthUser,
    body: Bytes,
) -> Result<Json<DocxImportResponse>, ProblemDetails> {
    let (content, warnings) = application::import_docx(body.to_vec()).await?;
    Ok(Json(DocxImportResponse { content, warnings }))
}

#[utoipa::path(
    post,
    path = "/export-docx",
    operation_id = "exportDocx",
    tag = "documents",
    security(("bearer" = [])),
    request_body = DocxExportBody,
    responses(
        (
            status = 200,
            description = "The document as a .docx attachment",
            body = DocxFile,
            content_type = "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Not an editor document", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn export_docx(
    _user: AuthUser,
    Json(body): Json<DocxExportBody>,
) -> Result<Response, ProblemDetails> {
    let bytes = application::export_docx(body.content, body.title).await?;
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"document.docx\""),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

#[derive(OpenApi)]
#[openapi(paths(create, list, get_one, rename, delete, import_docx, export_docx))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(rename).delete(delete))
        .route(
            "/import-docx",
            post(import_docx).layer(DefaultBodyLimit::max(documents_engine::MAX_DOCX_BYTES)),
        )
        .route(
            "/export-docx",
            post(export_docx).layer(DefaultBodyLimit::max(EXPORT_BODY_LIMIT)),
        )
        .with_state(service)
}
