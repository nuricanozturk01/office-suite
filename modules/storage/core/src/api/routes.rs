use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use platform::http::{AuthUser, ProblemDetails};
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;
use storage_contracts::MAX_PACKAGE_BYTES;
use utoipa::{IntoParams, OpenApi, ToSchema};

use crate::application::{StorageError, StorageService, StoredPackage, VersionList};

type Service = Arc<StorageService>;

const NAME_HEADER: HeaderName = HeaderName::from_static("x-package-name");
const VERSION_HEADER: HeaderName = HeaderName::from_static("x-package-version");

/// Raw file bytes; documents the binary body in the `OpenAPI` schema only.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[expect(
    dead_code,
    reason = "schema marker for the binary request and response bodies"
)]
struct PackageFile(Vec<u8>);

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct WriteQuery {
    /// File name, for example `Report.docx`.
    name: String,
    /// Version the save is based on; omit to replace unconditionally.
    base_version: Option<i64>,
    /// Combine with newer saves instead of refusing when the file changed since `base_version`.
    #[serde(default)]
    merge: bool,
}

#[derive(Serialize, ToSchema)]
struct WriteResponse {
    /// Version after the save.
    version: i64,
    /// Changes saved by somebody else in the meantime were merged into the file.
    merged: bool,
}

#[derive(Serialize, ToSchema)]
struct VersionEntry {
    /// Package version.
    version: i64,
    /// File name at that version.
    name: String,
    /// File size in bytes.
    size: i64,
    /// Who saved it.
    saved_by: Uuid,
    /// Display name of whoever saved it; `None` when the user is gone.
    saved_by_name: Option<String>,
    /// When it was saved (UTC).
    #[serde(with = "time::serde::rfc3339")]
    saved_at: time::OffsetDateTime,
}

#[derive(Serialize, ToSchema)]
struct VersionListResponse {
    /// The caller may restore an older version (needs edit access).
    can_restore: bool,
    /// Kept versions, newest first.
    versions: Vec<VersionEntry>,
}

impl From<VersionList> for VersionListResponse {
    fn from(list: VersionList) -> Self {
        Self {
            can_restore: list.can_restore,
            versions: list
                .entries
                .into_iter()
                .map(|(entry, name)| VersionEntry {
                    version: entry.version,
                    name: entry.name,
                    size: entry.size,
                    saved_by: entry.saved_by,
                    saved_by_name: name,
                    saved_at: entry.saved_at,
                })
                .collect(),
        }
    }
}

impl From<StorageError> for ProblemDetails {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Invalid => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            ),
            StorageError::NotFound => Self::new(StatusCode::NOT_FOUND, "not_found", "Not found"),
            StorageError::Conflict => {
                Self::new(StatusCode::CONFLICT, "version_conflict", "Version conflict")
            }
            StorageError::TooLarge => Self::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                "File is too large",
            ),
            StorageError::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            ),
        }
    }
}

fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).map_or('0', |c| c.to_ascii_uppercase())
}

fn percent_encode(name: &str) -> String {
    let mut out = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
    }
    out
}

#[utoipa::path(
    get,
    path = "/{resource_id}",
    operation_id = "readPackage",
    tag = "storage",
    security(("bearer" = [])),
    params(("resource_id" = Uuid, Path)),
    responses(
        (
            status = 200,
            description = "The stored file; its name and version are in `X-Package-Name` (percent-encoded) and `X-Package-Version`",
            body = PackageFile,
            content_type = "application/octet-stream",
        ),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn read(
    State(service): State<Service>,
    user: AuthUser,
    Path(resource): Path<Uuid>,
) -> Result<Response, ProblemDetails> {
    let package = service.read(user.0, resource).await?;
    Ok(file_response(package))
}

fn file_response(package: StoredPackage) -> Response {
    let mut response = package.bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if let Ok(value) = HeaderValue::from_str(&percent_encode(&package.name)) {
        headers.insert(NAME_HEADER, value);
    }
    headers.insert(VERSION_HEADER, HeaderValue::from(package.version));
    response
}

#[utoipa::path(
    put,
    path = "/{resource_id}",
    operation_id = "writePackage",
    tag = "storage",
    security(("bearer" = [])),
    params(("resource_id" = Uuid, Path), WriteQuery),
    request_body(
        content = PackageFile,
        content_type = "application/octet-stream",
        description = "The file, at most 64 MiB",
    ),
    responses(
        (status = 200, description = "Saved", body = WriteResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "Changed since it was read", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 413, description = "File too large", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn write(
    State(service): State<Service>,
    user: AuthUser,
    Path(resource): Path<Uuid>,
    Query(query): Query<WriteQuery>,
    body: Bytes,
) -> Result<Json<WriteResponse>, ProblemDetails> {
    let saved = service
        .write(
            user.0,
            resource,
            &query.name,
            &body,
            query.base_version,
            query.merge,
        )
        .await?;
    Ok(Json(WriteResponse {
        version: saved.version,
        merged: saved.merged,
    }))
}

#[utoipa::path(
    get,
    path = "/{resource_id}/versions",
    operation_id = "listPackageVersions",
    tag = "storage",
    security(("bearer" = [])),
    params(("resource_id" = Uuid, Path)),
    responses(
        (status = 200, description = "Kept versions, newest first", body = VersionListResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn versions(
    State(service): State<Service>,
    user: AuthUser,
    Path(resource): Path<Uuid>,
) -> Result<Json<VersionListResponse>, ProblemDetails> {
    let list = service.history(user.0, resource).await?;
    Ok(Json(list.into()))
}

#[utoipa::path(
    get,
    path = "/{resource_id}/versions/{version}",
    operation_id = "readPackageVersion",
    tag = "storage",
    security(("bearer" = [])),
    params(("resource_id" = Uuid, Path), ("version" = i64, Path)),
    responses(
        (
            status = 200,
            description = "The file as of that version; name and version are in `X-Package-Name` (percent-encoded) and `X-Package-Version`",
            body = PackageFile,
            content_type = "application/octet-stream",
        ),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn read_version(
    State(service): State<Service>,
    user: AuthUser,
    Path((resource, version)): Path<(Uuid, i64)>,
) -> Result<Response, ProblemDetails> {
    let package = service.read_version(user.0, resource, version).await?;
    Ok(file_response(package))
}

#[utoipa::path(
    post,
    path = "/{resource_id}/versions/{version}/restore",
    operation_id = "restorePackageVersion",
    tag = "storage",
    security(("bearer" = [])),
    params(("resource_id" = Uuid, Path), ("version" = i64, Path)),
    responses(
        (status = 200, description = "Restored as a new version", body = WriteResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn restore(
    State(service): State<Service>,
    user: AuthUser,
    Path((resource, version)): Path<(Uuid, i64)>,
) -> Result<Json<WriteResponse>, ProblemDetails> {
    let saved = service.restore(user.0, resource, version).await?;
    Ok(Json(WriteResponse {
        version: saved.version,
        merged: false,
    }))
}

#[derive(OpenApi)]
#[openapi(paths(read, write, versions, read_version, restore))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route(
            "/{resource_id}",
            get(read)
                .put(write)
                .layer(DefaultBodyLimit::max(MAX_PACKAGE_BYTES)),
        )
        .route("/{resource_id}/versions", get(versions))
        .route("/{resource_id}/versions/{version}", get(read_version))
        .route("/{resource_id}/versions/{version}/restore", post(restore))
        .with_state(service)
}
