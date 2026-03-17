use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use candid::Principal;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Error, Debug, PartialEq, ToSchema)]
#[non_exhaustive]
pub enum ApiError {
    #[error("invalid signature provided")]
    InvalidSignature,
    #[error("internal error: redis")]
    Redis,
    #[error("internal error: deser")]
    Deser,
    #[error("jwt error - invalid token")]
    Jwt,
    #[error("invalid authentication token")]
    AuthToken,
    #[error("missing authentication token")]
    AuthTokenMissing,
    #[error("failed to delete keys (redis)")]
    DeleteKeys,
    #[error("streak data for principal not found")]
    StreakDataNotFound,
    #[error("unknown: {0}")]
    Unknown(String),
    #[error("invalid email: {0}")]
    InvalidEmail(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("environment variable not found")]
    EnvironmentVariable,
    #[error("environment variable missing")]
    EnvironmentVariableMissing,
    #[error("failed to mark user session as registered: {0}")]
    UserAlreadyRegistered(String),
    #[error("invalid principal")]
    InvalidPrincipal,
    #[error("failed to update session: {0}")]
    UpdateSession(String),
    #[error("invalid username, must be 3-15 alphanumeric characters")]
    InvalidUsername,
    #[error("duplicate username")]
    DuplicateUsername,
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, ToSchema)]
pub struct StreakData {
    #[schema(value_type = String)]
    pub user_canister_id: Principal,
    pub user_name: String,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub signup_at: Option<i64>,

    #[serde(default)]
    pub is_migrated: bool,
}