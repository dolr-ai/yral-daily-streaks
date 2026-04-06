use crate::auth::{init_jwt, JwtDetails};
use crate::config::AppConfig;
use crate::utils::error::{Error, Result};
use crate::utils::yral_auth_jwt::YralAuthJwt;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_details: JwtDetails,
    pub yral_auth_jwt: YralAuthJwt,
}

impl AppState {
    pub async fn new(app_config: &AppConfig) -> Result<Self> {
        let db_pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&app_config.pg_database_url)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))?;

        Ok(AppState {
            db: db_pool,
            jwt_details: init_jwt(app_config)?,
            yral_auth_jwt: YralAuthJwt::init(app_config.yral_auth_public_key.clone())?,
        })
    }
}
