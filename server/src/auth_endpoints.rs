use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{AppState, repository};
use battld_common::api::*;

pub async fn request_challenge(
    State(state): State<AppState>,
    Json(request): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    let player_record = state.db.get_player_by_id(request.player_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    if player_record.public_key_hint != request.public_key_hint {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let nonce = state.nonce_cache.create_nonce(request.player_id).await;

    Ok(Json(ChallengeResponse {
        nonce,
        expires_in: 60,
    }))
}

pub async fn verify_challenge(
    State(state): State<AppState>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    state.nonce_cache
        .verify_and_consume(&request.nonce, request.player_id)
        .await
        .map_err(|e| {
            println!("Nonce verification failed: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    let player_record = state.db.get_player_by_id(request.player_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    if !crate::auth::verify_signature_for_nonce(&player_record, &request.signature, &request.nonce)
        .unwrap_or(false)
    {
        println!("Signature verification failed for player {}", request.player_id);
        return Err(StatusCode::UNAUTHORIZED);
    }

    let player = repository::fetch_player(&state.db, request.player_id)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_token = state.session_cache.create_session(request.player_id).await;

    let expires_at = SystemTime::now() + std::time::Duration::from_secs(86400);
    let expires_at_timestamp = expires_at
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(Json(AuthResponse {
        session_token,
        expires_at: expires_at_timestamp.to_string(),
        player,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(request): Json<LogoutRequest>,
) -> Result<StatusCode, StatusCode> {
    state.session_cache.revoke_session(&request.session_token).await;
    Ok(StatusCode::OK)
}
