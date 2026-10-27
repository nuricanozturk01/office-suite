use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use pdfs_contracts::{OwnerId, PdfId};
use platform::{
    http::{AuthUser, ProblemDetails},
    pagination::UpdatedCursor,
};
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;
use time::OffsetDateTime;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::{
    application::{PdfError, PdfService},
    domain::Pdf,
};

type Service = Arc<PdfService>;

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
struct PdfResponse {
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
    items: Vec<PdfResponse>,
    next_cursor: Option<String>,
}

impl From<&Pdf> for PdfResponse {
    fn from(doc: &Pdf) -> Self {
        Self {
            id: doc.id().to_string(),
            title: doc.title().as_str().to_owned(),
            created_at: doc.created_at(),
            updated_at: doc.updated_at(),
        }
    }
}

impl From<PdfError> for ProblemDetails {
    fn from(err: PdfError) -> Self {
        match err {
            PdfError::Invalid(e) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            )
            .with_detail(e.to_string()),
            PdfError::NotFound => Self::new(StatusCode::NOT_FOUND, "not_found", "Not found"),
            PdfError::Internal => Self::new(
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
    operation_id = "createPdf",
    tag = "pdfs",
    security(("bearer" = [])),
    request_body = TitleBody,
    responses(
        (status = 201, description = "Created", body = PdfResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn create(
    State(service): State<Service>,
    user: AuthUser,
    Json(body): Json<TitleBody>,
) -> Result<(StatusCode, Json<PdfResponse>), ProblemDetails> {
    let doc = service.create(owner(user), &body.title).await?;
    Ok((StatusCode::CREATED, Json(PdfResponse::from(&doc))))
}

#[utoipa::path(
    get,
    path = "",
    operation_id = "listPdfs",
    tag = "pdfs",
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
        items: page.items.iter().map(PdfResponse::from).collect(),
        next_cursor: page.next_cursor.map(|c| c.to_string()),
    }))
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getPdf",
    tag = "pdfs",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = PdfResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn get_one(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PdfResponse>, ProblemDetails> {
    let doc = service.get(owner(user), PdfId::from_uuid(id)).await?;
    Ok(Json(PdfResponse::from(&doc)))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "renamePdf",
    tag = "pdfs",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = TitleBody,
    responses(
        (status = 200, description = "OK", body = PdfResponse),
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
) -> Result<Json<PdfResponse>, ProblemDetails> {
    let doc = service
        .rename(owner(user), PdfId::from_uuid(id), &body.title)
        .await?;
    Ok(Json(PdfResponse::from(&doc)))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deletePdf",
    tag = "pdfs",
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
    service.delete(owner(user), PdfId::from_uuid(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(OpenApi)]
#[openapi(paths(create, list, get_one, rename, delete))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(rename).delete(delete))
        .with_state(service)
}
