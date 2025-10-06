use std::time::{SystemTime, UNIX_EPOCH};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
    
pub const HEADER_PLAYER_ID: &str = "x-player-id";
pub const HEADER_AUTH: &str = "authorization";

pub fn global_seed() -> u64 {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    unix_time / 3600
}

pub fn not_so_secret() -> (String, u64) {
    let seed = global_seed();

    let mut rng = StdRng::seed_from_u64(seed);

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let random_string: String = (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    (random_string, seed)
}