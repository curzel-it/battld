use axum::{ extract::Request, middleware::Next };

pub async fn log_request_middleware(request: Request, next: Next) -> axum::response::Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    println!("Incoming request: {method} {uri}");
    let response = next.run(request).await;
    println!("Response status: {} for {} {}", response.status(), method, uri);
    response
}