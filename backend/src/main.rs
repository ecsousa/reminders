mod config;
mod db;
mod handlers;
mod mcp;
mod middleware;
mod models;

use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use reqwest::Client;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::handlers::AppState;

pub fn build_app(app_state: Arc<AppState>) -> Router {
    let config = app_state.config.clone();

    let api_routes = Router::new()
        .route("/user-info", get(handlers::get_user_info))
        .route(
            "/reminders",
            post(handlers::create_reminder).get(handlers::get_reminders),
        )
        .route("/reminders/:id", delete(handlers::delete_reminder))
        .route("/trigger-reminders", put(handlers::trigger_reminders))
        .layer(axum_middleware::from_fn(middleware::auth_middleware))
        .layer(axum::extract::Extension(config));

    let mcp_routes = Router::new()
        .route(
            "/mcp",
            post(mcp::handle_mcp_post)
                .get(mcp::handle_mcp_get)
                .delete(mcp::handle_mcp_delete),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(mcp::oauth_protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(mcp::oauth_protected_resource_metadata),
        )
        .route(
            "/mcp/.well-known/oauth-protected-resource",
            get(mcp::oauth_protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(mcp::oauth_authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server/mcp",
            get(mcp::oauth_authorization_server_metadata),
        )
        .route(
            "/mcp/.well-known/oauth-authorization-server",
            get(mcp::oauth_authorization_server_metadata),
        )
        .route(
            "/.well-known/openid-configuration",
            get(mcp::oauth_authorization_server_metadata),
        )
        .route(
            "/mcp/.well-known/openid-configuration",
            get(mcp::oauth_authorization_server_metadata),
        )
        .route(
            "/mcp/oauth/authorize",
            get(mcp::oauth_authorize).post(mcp::oauth_authorize),
        )
        .route(
            "/oauth/authorize",
            get(mcp::oauth_authorize).post(mcp::oauth_authorize),
        )
        .route(
            "/authorize",
            get(mcp::oauth_authorize).post(mcp::oauth_authorize),
        )
        .route("/mcp/oauth/token", post(mcp::oauth_token))
        .route("/oauth/token", post(mcp::oauth_token))
        .route("/token", post(mcp::oauth_token))
        .layer(CorsLayer::permissive());

    let static_dir = ServeDir::new("public").not_found_service(ServeFile::new("public/index.html"));

    Router::new()
        .nest("/api", api_routes)
        .merge(mcp_routes)
        .fallback_service(static_dir)
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reminders_backend=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();

    let db_conn = db::init_db(&config.db_folder).await;

    let web_client = Client::builder()
        .build()
        .expect("Failed to create reqwest client");

    let app_state = Arc::new(AppState {
        db: db_conn,
        config: config.clone(),
        web_client,
    });

    let app = build_app(app_state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
