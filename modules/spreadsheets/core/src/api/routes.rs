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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_kernel::Uuid;
use spreadsheets_contracts::{OwnerId, SpreadsheetId};
use time::OffsetDateTime;
use utoipa::{
    IntoParams, OpenApi, ToSchema,
    openapi::schema::{AdditionalProperties, Object, ObjectBuilder},
};

use crate::{
    application::{SpreadsheetError, SpreadsheetService, SpreadsheetSummary},
    domain::Spreadsheet,
};

type Service = Arc<SpreadsheetService>;

fn workbook_schema() -> Object {
    ObjectBuilder::new()
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .description(Some("Sparse workbook"))
        .build()
}

#[derive(Deserialize, ToSchema)]
struct SpreadsheetCreateBody {
    title: String,
    /// Name of the first sheet (localized by the client).
    sheet_name: String,
}

#[derive(Deserialize, ToSchema)]
struct SpreadsheetTitleBody {
    title: String,
}

#[derive(Deserialize, ToSchema)]
struct SpreadsheetContentBody {
    /// Version the client last observed.
    version: i64,
    /// Sparse workbook. Validated before it is stored.
    #[schema(schema_with = workbook_schema)]
    content: Value,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SpreadsheetListQuery {
    /// `next_cursor` of the previous page; opaque.
    cursor: Option<String>,
    /// Page size.
    #[param(minimum = 1)]
    limit: Option<u32>,
}

#[derive(Serialize, ToSchema)]
struct SpreadsheetResponse {
    #[schema(format = Uuid)]
    id: String,
    title: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
struct SpreadsheetListResponse {
    items: Vec<SpreadsheetResponse>,
    next_cursor: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct SpreadsheetDetailResponse {
    #[schema(format = Uuid)]
    id: String,
    title: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
    version: i64,
    #[schema(schema_with = workbook_schema)]
    content: Value,
}

#[derive(Serialize, ToSchema)]
struct SpreadsheetContentResponse {
    version: i64,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

/// Raw `.xlsx` bytes; documents the binary body in the `OpenAPI` schema only.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[expect(
    dead_code,
    reason = "schema marker for the binary request and response bodies"
)]
struct XlsxFile(Vec<u8>);

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ImportQuery {
    /// Title of the new spreadsheet (usually the file name without extension).
    name: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct SpreadsheetImportResponse {
    spreadsheet: SpreadsheetDetailResponse,
    /// What the importer dropped or approximated (limits, dates, unsupported parts).
    warnings: Vec<String>,
}

impl From<&SpreadsheetSummary> for SpreadsheetResponse {
    fn from(summary: &SpreadsheetSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            title: summary.title.as_str().to_owned(),
            created_at: summary.created_at,
            updated_at: summary.updated_at,
        }
    }
}

impl From<&Spreadsheet> for SpreadsheetDetailResponse {
    fn from(spreadsheet: &Spreadsheet) -> Self {
        Self {
            id: spreadsheet.id().to_string(),
            title: spreadsheet.title().as_str().to_owned(),
            created_at: spreadsheet.created_at(),
            updated_at: spreadsheet.updated_at(),
            version: spreadsheet.version(),
            content: spreadsheet.content_value().clone(),
        }
    }
}

impl From<SpreadsheetError> for ProblemDetails {
    fn from(err: SpreadsheetError) -> Self {
        match err {
            SpreadsheetError::Invalid => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            ),
            SpreadsheetError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "not_found", "Not found")
            }
            SpreadsheetError::Conflict => {
                Self::new(StatusCode::CONFLICT, "version_conflict", "Version conflict")
            }
            SpreadsheetError::TooLarge => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File is too large",
            ),
            SpreadsheetError::Unreadable => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unreadable_file",
                "File is not a readable .xlsx workbook",
            ),
            SpreadsheetError::Internal => Self::new(
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
    operation_id = "createSpreadsheet",
    tag = "spreadsheets",
    security(("bearer" = [])),
    request_body = SpreadsheetCreateBody,
    responses(
        (status = 201, description = "Created", body = SpreadsheetDetailResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn create(
    State(service): State<Service>,
    user: AuthUser,
    Json(body): Json<SpreadsheetCreateBody>,
) -> Result<(StatusCode, Json<SpreadsheetDetailResponse>), ProblemDetails> {
    let spreadsheet = service
        .create(owner(user), &body.title, &body.sheet_name)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(SpreadsheetDetailResponse::from(&spreadsheet)),
    ))
}

#[utoipa::path(
    get,
    path = "",
    operation_id = "listSpreadsheets",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(SpreadsheetListQuery),
    responses(
        (status = 200, description = "OK", body = SpreadsheetListResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn list(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<SpreadsheetListQuery>,
) -> Result<Json<SpreadsheetListResponse>, ProblemDetails> {
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<UpdatedCursor>)
        .transpose()?;
    let page = service.list(owner(user), cursor, query.limit).await?;
    Ok(Json(SpreadsheetListResponse {
        next_cursor: page.next_cursor.map(|id| id.to_string()),
        items: page.items.iter().map(SpreadsheetResponse::from).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/{id}",
    operation_id = "getSpreadsheet",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = SpreadsheetDetailResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn get_one(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<SpreadsheetDetailResponse>, ProblemDetails> {
    let spreadsheet = service
        .get(owner(user), SpreadsheetId::from_uuid(id))
        .await?;
    Ok(Json(SpreadsheetDetailResponse::from(&spreadsheet)))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    operation_id = "renameSpreadsheet",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = SpreadsheetTitleBody,
    responses(
        (status = 200, description = "OK", body = SpreadsheetResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn rename(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SpreadsheetTitleBody>,
) -> Result<Json<SpreadsheetResponse>, ProblemDetails> {
    let spreadsheet = service
        .rename(owner(user), SpreadsheetId::from_uuid(id), &body.title)
        .await?;
    Ok(Json(SpreadsheetResponse {
        id: spreadsheet.id().to_string(),
        title: spreadsheet.title().as_str().to_owned(),
        created_at: spreadsheet.created_at(),
        updated_at: spreadsheet.updated_at(),
    }))
}

#[utoipa::path(
    put,
    path = "/{id}/content",
    operation_id = "saveSpreadsheetContent",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = SpreadsheetContentBody,
    responses(
        (status = 200, description = "Saved", body = SpreadsheetContentResponse),
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
    Json(body): Json<SpreadsheetContentBody>,
) -> Result<Json<SpreadsheetContentResponse>, ProblemDetails> {
    let spreadsheet = service
        .save_content(
            owner(user),
            SpreadsheetId::from_uuid(id),
            body.content,
            body.version,
        )
        .await?;
    Ok(Json(SpreadsheetContentResponse {
        version: spreadsheet.version(),
        updated_at: spreadsheet.updated_at(),
    }))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    operation_id = "deleteSpreadsheet",
    tag = "spreadsheets",
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
        .delete(owner(user), SpreadsheetId::from_uuid(id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/import",
    operation_id = "importSpreadsheet",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(ImportQuery),
    request_body(
        content = XlsxFile,
        content_type = "application/octet-stream",
        description = "The .xlsx file, at most 10 MiB",
    ),
    responses(
        (status = 201, description = "Imported", body = SpreadsheetImportResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 413, description = "File too large", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Not a readable workbook", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn import_xlsx(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<ImportQuery>,
    body: Bytes,
) -> Result<(StatusCode, Json<SpreadsheetImportResponse>), ProblemDetails> {
    let title = query.name.unwrap_or_else(|| "Imported".to_owned());
    let (spreadsheet, warnings) = service
        .import_xlsx(owner(user), &title, body.to_vec())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(SpreadsheetImportResponse {
            spreadsheet: SpreadsheetDetailResponse::from(&spreadsheet),
            warnings,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/{id}/export.xlsx",
    operation_id = "exportSpreadsheet",
    tag = "spreadsheets",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (
            status = 200,
            description = "The workbook as an .xlsx attachment",
            body = XlsxFile,
            content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn export_xlsx(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response, ProblemDetails> {
    let (spreadsheet, bytes) = service
        .export_xlsx(owner(user), SpreadsheetId::from_uuid(id))
        .await?;
    let disposition = content_disposition(spreadsheet.title().as_str());
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
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
    format!("attachment; filename=\"{fallback}.xlsx\"; filename*=UTF-8''{encoded}.xlsx")
}

#[derive(OpenApi)]
#[openapi(paths(
    create,
    list,
    get_one,
    rename,
    save_content,
    delete,
    import_xlsx,
    export_xlsx
))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).patch(rename).delete(delete))
        .route("/{id}/content", put(save_content))
        .route("/{id}/export.xlsx", get(export_xlsx))
        .route(
            "/import",
            post(import_xlsx).layer(DefaultBodyLimit::max(spreadsheets_engine::MAX_XLSX_BYTES)),
        )
        .with_state(service)
}
