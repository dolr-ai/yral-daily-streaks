use crate::api::store::StreakStore;
use crate::state::AppState;
use crate::types::*;
use crate::utils::error::{Error, NullOk, Result};
use crate::{
    error::ApiResult,
    types::{DeleteStreakRes, StreakResponse},
    utils::error::ErrorWrapper,
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
    get,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak updated successfully", body = StreakResponse),
        (status = 400, description = "Invalid request", body = String),
        (status = 401, description = "Unauthorized", body = String),
        (status = 500, description = "Internal server error", body = String)
    )
)]
pub async fn checkin(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth_header = match headers.get("Authorization") {
        Some(header) => header,
        None => {
            crate::sentry_utils::capture_api_error(
                &Error::AuthTokenMissing,
                "/metadata/{user_principal}",
                Some(&user_principal.to_text()),
            );
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                "Missing Authorization Header",
            )
                .into_response();
        }
    };

    let auth_jwt_token = match auth_header.to_str() {
        Ok(t) => t.trim_start_matches("Bearer "),
        Err(_) => {
            crate::sentry_utils::capture_api_error(
                &Error::AuthTokenInvalid,
                "/metadata/{user_principal}",
                Some(&user_principal.to_text()),
            );
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid Header Encoding",
            )
                .into_response();
        }
    };

    if let Err(e) = state.yral_auth_jwt.verify_token(auth_jwt_token) {
        crate::sentry_utils::capture_api_error(
            &e,
            "/metadata/{user_principal}",
            Some(&user_principal.to_text()),
        );
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            format!("Unauthorized: {}", e),
        )
            .into_response();
    }

    match checkin_impl(&state.db, user_principal).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => {
            crate::sentry_utils::capture_api_error(
                &e,
                "/metadata/{user_principal}",
                Some(&user_principal.to_text()),
            );
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error: {}", e),
            )
                .into_response();
        }
    }
}

#[utoipa::path(
    delete,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak deleted successfully", body = NullOk),
        (status = 400, description = "Invalid request", body = ErrorWrapper<Error>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<Error>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<Error>)
    )
)]
pub async fn delete_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    headers: HeaderMap,
) -> Result<Json<ApiResult<DeleteStreakRes>>> {
    let token = headers
        .get("Authorization")
        .ok_or(Error::AuthTokenMissing)?
        .to_str()
        .map_err(|_| Error::AuthTokenInvalid)?;
    let token = token.trim_start_matches("Bearer ");

    // Verify JWT token
    crate::auth::verify_token(token, &state.jwt_details)?;
    delete_streak_impl(&state.db, user_principal).await?;

    Ok(Json(Ok(())))
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

pub async fn checkin_impl<S: StreakStore>(
    store: &S,
    user_principal: Principal,
) -> Result<StreakResponse> {
    let principal_str = user_principal.to_text();
    let now_ms = now_epoch_ms();

    let data = store.get_streak(&principal_str).await?;

    let (new_streak, just_incremented, streak_action, last_credited_at) = match data {
        None => compute_streak(None, 0, now_ms),
        Some(row) => compute_streak(Some(row.last_checkin_epoch_ms), row.current_streak, now_ms),
    };

    if just_incremented || streak_action == "reset" {
        store
            .set_streak(&principal_str, new_streak, last_credited_at)
            .await?;
    }

    Ok(build_response(
        principal_str,
        new_streak,
        just_incremented,
        streak_action,
        now_ms,
        last_credited_at,
    ))
}

pub async fn delete_streak_impl<S: StreakStore>(
    store: &S,
    user_principal: Principal,
) -> Result<DeleteStreakRes> {
    store.delete_streak(&user_principal.to_text()).await
}
