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

fn parse_server_addrs() -> (String, String) {
    let server_url = std::env::var("SERVER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    // Parse the URL to extract protocol and port
    let url = server_url.trim();

    if url.starts_with("https://") {
        let without_protocol = url.trim_start_matches("https://");
        let port = if let Some(colon_pos) = without_protocol.find(':') {
            &without_protocol[colon_pos + 1..]
        } else {
            "443"
        };
        let http_port = "80";
        (format!("0.0.0.0:{}", http_port), format!("0.0.0.0:{}", port))
    } else {
        // http:// or no protocol
        let without_protocol = url.trim_start_matches("http://");
        let port = if let Some(colon_pos) = without_protocol.find(':') {
            &without_protocol[colon_pos + 1..]
        } else {
            "80"
        };
        (format!("0.0.0.0:{}", port), format!("0.0.0.0:443"))
    }
}

#[tokio::main]
async fn main() {
    println!("Tris Server starting...");

    dotenvy::dotenv().ok();

    let (http_addr, https_addr) = parse_server_addrs();

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
            let http_addr_clone = http_addr.clone();
            let http_future = async move {
                let listener = tokio::net::TcpListener::bind(&http_addr_clone).await.unwrap();
                println!("HTTP redirect server running on {}", http_addr_clone);
                axum::serve(listener, redirect_app).await.unwrap();
            };

            // Start HTTPS server
            let https_addr_clone = https_addr.clone();
            let https_future = async move {
                let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    PathBuf::from(&cert_path),
                    PathBuf::from(&key_path),
                )
                .await
                .expect("Failed to load SSL certificates");

                println!("HTTPS server running on {}", https_addr_clone);
                axum_server::bind_rustls(https_addr_clone.parse().unwrap(), config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await
                    .unwrap();
            };

            // Run both servers concurrently
            tokio::join!(http_future, https_future);
        }
        _ => {
            println!("No SSL certificates found, starting HTTP-only server...");
            let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
            println!("Server running on {}", http_addr);
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
        }
    }
}
