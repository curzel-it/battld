use axum::{
    extract::{State, Query},
    http::{StatusCode, HeaderMap},
    Json,
};
use serde::Deserialize;
use battld_common::{PlayerStats, LeaderboardResponse, LeaderboardEntry};

use crate::{auth, AppState};

#[derive(Deserialize)]
pub struct StatsQuery {
    player: Option<i64>,
}

pub async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StatsQuery>,
) -> Result<Json<PlayerStats>, StatusCode> {
    let authenticated_player_id = auth::authenticate_request(&state.db, &headers).await?;
    let target_player_id = params.player.unwrap_or(authenticated_player_id);

    let db = &state.db;

    // Query total, completed and dropped matches
    let stats: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as total,
            SUM(CASE WHEN in_progress = 0 AND outcome IS NOT NULL THEN 1 ELSE 0 END) as completed,
            SUM(CASE WHEN in_progress = 1 AND player2_id IS NOT NULL THEN 1 ELSE 0 END) as dropped
        FROM matches
        WHERE player1_id = ? OR player2_id = ?
        "#
    )
    .bind(target_player_id)
    .bind(target_player_id)
    .fetch_one(db.pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Query wins, losses, draws
    let outcomes: Vec<(Option<String>, i64, i64)> = sqlx::query_as(
        r#"
        SELECT outcome, player1_id, player2_id
        FROM matches
        WHERE (player1_id = ? OR player2_id = ?) AND in_progress = 0 AND outcome IS NOT NULL
        "#
    )
    .bind(target_player_id)
    .bind(target_player_id)
    .fetch_all(db.pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut won = 0i64;
    let mut lost = 0i64;
    let mut draw = 0i64;
    let mut score = 0i64;

    for (outcome, player1_id, _player2_id) in outcomes {
        if let Some(outcome) = outcome {
            let is_player1 = player1_id == target_player_id;
            match outcome.as_str() {
                "p1_win" if is_player1 => {
                    won += 1;
                    score += 3;
                },
                "p2_win" if !is_player1 => {
                    won += 1;
                    score += 3;
                },
                "draw" => {
                    draw += 1;
                    score += 1;
                },
                _ => {
                    lost += 1;
                    score -= 1;
                }
            }
        }
    }

    Ok(Json(PlayerStats {
        player_id: target_player_id,
        won,
        lost,
        draw,
        dropped: stats.2,
        total: stats.0,
        score,
    }))
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn get_leaderboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, StatusCode> {
    let _player_id = auth::authenticate_request(&state.db, &headers).await?;
    let db = &state.db;

    let limit = params.limit.unwrap_or(10).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    // Query players with score - simple select ordered by score
    #[derive(sqlx::FromRow)]
    struct LeaderboardRow {
        id: i64,
        name: String,
        score: i64,
    }

    // Get total count of players with score > 0
    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) as count FROM players WHERE score > 0")
        .fetch_one(db.pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get paginated leaderboard - simple query using pre-calculated scores
    let scores: Vec<LeaderboardRow> = sqlx::query_as(
        r#"
        SELECT id, name, score
        FROM players
        WHERE score > 0
        ORDER BY score DESC, id ASC
        LIMIT ? OFFSET ?
        "#
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db.pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entries: Vec<LeaderboardEntry> = scores
        .iter()
        .enumerate()
        .map(|(idx, r)| LeaderboardEntry {
            player_id: r.id,
            player_name: r.name.clone(),
            rank: (offset + idx as i64 + 1),
            score: r.score,
        })
        .collect();

    Ok(Json(LeaderboardResponse {
        entries,
        total_count: total_count.0,
    }))
}
