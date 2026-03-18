use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use yral_or_not::config::AppConfig;
use yral_or_not::state::AppState;
use yral_or_not::utils::error::*;

async fn main_impl() -> Result<()> {
    let conf = AppConfig::load()?;

    let state = Arc::new(AppState::new(&conf).await?);

    // Build the application router with all routes defined here
    let app = Router::new()
        // API routes
        // OpenAPI/Swagger UI routes
        .route("/explorer/{*tail}", get(services::openapi::get_swagger))
        .route("/explorer/", get(services::openapi::get_swagger_root))
        .route("/healthz", get(api::handlers::healthz))
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
