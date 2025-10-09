use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer, key_extractor::PeerIpKeyExtractor};
use governor::middleware::NoOpMiddleware;
use governor::clock::QuantaInstant;

/// Creates a rate limiting layer for API endpoints
/// Default: 10 requests per second per IP address
pub fn create_rate_limiter() -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    let requests_per_second = std::env::var("RATE_LIMIT_RockPaperScissors")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(requests_per_second)
            .burst_size((requests_per_second * 2) as u32)
            .finish()
            .expect("Failed to build rate limiter config"),
    );

    GovernorLayer {
        config: governor_conf,
    }
}
