use std::sync::Arc;

use axum::{
    routing::{delete, get, post, patch},
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use yral_or_not::config::AppConfig;
use yral_or_not::state::AppState;
use yral_or_not::utils::error::*;
use yral_or_not::api::handlers::*;
use yral_or_not::{get_swagger, get_swagger_root};

async fn main_impl() -> Result<()> {
    let conf = AppConfig::load()?;

    let state = Arc::new(AppState::new(&conf).await?);

    // Build the application router with all routes defined here
    let app = Router::new()
        // API routes
        .route("/streaks/{user_prinicipal}", get(get_streak))
        .route("/streaks/{user_prinicipal}", post(create_streak))
        .route("/streaks/{user_prinicipal}", patch(update_streak))
        .route("/streaks/{user_prinicipal}", delete(delete_streak))  
        // OpenAPI/Swagger UI routes
        .route("/explorer/{*tail}", get(get_swagger))
        .route("/explorer/", get(get_swagger_root))
        .route("/healthz", get(healthz))
        .layer(CorsLayer::permissive())
        // Add sentry middleware layer
        // .layer(sentry_tower_layer)
        // Add shared state
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(conf.bind_address)
        .await
        .map_err(|e| Error::IO(e))?;

    log::info!("Server starting on {}", conf.bind_address);

    axum::serve(listener, app).await.map_err(|e| Error::IO(e))?;

    Ok(())
}

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            main_impl().await.unwrap();
        });
}
