use crate::api::store::KvStore;
use crate::state::AppState;
use crate::types::DeleteStreakRes;
use crate::utils::error::{Error, Result};
use crate::{
    error::ApiResult,
    types::StreakResponse,
    utils::error::{ErrorWrapper, NullOk, OkWrapper},
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use candid::Principal;
use chrono::{NaiveDate, Utc};
use chrono_tz::Asia::Kolkata;
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = StreakResponse,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak fetched successfully", body = OkWrapper<StreakResponse>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<StreakResponse>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<StreakResponse>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<StreakResponse>)
    )
)]
pub async fn get_streak(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    headers: HeaderMap,
) -> Result<Json<ApiResult<StreakResponse>>> {
    let Some(auth_header) = headers.get("Authorization") else {
        return Err(Error::AuthTokenMissing);
    };

    let auth_jwt_token = auth_header
        .to_str()
        .map_err(|_| Error::AuthTokenInvalid)?
        .trim_start_matches("Bearer ");

    let _jwt_claim = state.yral_auth_jwt.verify_token(auth_jwt_token)?;
    let response = get_streak_impl(&state.dragonfly_redis, user_principal)
        .await
        .map_err(|e| e)?;

    Ok(Json(Ok(response)))
}

#[utoipa::path(
    post,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = StreakResponse,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak updated successfully", body = OkWrapper<StreakResponse>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<StreakResponse>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<StreakResponse>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<StreakResponse>)
    )
)]
pub async fn checkin(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    headers: HeaderMap,
) -> Result<Json<ApiResult<StreakResponse>>> {
    let Some(auth_header) = headers.get("Authorization") else {
        return Err(Error::AuthTokenMissing);
    };

    let auth_jwt_token = auth_header
        .to_str()
        .map_err(|_| Error::AuthTokenInvalid)?
        .trim_start_matches("Bearer ");

    let _jwt_claim = state.yral_auth_jwt.verify_token(auth_jwt_token)?;

    let response = checkin_impl(&state.dragonfly_redis, user_principal)
        .await
        .map_err(|e| e)?;

    Ok(Json(Ok(response)))
}

#[utoipa::path(
    delete,
    path = "/streak/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = StreakResponse,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Streak deleted successfully", body = OkWrapper<StreakResponse>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<StreakResponse>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<StreakResponse>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<StreakResponse>)
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

    let response = delete_streak_impl(&state.dragonfly_redis, user_principal)
        .await
        .map_err(|e| e)?;

    Ok(Json(Ok(response)))
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

pub async fn get_streak_impl<S: KvStore>(
    store: &S,
    user_principal: Principal,
) -> Result<StreakResponse> {
    let key = format!("daily_streaks:{}", user_principal.to_text());
    let fields = [
        "current_streak".to_string(),
        "last_checkin_date".to_string(),
    ];
    let data = store.hmget(&key, &fields).await?;

    Ok(StreakResponse {
        current_streak: data[0].clone(),
        last_checkin_date: data[1].clone(),
    })
}

pub async fn checkin_impl<S: KvStore>(
    store: &S,
    user_principal: Principal,
) -> Result<StreakResponse> {
    let key = format!("daily_streaks:{}", user_principal.to_text());
    let fields = [
        "current_streak".to_string(),
        "last_checkin_date".to_string(),
    ];
    let data = store.hmget(&key, &fields).await?;

    let current_streak: u64 = data[0]
        .as_deref()
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|_| Error::Unknown("Failed to parse current streak number".to_string()))?;
    let last_checkin_date = data[1].as_deref();

    let (current_streak, latest_date) = compute_streak(last_checkin_date, current_streak);
    store
        .hmset(
            &key,
            &[
                ("current_streak", current_streak.as_bytes()),
                ("last_checkin_date", latest_date.as_bytes()),
            ],
        )
        .await?;

    Ok(StreakResponse {
        current_streak: Some(current_streak),
        last_checkin_date: Some(latest_date),
    })
}

pub async fn delete_streak_impl<S: KvStore>(
    store: &S,
    user_principal: Principal,
) -> Result<DeleteStreakRes> {
    let key = format!("daily_streaks:{}", user_principal.to_text());
    let response = store.del(&key).await?;
    Ok(response)
}

pub fn compute_streak(last_checkin_date: Option<&str>, current: u64) -> (String, String) {
    let today = Utc::now().with_timezone(&Kolkata).date_naive();
    let last_checkin_date: Option<NaiveDate> =
        last_checkin_date.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

    match last_checkin_date {
        None => (1.to_string(), today.to_string()),
        Some(d) if d == today => (current.to_string(), today.to_string()),
        Some(d) if (today - d).num_days() == 1 => ((current + 1).to_string(), today.to_string()),
        _ => (1.to_string(), today.to_string()),
    }
}
