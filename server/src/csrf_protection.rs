use axum::{
    extract::Request,
    http::{StatusCode, Method},
    middleware::Next,
    response::Response,
};

/// CSRF protection middleware that requires a custom header for non-GET requests
/// This prevents browsers from making cross-origin requests without CORS preflight
pub async fn csrf_protection_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip CSRF check for GET, HEAD, OPTIONS (safe methods)
    if matches!(request.method(), &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return Ok(next.run(request).await);
    }

    // Skip CSRF check for WebSocket upgrade requests
    if request.uri().path() == "/ws" {
        return Ok(next.run(request).await);
    }

    // Require custom header for all other requests (POST, PUT, DELETE, etc.)
    const REQUIRED_HEADER: &str = "x-battld-client";

    if request.headers().get(REQUIRED_HEADER).is_none() {
        println!("CSRF protection: Blocked request without {} header to {}", REQUIRED_HEADER, request.uri().path());
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
