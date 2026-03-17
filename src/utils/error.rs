use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use redis::RedisError;
use std::env::VarError;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Error, Debug, ToSchema)]
pub enum Error {
    #[error(transparent)]
    #[schema(value_type = IOErrorData)]
    IO(#[from] std::io::Error),
    #[error("failed to load config {0}")]
    #[schema(value_type = ConfigErrorDetail)]
    Config(#[from] config::ConfigError),
    #[error("{0}")]
    #[schema(value_type = IdentityErrorDetail)]
    Identity(#[from] yral_identity::Error),
    #[error("{0}")]
    #[schema(value_type = RedisErrorDetail)]
    Redis(#[from] RedisError),
    #[error("failed to deserialize json {0}")]
    #[schema(value_type = SerdeJsonErrorDetail)]
    Deser(#[from] serde_json::Error),
    #[error("jwt {0}")]
    #[schema(value_type = JwtErrorDetail)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("auth token missing")]
    AuthTokenMissing,
    #[error("auth token invalid")]
    AuthTokenInvalid,
    #[error("firebase api error {0}")]
    FirebaseApiErr(String),
    #[error("unknown error {0}")]
    Unknown(String),
    #[error("Environment variable error: {0}")]
    #[schema(value_type = VarErrorDetail)]
    EnvironmentVariable(#[from] VarError),
    #[error("Environment variable missing: {0}")]
    EnvironmentVariableMissing(String),
    #[error("failed to mark user sessin as registered")]
    UserAlreadyRegistered(String),
    #[error("failed to initialize backend admin ic agent")]
    BackendAdminIdentityInvalid(String),
    #[error("failed to parse principal {0}")]
    #[schema(value_type = PrincipalErrorDetail)]
    InvalidPrincipal(#[from] PrincipalError),
    #[error("failed to update session: {0}")]
    UpdateSession(String),
    #[error("swagger ui error {0}")]
    SwaggerUi(String),
    #[error("invalid username, must be 3-15 alphanumeric characters")]
    InvalidUsername,
    #[error("duplicate username")]
    DuplicateUsername,
    #[error("Invalid email")]
    InvalidEmail(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;