use crate::api::store::StreakStore;
use crate::state::AppState;
use crate::types::DeleteStreakRes;
use crate::utils::error::{Error, Result};
use crate::{
    error::ApiResult,
    types::StreakResponse,
    utils::error::{ErrorWrapper, OkWrapper},
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
    let response = get_streak_impl(&state.db, user_principal).await?;

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

    let response = checkin_impl(&state.db, user_principal).await?;

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

pub async fn get_streak_impl<S: StreakStore>(
    store: &S,
    user_principal: Principal,
) -> Result<StreakResponse> {
    let principal_str = user_principal.to_text();
    let data = store.get_streak(&principal_str).await?;
    Ok(data.unwrap_or(StreakResponse {
        current_streak: None,
        last_checkin_date: None,
    }))
}

pub async fn checkin_impl<S: StreakStore>(
    store: &S,
    user_principal: Principal,
) -> Result<StreakResponse> {
    let principal_str = user_principal.to_text();
    let current_streak_data = store.get_streak(&principal_str).await?;

    let (current_streak_num, last_checkin_date) = match current_streak_data {
        Some(data) => (
            data.current_streak
                .as_deref()
                .unwrap_or("1")
                .parse::<u64>()
                .map_err(|_| Error::DataParseError("Invalid streak number".to_string()))?,
            data.last_checkin_date,
        ),
        None => (1, None),
    };

    let (new_streak, latest_date) =
        compute_streak(last_checkin_date.as_deref(), current_streak_num);

    store
        .set_streak(&principal_str, &new_streak, &latest_date)
        .await?;

    Ok(StreakResponse {
        current_streak: Some(new_streak),
        last_checkin_date: Some(latest_date),
    })
}

pub async fn delete_streak_impl<S: StreakStore>(
    store: &S,
    user_principal: Principal,
) -> Result<DeleteStreakRes> {
    store.delete_streak(&user_principal.to_text()).await
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
