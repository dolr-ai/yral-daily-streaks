use crate::error::{ApiError, ApiResult};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ic_agent::export::PrincipalError;
use jsonwebtoken::errors as jwt_errors;
use redis::RedisError;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{env::VarError, ops::Deref};
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
    #[error("unknown error {0}")]
    Unknown(String),
    #[error("Environment variable error: {0}")]
    #[schema(value_type = VarErrorDetail)]
    EnvironmentVariable(#[from] VarError),
    #[error("Environment variable missing: {0}")]
    EnvironmentVariableMissing(String),
    #[error("failed to parse principal {0}")]
    #[schema(value_type = PrincipalErrorDetail)]
    InvalidPrincipal(#[from] PrincipalError),
    #[error("swagger ui error {0}")]
    SwaggerUi(String),
    #[error("invalid username, must be 3-15 alphanumeric characters")]
    InvalidUsername,
    #[error("Invalid email")]
    InvalidEmail(String),
}

impl From<&Error> for ApiResult<()> {
    fn from(value: &Error) -> Self {
        let err = match value {
            Error::IO(_) | Error::Config(_) => {
                log::warn!("internal error {value}");
                ApiError::Unknown("internal error, reported".into())
            }
            Error::Redis(e) => {
                log::warn!("redis error {e}");
                ApiError::Redis
            }
            Error::Deser(e) => {
                log::warn!("deserialization error {e}");
                ApiError::Deser
            }
            Error::Jwt(_) => ApiError::Jwt,
            Error::AuthTokenMissing => ApiError::AuthTokenMissing,
            Error::AuthTokenInvalid => ApiError::AuthToken,
            Error::Unknown(e) => ApiError::Unknown(e.clone()),
            Error::EnvironmentVariable(_) => ApiError::EnvironmentVariable,
            Error::EnvironmentVariableMissing(_) => ApiError::EnvironmentVariableMissing,
            Error::InvalidPrincipal(_) => ApiError::InvalidPrincipal,
            Error::SwaggerUi(e) => {
                log::warn!("swagger ui error {e}");
                ApiError::Unknown(format!("Swagger UI error: {}", e))
            }
            Error::InvalidUsername => ApiError::InvalidUsername,
            Error::InvalidEmail(email) => ApiError::InvalidEmail(email.clone()),
        };
        ApiResult::Err(err)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Implement IntoResponse for axum error handling
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let api_error = Result::from(&self);
        let status_code = self.status_code();

        (status_code, Json(api_error)).into_response()
    }
}
impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::IO(_)
            | Error::Config(_)
            | Error::Redis(_)
            | Error::Deser(_)
            | Error::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Jwt(_) | Error::AuthTokenInvalid | Error::AuthTokenMissing => {
                StatusCode::UNAUTHORIZED
            }
            Error::EnvironmentVariable(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::EnvironmentVariableMissing(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidPrincipal(_) | Error::InvalidEmail(_) | Error::InvalidUsername => {
                StatusCode::BAD_REQUEST
            }
            Error::SwaggerUi(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, ToSchema)]
pub enum IOErrorData {
    #[schema(example = "Os(\"Invalid OS error\")")]
    Os(String),
    #[schema(example = "Simple(\"Invalid simple error\")")]
    Simple(String),
    #[schema(example = "SimpleMessage(\"Invalid simple message error\")")]
    SimpleMessage(String),
    #[schema(example = "Custom(\"Invalid custom error\")")]
    Custom(String),
}

#[derive(Debug, ToSchema, Serialize)]
pub struct ConfigErrorDetail {
    #[schema(example = "Frozen")]
    pub kind: String,
    #[schema(example = "Configuration is frozen and no further mutations can be made.")]
    pub message: String,
}

impl From<config::ConfigError> for ConfigErrorDetail {
    fn from(e: config::ConfigError) -> Self {
        match e {
            config::ConfigError::Frozen => ConfigErrorDetail {
                kind: "Frozen".to_string(),
                message: "Configuration is frozen and no further mutations can be made.".to_string(),
            },
            config::ConfigError::NotFound(s) => ConfigErrorDetail {
                kind: "NotFound".to_string(),
                message: format!("Configuration property not found: {}", s),
            },
            config::ConfigError::PathParse(s) => ConfigErrorDetail {
                kind: "PathParse".to_string(),
                message: format!("Configuration path could not be parsed: {:?}", s),
            },
            config::ConfigError::FileParse { uri, cause } => ConfigErrorDetail {
                kind: "FileParse".to_string(),
                message: format!(
                    "Configuration file could not be parsed. URI: {:?}, Cause: {}",
                    uri,
                    cause.to_string()
                ),
            },
            config::ConfigError::Type {
                origin,
                unexpected,
                expected,
                key,
            } => ConfigErrorDetail {
                kind: "Type".to_string(),
                message: format!(
                    "Configuration type error. Origin: {:?}, Unexpected: {}, Expected: {}, Key: {:?}",
                    origin,
                    unexpected.to_string(),
                    expected,
                    key
                ),
            },
            config::ConfigError::Message(s) => ConfigErrorDetail {
                kind: "Message".to_string(),
                message: s,
            },
            config::ConfigError::Foreign(e) => ConfigErrorDetail {
                kind: "Foreign".to_string(),
                message: format!("Foreign error: {}", e.to_string()),
            },
        }
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct IdentityErrorDetail {
    #[schema(example = "Signature verification failed")]
    pub message: String,
}

impl From<yral_identity::Error> for IdentityErrorDetail {
    fn from(e: yral_identity::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

impl std::fmt::Display for IdentityErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct RedisErrorDetail {
    #[schema(example = "ResponseError")]
    pub kind: RedisErrorKind,
    #[schema(example = "Connection refused")]
    pub detail: String,
}

#[derive(Debug, ToSchema, Serialize)]
pub enum RedisErrorKind {
    ResponseError,
    ParseError,
    AuthenticationFailed,
    TypeError,
    ExecAbortError,
    BusyLoadingError,
    NoScriptError,
    InvalidClientConfig,
    Moved,
    Ask,
    TryAgain,
    ClusterDown,
    CrossSlot,
    MasterDown,
    IoError,
    ClientError,
    ExtensionError,
    ReadOnly,
    MasterNameNotFoundBySentinel,
    NoValidReplicasFoundBySentinel,
    EmptySentinelList,
    NotBusy,
    ClusterConnectionNotFound,
    Unknown,
}

impl From<redis::ErrorKind> for RedisErrorKind {
    fn from(e: redis::ErrorKind) -> Self {
        match e {
            redis::ErrorKind::AuthenticationFailed => RedisErrorKind::AuthenticationFailed,
            redis::ErrorKind::InvalidClientConfig => RedisErrorKind::InvalidClientConfig,
            redis::ErrorKind::MasterNameNotFoundBySentinel => {
                RedisErrorKind::MasterNameNotFoundBySentinel
            }
            redis::ErrorKind::NoValidReplicasFoundBySentinel => {
                RedisErrorKind::NoValidReplicasFoundBySentinel
            }
            redis::ErrorKind::EmptySentinelList => RedisErrorKind::EmptySentinelList,
            redis::ErrorKind::ClusterConnectionNotFound => {
                RedisErrorKind::ClusterConnectionNotFound
            }
            _ => RedisErrorKind::Unknown,
        }
    }
}

impl From<RedisError> for RedisErrorDetail {
    fn from(e: RedisError) -> Self {
        Self {
            kind: RedisErrorKind::from(e.kind()),
            detail: e.to_string(),
        }
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct SerdeJsonErrorDetail {
    #[schema(example = 1)]
    pub line: usize,
    #[schema(example = 1)]
    pub column: usize,
    #[schema(example = "EOF while parsing a value")]
    pub message: String,
}

impl From<serde_json::Error> for SerdeJsonErrorDetail {
    fn from(e: serde_json::Error) -> Self {
        Self {
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        }
    }
}

impl From<&serde_json::Error> for SerdeJsonErrorDetail {
    fn from(e: &serde_json::Error) -> Self {
        Self {
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        }
    }
}

impl std::fmt::Display for SerdeJsonErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {} column {}: {}",
            self.line, self.column, self.message
        )
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct JwtErrorDetail {
    #[schema(example = "InvalidToken")]
    pub kind: String,
    #[schema(example = "Expired token")]
    pub message: String,
}

impl From<jwt_errors::Error> for JwtErrorDetail {
    fn from(e: jwt_errors::Error) -> Self {
        let kind_str = match e.kind() {
            jwt_errors::ErrorKind::InvalidToken => "InvalidToken".to_string(),
            jwt_errors::ErrorKind::InvalidSignature => "InvalidSignature".to_string(),
            jwt_errors::ErrorKind::InvalidEcdsaKey => "InvalidEcdsaKey".to_string(),
            jwt_errors::ErrorKind::InvalidRsaKey(err) => format!("InvalidRsaKey: {}", err),
            jwt_errors::ErrorKind::RsaFailedSigning => "RsaFailedSigning".to_string(),
            jwt_errors::ErrorKind::InvalidAlgorithmName => "InvalidAlgorithmName".to_string(),
            jwt_errors::ErrorKind::InvalidKeyFormat => "InvalidKeyFormat".to_string(),
            jwt_errors::ErrorKind::MissingRequiredClaim(claim) => {
                format!("MissingRequiredClaim: {}", claim)
            }
            jwt_errors::ErrorKind::ExpiredSignature => "ExpiredSignature".to_string(),
            jwt_errors::ErrorKind::InvalidIssuer => "InvalidIssuer".to_string(),
            jwt_errors::ErrorKind::InvalidAudience => "InvalidAudience".to_string(),
            jwt_errors::ErrorKind::InvalidSubject => "InvalidSubject".to_string(),
            jwt_errors::ErrorKind::ImmatureSignature => "ImmatureSignature".to_string(),
            jwt_errors::ErrorKind::InvalidAlgorithm => "InvalidAlgorithm".to_string(),
            jwt_errors::ErrorKind::MissingAlgorithm => "MissingAlgorithm".to_string(),
            jwt_errors::ErrorKind::Base64(err) => format!("Base64: {}", err),
            jwt_errors::ErrorKind::Json(json_err) => {
                format!("Json: {}", SerdeJsonErrorDetail::from(json_err.deref()))
            }
            jwt_errors::ErrorKind::Utf8(err) => format!("Utf8: {}", err),
            jwt_errors::ErrorKind::Crypto(err) => format!("Crypto: {}", err),
            _ => "Unknown".to_string(),
        };
        Self {
            kind: kind_str,
            message: e.to_string(),
        }
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct VarErrorDetail {
    #[schema(example = "NotPresent")]
    pub kind: String,
    #[schema(example = "Environment variable not present, or not unicode")]
    pub message: String,
}

impl From<VarError> for VarErrorDetail {
    fn from(e: VarError) -> Self {
        match e {
            VarError::NotPresent => VarErrorDetail {
                kind: "NotPresent".to_string(),
                message: "Environment variable not present".to_string(),
            },
            VarError::NotUnicode(os_string) => VarErrorDetail {
                kind: "NotUnicode".to_string(),
                message: format!(
                    "Environment variable not unicode. Original value (lossy): {}",
                    os_string.to_string_lossy()
                ),
            },
        }
    }
}

#[derive(Debug, ToSchema, Serialize)]
pub struct PrincipalErrorDetail {
    #[schema(example = "BytesTooLong")]
    pub kind: String,
    #[schema(example = "Bytes is longer than 29 bytes.")]
    pub message: String,
}

impl From<PrincipalError> for PrincipalErrorDetail {
    fn from(e: PrincipalError) -> Self {
        match e {
            PrincipalError::BytesTooLong() => PrincipalErrorDetail {
                kind: "BytesTooLong".to_string(),
                message: "Bytes is longer than 29 bytes.".to_string(),
            },
            PrincipalError::InvalidBase32() => PrincipalErrorDetail {
                kind: "InvalidBase32".to_string(),
                message: "Text must be in valid Base32 encoding.".to_string(),
            },
            PrincipalError::TextTooShort() => PrincipalErrorDetail {
                kind: "TextTooShort".to_string(),
                message: "Text is too short.".to_string(),
            },
            PrincipalError::TextTooLong() => PrincipalErrorDetail {
                kind: "TextTooLong".to_string(),
                message: "Text is too long.".to_string(),
            },
            PrincipalError::CheckSequenceNotMatch() => PrincipalErrorDetail {
                kind: "CheckSequenceNotMatch".to_string(),
                message: "CRC32 check sequence doesn't match with calculated from Principal bytes."
                    .to_string(),
            },
            PrincipalError::AbnormalGrouped(principal) => PrincipalErrorDetail {
                kind: "AbnormalGrouped".to_string(),
                message: format!(
                    "Text should be separated by - (dash) every 5 characters: expected \"{}\"",
                    principal.to_text()
                ),
            },
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct ErrorWrapper<T: ToSchema> {
    Err: T,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct OkWrapper<T: ToSchema> {
    Ok: T,
}

#[derive(Debug, ToSchema, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NullOk {
    Ok: (),
}
