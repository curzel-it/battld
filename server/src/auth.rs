use axum::{
    extract::{Json, State},
    http::{StatusCode, HeaderMap},
};
use battld_common::*;

use crate::database::Database;
use crate::repository;
use crate::AppState;

pub async fn create_player(
    State(state): State<AppState>,
    Json(request): Json<CreatePlayerRequest>
) -> Result<Json<Player>, StatusCode> {
    let db = &state.db;
    println!("API: Creating new player '{}'", request.name);

    // Create player using repository
    let user_id = match repository::create_player(db, &request.name, &request.public_key_hint, &request.public_key).await {
        Some(id) => id,
        _ => {
            println!("Player creation failed!");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Fetch the created player using repository
    let player = match repository::fetch_player(db, user_id).await {
        Some(player) => {
            println!("Fetched created player: id={}", player.id);
            player
        },
        None => {
            println!("Failed to retrieve created player with ID {user_id}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    println!("API: Successfully created player '{}' with ID {}", request.name, user_id);

    Ok(Json(player))
}

// Helper function to authenticate and extract player ID from headers (NEW - uses session tokens)
pub async fn authenticate_request(
    session_cache: &crate::session_cache::SessionCache,
    headers: &HeaderMap,
) -> Result<i64, StatusCode> {
    // Extract Authorization header with format "Bearer <session_token>"
    let session_token = headers.get(HEADER_AUTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify session token (NEW WAY - no more time-based verification!)
    session_cache
        .verify_session(session_token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

// DEPRECATED: Legacy authentication for backward compatibility
#[deprecated(note = "Use session token authentication via authenticate_request instead")]
pub async fn authenticate_request_legacy(
    db: &Database,
    headers: &HeaderMap,
) -> Result<i64, StatusCode> {
    // Extract player ID from headers
    let player_id = headers.get(HEADER_PLAYER_ID)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract Authorization header with format "Bearer <encrypted_token>"
    let encrypted_token = headers.get(HEADER_AUTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify authentication
    match authenticate_user(db, player_id, encrypted_token).await {
        Ok(true) => Ok(player_id),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

// Helper function to verify a signature against a player's public key
pub fn verify_signature(
    player: &crate::database::PlayerRecord,
    encrypted_token: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign};
    use rsa::sha2::Sha256;
    use base64::{Engine as _, engine::general_purpose};
    use sha2::Digest;

    // Generate the current expected random string using shared logic
    let (expected_random_string, _seed) = not_so_secret();

    // Decode the player's public key
    let public_key = match RsaPublicKey::from_pkcs1_pem(&player.public_key) {
        Ok(key) => key,
        Err(_) => RsaPublicKey::from_public_key_pem(&player.public_key)?,
    };

    // Decode the base64 signature
    let signature = general_purpose::STANDARD.decode(encrypted_token)?;

    // Hash the expected random string first, then verify
    let mut hasher = Sha256::new();
    hasher.update(expected_random_string.as_bytes());
    let hashed = hasher.finalize();

    // Verify the signature using PKCS1v15 with SHA256
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let verification_result = public_key.verify(padding, &hashed, &signature);

    Ok(verification_result.is_ok())
}

// New function: verify signature against arbitrary nonce (not time-based)
pub fn verify_signature_for_nonce(
    player: &crate::database::PlayerRecord,
    encrypted_token: &str,
    nonce: &str, // The nonce we want to verify against
) -> Result<bool, Box<dyn std::error::Error>> {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign};
    use rsa::sha2::Sha256;
    use base64::{Engine as _, engine::general_purpose};
    use sha2::Digest;

    // Decode the player's public key
    let public_key = match RsaPublicKey::from_pkcs1_pem(&player.public_key) {
        Ok(key) => key,
        Err(_) => RsaPublicKey::from_public_key_pem(&player.public_key)?,
    };

    // Decode the base64 signature
    let signature = general_purpose::STANDARD.decode(encrypted_token)?;

    // Hash the nonce (not time-based string!)
    let mut hasher = Sha256::new();
    hasher.update(nonce.as_bytes());
    let hashed = hasher.finalize();

    // Verify the signature using PKCS1v15 with SHA256
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let verification_result = public_key.verify(padding, &hashed, &signature);

    Ok(verification_result.is_ok())
}

// Authentication function to verify encrypted random string
async fn authenticate_user(
    db: &Database,
    user_id: i64,
    encrypted_token: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign};
    use rsa::sha2::Sha256;
    use base64::{Engine as _, engine::general_purpose};

    println!("DEBUG: Authenticating user ID: {user_id}");

    // Get the player from repository to retrieve their public key
    let public_key = match repository::public_key_from_player_id(db, user_id).await {
        Some(public_key) => public_key,
        _ => return Ok(false),
    };

    // Generate the current expected random string using shared logic
    let (expected_random_string, seed) = not_so_secret();
    println!("DEBUG: Expected random string: {expected_random_string} (seed: {seed})");
    println!("DEBUG: Received signature: {encrypted_token}");

    // Decode the player's public key (try PKCS#1 format first, then PKCS#8)
    let public_key = match RsaPublicKey::from_pkcs1_pem(&public_key) {
        Ok(key) => {
            println!("DEBUG: Successfully decoded public key (PKCS#1 format)");
            key
        },
        Err(_) => {
            // Try PKCS#8 format as fallback
            match RsaPublicKey::from_public_key_pem(&public_key) {
                Ok(key) => {
                    println!("DEBUG: Successfully decoded public key (PKCS#8 format)");
                    key
                },
                Err(e) => {
                    println!("DEBUG: Failed to decode public key in both PKCS#1 and PKCS#8 formats: {e}");
                    return Ok(false);
                },
            }
        },
    };

    // Decode the base64 signature
    let signature = match general_purpose::STANDARD.decode(encrypted_token) {
        Ok(data) => {
            println!("DEBUG: Successfully decoded signature, length: {}", data.len());
            data
        },
        Err(e) => {
            println!("DEBUG: Failed to decode base64 signature: {e}");
            return Ok(false);
        },
    };

    // Hash the expected random string first, then verify
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(expected_random_string.as_bytes());
    let hashed = hasher.finalize();

    // Verify the signature using PKCS1v15 with SHA256
    let padding = Pkcs1v15Sign::new::<Sha256>();
    let verification_result = public_key.verify(padding, &hashed, &signature);

    println!("DEBUG: Signature verification result: {verification_result:?}");

    Ok(verification_result.is_ok())
}