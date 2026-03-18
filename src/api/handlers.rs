use crate::state::AppState;
use crate::utils::error::{Error, Result};
use crate::{
    error::ApiResult,
    types::CreateStreakRes,
    utils::error::{ErrorWrapper, NullOk, OkWrapper},
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use candid::Principal;
use std::sync::Arc;

#[utoipa::path(
    post,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = CreateStreakRes,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Created streak successfully", body = OkWrapper<CreateStreakRes>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<CreateStreakRes>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<CreateStreakRes>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<CreateStreakRes>)
    )
)]
pub async fn create_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
) -> Result<Json<ApiResult<CreateStreakRes>>> {
    todo!()
}

#[utoipa::path(
    get,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = CreateStreakRes,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak fetched successfully", body = OkWrapper<CreateStreakRes>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<CreateStreakRes>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<CreateStreakRes>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<CreateStreakRes>)
    )
)]
pub async fn get_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
) -> Result<Json<ApiResult<CreateStreakRes>>> {
    todo!()
}

#[utoipa::path(
    patch,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = CreateStreakRes,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak updated successfully", body = OkWrapper<CreateStreakRes>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<CreateStreakRes>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<CreateStreakRes>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<CreateStreakRes>)
    )
)]
pub async fn update_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
) -> Result<Json<ApiResult<CreateStreakRes>>> {
    todo!()
}
#[utoipa::path(
    delete,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = CreateStreakRes,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak deleted successfully", body = OkWrapper<CreateStreakRes>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<CreateStreakRes>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<CreateStreakRes>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<CreateStreakRes>)
    )
)]
pub async fn delete_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
) -> Result<Json<ApiResult<CreateStreakRes>>> {
    todo!()
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value)
    ),
    tag = "Health"
)]
pub async fn healthz() -> axum::response::Response {
    Json(serde_json::json!({"status": "ok"})).into_response()
}
