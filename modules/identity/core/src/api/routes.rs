use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use identity_contracts::UserId;
use platform::http::{AuthUser, ProblemDetails};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use workspace_contracts::WorkspaceExportPort;

use crate::{
    application::{GdprExport, IdentityError, IdentityService, RefreshGrant},
    domain::User,
};

/// Handler state: the use cases plus how the refresh cookie is issued.
#[derive(Clone)]
pub(crate) struct ApiState {
    pub(crate) service: Arc<IdentityService>,
    /// Adds `Secure` to the refresh cookie (required outside local development).
    pub(crate) cookie_secure: bool,
    /// Port for GDPR data export (workspace-owned files).
    pub(crate) workspace_export: Arc<dyn WorkspaceExportPort>,
}

const REFRESH_COOKIE: &str = "refresh_token";
/// The cookie is only sent to the auth endpoints that need it.
const REFRESH_COOKIE_PATH: &str = "/api/v1/auth";
/// Cookie-authenticated endpoints require this header. A cross-site request cannot set it
/// without a CORS preflight (which this API never approves), so it works as a CSRF guard on top
/// of `SameSite=Strict`.
const CSRF_HEADER: &str = "x-requested-with";
const CSRF_VALUE: &str = "office";

fn cookie_header(value: &str, max_age: u64, secure: bool) -> Option<HeaderValue> {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{REFRESH_COOKIE}={value}; HttpOnly; SameSite=Strict; Path={REFRESH_COOKIE_PATH}; Max-Age={max_age}{secure}"
    ))
    .ok()
}

fn set_cookie(grant: &RefreshGrant, secure: bool) -> Option<HeaderValue> {
    cookie_header(&grant.token, grant.max_age_secs, secure)
}

fn clear_cookie(secure: bool) -> Option<HeaderValue> {
    cookie_header("", 0, secure)
}

fn presented_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().strip_prefix(REFRESH_COOKIE)?.strip_prefix('='))
        .find(|token| !token.is_empty())
}

fn require_csrf_header(headers: &HeaderMap) -> Result<(), ProblemDetails> {
    if headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok()) == Some(CSRF_VALUE) {
        Ok(())
    } else {
        Err(ProblemDetails::new(
            StatusCode::FORBIDDEN,
            "csrf_rejected",
            "Missing request header",
        ))
    }
}

fn with_cookie(cookie: Option<HeaderValue>, response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

#[derive(Deserialize, ToSchema)]
struct RegisterRequest {
    email: String,
    display_name: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize, ToSchema)]
struct UserResponse {
    #[schema(format = Uuid)]
    id: String,
    email: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deletion_scheduled_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
    user: UserResponse,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id().to_string(),
            email: user.email().as_str().to_owned(),
            display_name: user.display_name().as_str().to_owned(),
            avatar_url: user.avatar_url().map(|a| a.as_str().to_owned()),
            bio: user.bio().map(|b| b.as_str().to_owned()),
            deletion_scheduled_at: user.deletion_scheduled_at().map(|t| t.to_string()),
        }
    }
}

impl From<IdentityError> for ProblemDetails {
    fn from(err: IdentityError) -> Self {
        match err {
            IdentityError::Invalid(e) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            )
            .with_detail(e.to_string()),
            IdentityError::EmailTaken => Self::new(
                StatusCode::CONFLICT,
                "email_taken",
                "Email already registered",
            ),
            IdentityError::InvalidCredentials => Self::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid credentials",
            ),
            IdentityError::InvalidRefreshToken => Self::new(
                StatusCode::UNAUTHORIZED,
                "invalid_refresh_token",
                "Invalid refresh token",
            ),
            IdentityError::NotFound => Self::new(StatusCode::NOT_FOUND, "not_found", "Not found"),
            IdentityError::Internal => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Internal error",
            ),
            IdentityError::WrongPassword => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "wrong_password",
                "Current password is incorrect",
            ),
            IdentityError::DeletionAlreadyScheduled => Self::new(
                StatusCode::CONFLICT,
                "deletion_already_scheduled",
                "Account deletion is already scheduled",
            ),
            IdentityError::NoPendingDeletion => Self::new(
                StatusCode::CONFLICT,
                "no_pending_deletion",
                "No deletion is scheduled",
            ),
            IdentityError::AtLeastOneFieldRequired => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "at_least_one_field_required",
                "At least one field must be provided",
            ),
        }
    }
}

