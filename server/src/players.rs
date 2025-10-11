use axum::{
    extract::{State, Json},
    http::{StatusCode, HeaderMap},
};
use battld_common::{games::matches::Match, *};

use crate::{repository, auth, AppState};

pub async fn get_player(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Player>, StatusCode> {
    let player_id = auth::authenticate_request(&state.session_cache, &headers).await?;
    let db = &state.db;
    println!("API: Getting player {player_id}");

    match repository::fetch_player(db, player_id).await {
        Some(player) => {
            println!("API: Successfully fetched player {player_id}");
            Ok(Json(player))
        },
        None => {
            println!("API: Player {player_id} not found");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn post_player(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Player>, StatusCode> {
    get_player(State(state), headers).await
}

pub async fn get_player_by_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>
) -> Result<Json<Player>, StatusCode> {
    let _authenticated_player_id = auth::authenticate_request(&state.session_cache, &headers).await?;
    let db = &state.db;

    match repository::fetch_player(db, id).await {
        Some(player) => {
            println!("API: Successfully fetched player {id}");
            Ok(Json(player))
        },
        None => {
            println!("API: Player {id} not found");
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn get_active_matches(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Match>>, StatusCode> {
    println!("API: GET /matches/active request received");
    let player_id = match auth::authenticate_request(&state.session_cache, &headers).await {
        Ok(id) => {
            println!("API: Authentication successful for player {id}");
            id
        }
        Err(e) => {
            println!("API: Authentication failed: {e:?}");
            return Err(e);
        }
    };
    let db = &state.db;
    println!("API: Getting active matches for player {player_id}");

    // Get active match for this player
    if let Some(match_record) = db.get_active_match_for_player(player_id).await {
        if let Some(match_data) = match_record.to_match() {
            println!("API: Found active match {} for player {player_id}", match_data.id);
            return Ok(Json(vec![match_data]));
        }
    }

    println!("API: No active matches for player {player_id}");
    Ok(Json(vec![]))
}

