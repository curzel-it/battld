use axum::{
    routing::{get, post},
    Router,
    middleware::{self},
    response::{Html, Redirect, IntoResponse},
    extract::Host,
    http::Uri,
};
use std::{sync::Arc, path::PathBuf, net::SocketAddr};
use tower_http::services::ServeDir;

mod auth;
mod database;
mod game_logic;
mod game_router;
mod games;
mod log_requests;
mod players;
mod rate_limit;
mod repository;
mod server_init;
mod stats;
mod websocket;

use database::Database;
use log_requests::log_request_middleware;
use websocket::ConnectionRegistry;

const HTTP_ADDR: &str = "0.0.0.0:8080";
const HTTPS_ADDR: &str = "0.0.0.0:443";
const DATABASE_URL: &str = "sqlite://game.db";

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub registry: Arc<ConnectionRegistry>,
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn redirect_to_https(Host(host): Host, uri: Uri) -> impl IntoResponse {
    // Remove port from host if present (we'll use standard HTTPS port 443)
    let host = host.split(':').next().unwrap_or(&host);

    let uri = format!("https://{}{}", host, uri.path());
    Redirect::permanent(&uri)
}

#[tokio::main]
async fn main() {
    println!("Tris Server starting...");

    dotenvy::dotenv().ok();

    let db = Database::new(DATABASE_URL).await.expect("Failed to connect to database");
    db.initialize().await.expect("Failed to initialize database schema");
    println!("Database initialized successfully");

    server_init::seed_users(db.pool()).await.expect("Failed to seed users");

    let state = AppState {
        db: Arc::new(db),
        registry: Arc::new(ConnectionRegistry::new()),
    };

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    // Create rate-limited API routes
    let api_routes = Router::new()
        .route("/player", post(auth::create_player))
        .route("/player", get(players::get_player))
        .route("/player/current", get(players::post_player))
        .route("/player/:id", get(players::get_player_by_id))
        .route("/matches/active", get(players::get_active_matches))
        .route("/stats", get(stats::get_stats))
        .route("/leaderboard", get(stats::get_leaderboard))
        .layer(rate_limit::create_rate_limiter())
        .with_state(state.clone());

    let app = Router::new()
        .route("/", get(serve_index))
        .merge(api_routes)
        .route("/ws", get(websocket::ws_handler))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(middleware::from_fn(log_request_middleware))
        .with_state(state);

    // Check if SSL certificates are available
    let ssl_cert_path = std::env::var("SSL_CERT_PATH").ok();
    let ssl_key_path = std::env::var("SSL_KEY_PATH").ok();

    match (ssl_cert_path, ssl_key_path) {
        (Some(cert_path), Some(key_path)) => {
            println!("SSL certificates found, starting HTTPS server...");

            // Create redirect router for HTTP -> HTTPS
            let redirect_app = Router::new()
                .fallback(redirect_to_https);

            // Start HTTP redirect server
            let http_future = async {
                let listener = tokio::net::TcpListener::bind(HTTP_ADDR).await.unwrap();
                println!("HTTP redirect server running on {HTTP_ADDR}");
                axum::serve(listener, redirect_app).await.unwrap();
            };

            // Start HTTPS server
            let https_future = async {
                let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    PathBuf::from(&cert_path),
                    PathBuf::from(&key_path),
                )
                .await
                .expect("Failed to load SSL certificates");

                println!("HTTPS server running on {HTTPS_ADDR}");
                axum_server::bind_rustls(HTTPS_ADDR.parse().unwrap(), config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .unwrap();
            };

            // Run both servers concurrently
            tokio::join!(http_future, https_future);
        }
        _ => {
            println!("No SSL certificates found, starting HTTP-only server...");
            let listener = tokio::net::TcpListener::bind(HTTP_ADDR).await.unwrap();
            println!("Server running on {HTTP_ADDR}");
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
        }
    }
}