#[utoipa::path(
    post,
    path = "/register",
    operation_id = "register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Created", body = UserResponse),
        (status = 409, description = "Conflict", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn register(
    State(state): State<ApiState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ProblemDetails> {
    let user = state
        .service
        .register(&body.email, &body.display_name, &body.password)
        .await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(&user))))
}

#[utoipa::path(
    post,
    path = "/login",
    operation_id = "login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "OK", body = LoginResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn login(
    State(state): State<ApiState>,
    Json(body): Json<LoginRequest>,
) -> Result<Response, ProblemDetails> {
    let (user, token, grant) = state.service.login(&body.email, &body.password).await?;
    Ok(with_cookie(
        set_cookie(&grant, state.cookie_secure),
        Json(LoginResponse {
            access_token: token.value,
            token_type: "Bearer",
            expires_in: token.expires_in_secs,
            user: UserResponse::from(&user),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/refresh",
    operation_id = "refresh",
    tag = "auth",
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    responses(
        (status = 200, description = "New access token; the refresh cookie is rotated", body = LoginResponse),
        (status = 401, description = "No or invalid refresh cookie", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "Missing CSRF header", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn refresh(State(state): State<ApiState>, headers: HeaderMap) -> Result<Response, Response> {
    require_csrf_header(&headers).map_err(IntoResponse::into_response)?;
    let invalid = || {
        with_cookie(
            clear_cookie(state.cookie_secure),
            ProblemDetails::from(IdentityError::InvalidRefreshToken),
        )
    };
    let presented = presented_token(&headers).ok_or_else(invalid)?;
    match state.service.refresh(presented).await {
        Ok((user, token, grant)) => Ok(with_cookie(
            set_cookie(&grant, state.cookie_secure),
            Json(LoginResponse {
                access_token: token.value,
                token_type: "Bearer",
                expires_in: token.expires_in_secs,
                user: UserResponse::from(&user),
            }),
        )),
        Err(IdentityError::InvalidRefreshToken) => Err(invalid()),
        Err(other) => Err(ProblemDetails::from(other).into_response()),
    }
}

#[utoipa::path(
    post,
    path = "/logout",
    operation_id = "logout",
    tag = "auth",
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    responses(
        (status = 204, description = "Signed out; the refresh cookie is expired"),
        (status = 403, description = "Missing CSRF header", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn logout(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Response, ProblemDetails> {
    require_csrf_header(&headers)?;
    if let Some(presented) = presented_token(&headers) {
        state.service.logout(presented).await?;
    }
    Ok(with_cookie(
        clear_cookie(state.cookie_secure),
        StatusCode::NO_CONTENT,
    ))
}

#[utoipa::path(
    get,
    path = "/me",
    operation_id = "me",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "OK", body = UserResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn me(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
) -> Result<Json<UserResponse>, ProblemDetails> {
    let user = state.service.get(UserId::from_uuid(id)).await?;
    Ok(Json(UserResponse::from(&user)))
}

#[derive(Deserialize, ToSchema)]
struct UpdateProfileRequest {
    display_name: String,
    #[serde(default)]
    bio: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize, ToSchema)]
struct ScheduleDeletionRequest {
    /// Must exactly match the caller's display name (case-sensitive).
    confirm_display_name: String,
}

#[derive(Serialize, ToSchema)]
struct ScheduleDeletionResponse {
    execute_after: String,
}

#[derive(Serialize, ToSchema)]
struct GdprExportResponse {
    schema_version: u8,
    generated_at: String,
    #[serde(flatten)]
    export: GdprExport,
}

#[utoipa::path(
    patch,
    path = "/me",
    operation_id = "updateProfile",
    tag = "auth",
    security(("bearer" = [])),
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Updated", body = UserResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "CSRF rejected", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn update_profile(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
    headers: HeaderMap,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, ProblemDetails> {
    require_csrf_header(&headers)?;
    let user = state
        .service
        .update_profile(
            UserId::from_uuid(id),
            &body.display_name,
            body.bio.as_deref(),
            body.avatar_url.as_deref(),
        )
        .await?;
    Ok(Json(UserResponse::from(&user)))
}

#[utoipa::path(
    post,
    path = "/me/change-password",
    operation_id = "changePassword",
    tag = "auth",
    security(("bearer" = [])),
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed; all sessions revoked"),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "CSRF rejected", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "Wrong password or validation failed", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn change_password(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
    headers: HeaderMap,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ProblemDetails> {
    require_csrf_header(&headers)?;
    state
        .service
        .change_password(
            UserId::from_uuid(id),
            &body.current_password,
            &body.new_password,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/me/export",
    operation_id = "exportMyData",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "GDPR export JSON", body = GdprExportResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn export_my_data(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
) -> Result<Response, ProblemDetails> {
    let export = state
        .service
        .gdpr_export(UserId::from_uuid(id), state.workspace_export.as_ref())
        .await?;
    let body = GdprExportResponse {
        schema_version: 1,
        generated_at: time::OffsetDateTime::now_utc().to_string(),
        export,
    };
    let json = serde_json::to_string(&body).map_err(|_| {
        ProblemDetails::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Internal error",
        )
    })?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"my-data.json\"",
            ),
        ],
        json,
    )
        .into_response())
}

#[utoipa::path(
    delete,
    path = "/me",
    operation_id = "scheduleAccountDeletion",
    tag = "auth",
    security(("bearer" = [])),
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    request_body = ScheduleDeletionRequest,
    responses(
        (status = 200, description = "Deletion scheduled", body = ScheduleDeletionResponse),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "CSRF rejected or display name mismatch", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "Already scheduled", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn schedule_account_deletion(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
    headers: HeaderMap,
    Json(body): Json<ScheduleDeletionRequest>,
) -> Result<Json<ScheduleDeletionResponse>, ProblemDetails> {
    require_csrf_header(&headers)?;
    let user_id = UserId::from_uuid(id);
    let user = state.service.get(user_id).await?;
    if user.display_name().as_str() != body.confirm_display_name {
        return Err(ProblemDetails::new(
            StatusCode::FORBIDDEN,
            "display_name_mismatch",
            "Display name does not match",
        ));
    }
    let execute_after = state.service.schedule_deletion(user_id).await?;
    Ok(Json(ScheduleDeletionResponse {
        execute_after: execute_after.to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/me/cancel-deletion",
    operation_id = "cancelAccountDeletion",
    tag = "auth",
    security(("bearer" = [])),
    params(("x-requested-with" = String, Header, description = "Must be `office` (CSRF guard)")),
    responses(
        (status = 204, description = "Deletion cancelled"),
        (status = 401, description = "Not authenticated", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "CSRF rejected", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "No pending deletion", body = ProblemDetails, content_type = "application/problem+json"),
    ),
)]
async fn cancel_account_deletion(
    State(state): State<ApiState>,
    AuthUser(id): AuthUser,
    headers: HeaderMap,
) -> Result<StatusCode, ProblemDetails> {
    require_csrf_header(&headers)?;
    state.service.cancel_deletion(UserId::from_uuid(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn router(state: ApiState) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route(
            "/me",
            get(me)
                .patch(update_profile)
                .delete(schedule_account_deletion),
        )
        .route("/me/change-password", post(change_password))
        .route("/me/export", get(export_my_data))
        .route("/me/cancel-deletion", post(cancel_account_deletion))
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(paths(
    register,
    login,
    refresh,
    logout,
    me,
    update_profile,
    change_password,
    export_my_data,
    schedule_account_deletion,
    cancel_account_deletion,
))]
pub(crate) struct ApiDoc;
