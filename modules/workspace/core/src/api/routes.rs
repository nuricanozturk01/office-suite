use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
};
use identity_contracts::UserId;
use platform::http::{AuthUser, ProblemDetails};
use serde::{Deserialize, Serialize};
use shared_kernel::Uuid;
use time::OffsetDateTime;
use utoipa::{IntoParams, OpenApi, ToSchema};
use workspace_contracts::{ResourceId, ResourceKind, Role};

use crate::{
    application::{ShareRecord, SharedItem, WorkspaceError, WorkspaceService},
    domain::ShareError,
};

type Service = Arc<WorkspaceService>;

/// Roles that can be granted through a share.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum GrantableRole {
    Viewer,
    Commenter,
    Editor,
}

impl From<GrantableRole> for Role {
    fn from(role: GrantableRole) -> Self {
        match role {
            GrantableRole::Viewer => Self::Viewer,
            GrantableRole::Commenter => Self::Commenter,
            GrantableRole::Editor => Self::Editor,
        }
    }
}

/// Effective role in responses.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum RoleDto {
    Viewer,
    Commenter,
    Editor,
    Owner,
}

impl From<Role> for RoleDto {
    fn from(role: Role) -> Self {
        match role {
            Role::Viewer => Self::Viewer,
            Role::Commenter => Self::Commenter,
            Role::Editor => Self::Editor,
            Role::Owner => Self::Owner,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum KindDto {
    Document,
    Spreadsheet,
    Presentation,
    Pdf,
}

impl From<ResourceKind> for KindDto {
    fn from(kind: ResourceKind) -> Self {
        match kind {
            ResourceKind::Document => Self::Document,
            ResourceKind::Spreadsheet => Self::Spreadsheet,
            ResourceKind::Presentation => Self::Presentation,
            ResourceKind::Pdf => Self::Pdf,
        }
    }
}

#[derive(Deserialize, ToSchema)]
struct ShareBody {
    email: String,
    role: GrantableRole,
}

#[derive(Serialize, ToSchema)]
struct ShareResponse {
    #[schema(format = Uuid)]
    user_id: String,
    email: String,
    role: RoleDto,
    #[schema(format = Uuid)]
    shared_by: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl From<&ShareRecord> for ShareResponse {
    fn from(r: &ShareRecord) -> Self {
        Self {
            user_id: r.grantee.to_string(),
            email: r.email.clone(),
            role: r.role.into(),
            shared_by: r.shared_by.to_string(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
struct SharesResponse {
    items: Vec<ShareResponse>,
}

#[derive(Serialize, ToSchema)]
struct SharedItemResponse {
    #[schema(format = Uuid)]
    id: String,
    kind: KindDto,
    title: String,
    role: RoleDto,
    #[schema(format = Uuid)]
    owner_id: String,
    #[serde(with = "time::serde::rfc3339")]
    shared_at: OffsetDateTime,
}

impl From<&SharedItem> for SharedItemResponse {
    fn from(i: &SharedItem) -> Self {
        Self {
            id: i.id.to_string(),
            kind: i.kind.into(),
            title: i.title.clone(),
            role: i.role.into(),
            owner_id: i.owner_id.to_string(),
            shared_at: i.shared_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
struct SharedListResponse {
    items: Vec<SharedItemResponse>,
    #[schema(format = Uuid)]
    next_cursor: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    /// `next_cursor` of the previous page.
    cursor: Option<Uuid>,
    /// Page size.
    #[param(minimum = 1)]
    limit: Option<u32>,
}

impl From<WorkspaceError> for ProblemDetails {
    fn from(err: WorkspaceError) -> Self {
        match err {
            WorkspaceError::NotFound => Self::new(StatusCode::NOT_FOUND, "not_found", "Not found"),
            WorkspaceError::UserNotFound => {
                Self::new(StatusCode::NOT_FOUND, "user_not_found", "User not found")
            }
            WorkspaceError::Invalid(ShareError::OwnerRoleNotGrantable) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_role",
                "The owner role cannot be granted",
            ),
            WorkspaceError::Invalid(ShareError::CannotShareWithOwner) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "cannot_share_with_owner",
                "Cannot share with the owner",
            ),
            WorkspaceError::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            ),
        }
    }
}

const fn actor(user: AuthUser) -> UserId {
    UserId::from_uuid(user.0)
}

#[utoipa::path(
    get,
    path = "/{id}/shares",
    operation_id = "listShares",
    tag = "workspace",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = SharesResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn list_shares(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<SharesResponse>, ProblemDetails> {
    let shares = service
        .list_shares(actor(user), ResourceId::from_uuid(id))
        .await?;
    Ok(Json(SharesResponse {
        items: shares.iter().map(ShareResponse::from).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/{id}/shares",
    operation_id = "shareFile",
    tag = "workspace",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = ShareBody,
    responses(
        (status = 200, description = "Shared (or role changed)", body = ShareResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Resource or user not found", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Invalid share", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn share(
    State(service): State<Service>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ShareBody>,
) -> Result<Json<ShareResponse>, ProblemDetails> {
    let record = service
        .share(
            actor(user),
            ResourceId::from_uuid(id),
            &body.email,
            body.role.into(),
        )
        .await?;
    Ok(Json(ShareResponse::from(&record)))
}

#[utoipa::path(
    delete,
    path = "/{id}/shares/{user_id}",
    operation_id = "revokeShare",
    tag = "workspace",
    security(("bearer" = [])),
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 204, description = "Access removed"),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "Not found", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn revoke(
    State(service): State<Service>,
    user: AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ProblemDetails> {
    service
        .revoke(
            actor(user),
            ResourceId::from_uuid(id),
            UserId::from_uuid(user_id),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/shared-with-me",
    operation_id = "listSharedWithMe",
    tag = "workspace",
    security(("bearer" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "OK", body = SharedListResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn shared_with_me(
    State(service): State<Service>,
    user: AuthUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<SharedListResponse>, ProblemDetails> {
    let page = service
        .shared_with_me(
            actor(user),
            query.cursor.map(ResourceId::from_uuid),
            query.limit,
        )
        .await?;
    Ok(Json(SharedListResponse {
        items: page.items.iter().map(SharedItemResponse::from).collect(),
        next_cursor: page.next_cursor.map(|c| c.to_string()),
    }))
}

#[derive(OpenApi)]
#[openapi(paths(list_shares, share, revoke, shared_with_me))]
pub(crate) struct ApiDoc;

pub(crate) fn router(service: Service) -> Router {
    Router::new()
        .route("/shared-with-me", get(shared_with_me))
        .route("/{id}/shares", get(list_shares).post(share))
        .route("/{id}/shares/{user_id}", delete(revoke))
        .with_state(service)
}
