use axum::{
    extract::{Json, State},
    http::{StatusCode, HeaderMap},
};
use battld_common::*;

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

pub async fn authenticate_request(
    session_cache: &crate::session_cache::SessionCache,
    headers: &HeaderMap,
) -> Result<i64, StatusCode> {
    let session_token = headers.get(HEADER_AUTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    session_cache
        .verify_session(session_token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub fn verify_signature_for_nonce(
    player: &crate::database::PlayerRecord,
    encrypted_token: &str,
    nonce: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign};
    use rsa::sha2::Sha256;
    use base64::{Engine as _, engine::general_purpose};
    use sha2::Digest;

    let public_key = match RsaPublicKey::from_pkcs1_pem(&player.public_key) {
        Ok(key) => key,
        Err(_) => RsaPublicKey::from_public_key_pem(&player.public_key)?,
    };

    let signature = general_purpose::STANDARD.decode(encrypted_token)?;

    let mut hasher = Sha256::new();
    hasher.update(nonce.as_bytes());
    let hashed = hasher.finalize();

    let padding = Pkcs1v15Sign::new::<Sha256>();
    let verification_result = public_key.verify(padding, &hashed, &signature);

    Ok(verification_result.is_ok())
}