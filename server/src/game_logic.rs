use battld_common::{GameState, MatchOutcome, MatchEndReason, ServerMessage};
use crate::database::Database;

/// Represents a message to be sent to a specific player
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub player_id: i64,
    pub message: ServerMessage,
}

/// Handle resume match request - returns messages to send
pub async fn handle_resume_match_logic(
    player_id: i64,
    resumable_match_id: Option<i64>,
    db: &Database,
) -> Vec<OutgoingMessage> {
    let match_id = match resumable_match_id {
        Some(id) => id,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "No resumable match found".to_string(),
                },
            }];
        }
    };

    // Get the match data
    let match_record = match db.get_match_by_id(match_id).await {
        Some(m) => m,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "Match not found".to_string(),
                },
            }];
        }
    };

    let match_info = match match_record.to_match() {
        Some(m) => m,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "Failed to load match data".to_string(),
                },
            }];
        }
    };

    if !match_info.in_progress {
        return vec![OutgoingMessage {
            player_id,
            message: ServerMessage::Error {
                message: "Match is no longer active".to_string(),
            },
        }];
    }

    println!("Player {player_id} resumed match {match_id}");

    // Send GameStateUpdate to both players
    let opponent_id = if match_info.player1_id == player_id {
        match_info.player2_id
    } else {
        match_info.player1_id
    };

    vec![
        OutgoingMessage {
            player_id,
            message: ServerMessage::GameStateUpdate {
                match_data: match_info.clone(),
            },
        },
        OutgoingMessage {
            player_id: opponent_id,
            message: ServerMessage::GameStateUpdate {
                match_data: match_info,
            },
        },
    ]
}

/// Handle matchmaking request - returns messages to send
pub async fn handle_join_matchmaking_logic(
    player_id: i64,
    db: &Database,
) -> Vec<OutgoingMessage> {
    use rand::Rng;

    // Check if player already has an active match
    if let Some(match_record) = db.get_active_match_for_player(player_id).await {
        println!("Player {player_id} already in match {}", match_record.id);
        if let Some(match_info) = match_record.to_match() {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::GameStateUpdate {
                    match_data: match_info,
                },
            }];
        }
        return vec![];
    }

    // Try to find a waiting opponent
    if let Some(waiting_match) = db.find_waiting_match(player_id).await {
        let p1_id = waiting_match.player1_id;
        let p2_id = player_id;
        println!("Matching player {player_id} with waiting player {p1_id}");

        // Randomize who goes first
        let first_player = {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.5) { 1 } else { 2 }
        };

        // Initialize game state
        let game_state = GameState::new();
        let game_state_json = game_state.to_json();

        // Update the waiting match
        if (db.join_waiting_match(waiting_match.id, p2_id, first_player as i64, &game_state_json).await).is_ok() {
            if let Some(match_record) = db.get_match_by_id(waiting_match.id).await {
                if let Some(match_info) = match_record.to_match() {
                    // Notify both players
                    return vec![
                        OutgoingMessage {
                            player_id: p1_id,
                            message: ServerMessage::MatchFound {
                                match_data: match_info.clone(),
                            },
                        },
                        OutgoingMessage {
                            player_id: p2_id,
                            message: ServerMessage::MatchFound {
                                match_data: match_info,
                            },
                        },
                    ];
                }
            }
        }
    } else {
        // No opponent found, create a waiting match
        if (db.create_waiting_match(player_id).await).is_ok() {
            println!("Player {player_id} created waiting match");
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::WaitingForOpponent,
            }];
        }
    }

    vec![]
}

/// Handle a move request - returns messages to send
pub async fn handle_make_move_logic(
    player_id: i64,
    row: usize,
    col: usize,
    db: &Database,
) -> Vec<OutgoingMessage> {
    // Get active match for this player
    let match_record = match db.get_active_match_for_player(player_id).await {
        Some(m) => m,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "No active match found".to_string(),
                },
            }];
        }
    };

    let mut game_match = match match_record.to_match() {
        Some(m) => m,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "Failed to load match data".to_string(),
                },
            }];
        }
    };

    // Verify match is still in progress
    if !game_match.in_progress {
        return vec![OutgoingMessage {
            player_id,
            message: ServerMessage::Error {
                message: "Match already finished".to_string(),
            },
        }];
    }

    // Determine which player number the current player is
    let player_number = if game_match.player1_id == player_id {
        1
    } else if game_match.player2_id == player_id {
        2
    } else {
        return vec![OutgoingMessage {
            player_id,
            message: ServerMessage::Error {
                message: "Player not in this match".to_string(),
            },
        }];
    };

    // Verify it's this player's turn
    if game_match.current_player != player_number {
        return vec![OutgoingMessage {
            player_id,
            message: ServerMessage::Error {
                message: "Not your turn".to_string(),
            },
        }];
    }

    // Convert row/col to index and make the move
    let index = match GameState::coords_to_index(row, col) {
        Some(i) => i,
        None => {
            return vec![OutgoingMessage {
                player_id,
                message: ServerMessage::Error {
                    message: "Invalid coordinates".to_string(),
                },
            }];
        }
    };

    if let Err(e) = game_match.game_state.place_move(index, player_number) {
        return vec![OutgoingMessage {
            player_id,
            message: ServerMessage::Error {
                message: e,
            },
        }];
    }

    // Check for winner
    let outcome = if let Some(winner) = game_match.game_state.check_winner() {
        Some(if winner == 1 {
            MatchOutcome::Player1Win
        } else {
            MatchOutcome::Player2Win
        })
    } else if game_match.game_state.is_full() {
        Some(MatchOutcome::Draw)
    } else {
        None
    };

    // Switch to next player if game continues
    let next_player = if outcome.is_some() {
        game_match.current_player
    } else if game_match.current_player == 1 { 2 } else { 1 };

    let in_progress = outcome.is_none();
    let outcome_str = outcome.as_ref().map(|o| o.to_string());

    // Update match in database
    if (db.update_match(
        game_match.id,
        next_player as i64,
        &game_match.game_state.to_json(),
        in_progress,
        outcome_str.as_deref(),
    ).await).is_ok() {
        game_match.current_player = next_player;
        game_match.in_progress = in_progress;
        game_match.outcome = outcome;

        println!("Player {player_id} made move at ({}, {}). Match {}: in_progress={}, outcome={:?}",
            row, col, game_match.id, in_progress, game_match.outcome);

        // If match ended, update player scores
        if !in_progress {
            if let Some(match_record) = db.get_match_by_id(game_match.id).await {
                let _ = db.update_player_scores_from_match(&match_record).await;
            }
        }

        let mut messages = vec![
            OutgoingMessage {
                player_id: game_match.player1_id,
                message: ServerMessage::GameStateUpdate {
                    match_data: game_match.clone(),
                },
            },
            OutgoingMessage {
                player_id: game_match.player2_id,
                message: ServerMessage::GameStateUpdate {
                    match_data: game_match.clone(),
                },
            },
        ];

        // If match ended, send MatchEnded (clients will close their own connections)
        if !in_progress {
            messages.push(OutgoingMessage {
                player_id: game_match.player1_id,
                message: ServerMessage::MatchEnded {
                    reason: MatchEndReason::Ended,
                },
            });
            messages.push(OutgoingMessage {
                player_id: game_match.player2_id,
                message: ServerMessage::MatchEnded {
                    reason: MatchEndReason::Ended,
                },
            });
        }

        return messages;
    }

    vec![]
}

/// Handle disconnect - returns messages to send and whether to start a disconnect timer
pub async fn handle_disconnect_logic(
    player_id: i64,
    db: &Database,
) -> (Vec<OutgoingMessage>, Option<i64>) {
    // Check if player is in matchmaking
    if let Some(waiting_match) = db.get_waiting_match_for_player(player_id).await {
        // Remove from matchmaking queue on disconnect
        let _ = db.delete_match(waiting_match.id).await;
        println!("Player {player_id} disconnected from matchmaking");
        return (vec![], None);
    }

    // Check if player has an active match
    let match_record = match db.get_active_match_for_player(player_id).await {
        Some(m) => m,
        None => return (vec![], None), // No active match
    };

    let game_match = match match_record.to_match() {
        Some(m) => m,
        None => return (vec![], None),
    };

    if !game_match.in_progress {
        return (vec![], None); // Match already finished
    }

    // Get opponent's ID
    let opponent_id = if game_match.player1_id == player_id {
        game_match.player2_id
    } else {
        game_match.player1_id
    };

    println!("Player {player_id} disconnected from active match {}, starting 10s grace period", game_match.id);

    // Notify opponent that this player disconnected
    let messages = vec![OutgoingMessage {
        player_id: opponent_id,
        message: ServerMessage::PlayerDisconnected { player_id },
    }];

    // Return messages and match_id to start timer
    (messages, Some(game_match.id))
}

/// Handle disconnect timeout - returns messages to send
pub async fn handle_disconnect_timeout_logic(
    player_id: i64,
    match_id: i64,
    db: &Database,
) -> Vec<OutgoingMessage> {
    // Get the match
    let match_record = match db.get_match_by_id(match_id).await {
        Some(m) => m,
        None => return vec![],
    };

    let game_match = match match_record.to_match() {
        Some(m) => m,
        None => return vec![],
    };

    if !game_match.in_progress {
        return vec![]; // Match already finished
    }

    // Get opponent's ID
    let opponent_id = if game_match.player1_id == player_id {
        game_match.player2_id
    } else {
        game_match.player1_id
    };

    // Mark match as draw due to disconnect timeout
    let _ = db.update_match(
        game_match.id,
        game_match.current_player as i64,
        &game_match.game_state.to_json(),
        false, // not in progress
        Some(&MatchOutcome::Draw.to_string()),
    ).await;

    println!("Player {player_id} failed to reconnect to match {match_id} within 10s - ending match");

    // Update player scores for the draw
    if let Some(match_record) = db.get_match_by_id(match_id).await {
        let _ = db.update_player_scores_from_match(&match_record).await;
    }

    // Send MatchEnded to opponent (if still connected)
    vec![OutgoingMessage {
        player_id: opponent_id,
        message: ServerMessage::MatchEnded {
            reason: MatchEndReason::Disconnection,
        },
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use battld_common::GameState;
    use sqlx::SqlitePool;

    // Helper function to create a test database
    async fn create_test_db() -> Database {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let db = Database::from_pool(pool);
        db.initialize().await.unwrap();
        db
    }

    // Helper to create a test player
    async fn create_test_player(db: &Database, name: &str) -> i64 {
        db.create_player(&format!("{name}_hint"), &format!("{name}_key"), name).await.unwrap()
    }

    #[tokio::test]
    async fn test_make_move_not_authenticated() {
        let db = create_test_db().await;

        // Try to make a move when player has no active match
        let messages = handle_make_move_logic(999, 0, 0, &db).await;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].player_id, 999);
        match &messages[0].message {
            ServerMessage::Error { message } => {
                assert_eq!(message, "No active match found");
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[tokio::test]
    async fn test_make_move_not_your_turn() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match where player 1 goes first
        let game_state = GameState::new();
        let _match_id = db.create_match(p1, p2, 1, &game_state.to_json()).await.unwrap();

        // Try to make a move as player 2 (not their turn)
        let messages = handle_make_move_logic(p2, 0, 0, &db).await;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].player_id, p2);
        match &messages[0].message {
            ServerMessage::Error { message } => {
                assert_eq!(message, "Not your turn");
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[tokio::test]
    async fn test_make_move_valid() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match where player 1 goes first
        let game_state = GameState::new();
        let match_id = db.create_match(p1, p2, 1, &game_state.to_json()).await.unwrap();

        // Make a valid move as player 1
        let messages = handle_make_move_logic(p1, 0, 0, &db).await;

        // Should send GameStateUpdate to both players
        assert_eq!(messages.len(), 2);

        // Check both players get the update
        let player_ids: Vec<i64> = messages.iter().map(|m| m.player_id).collect();
        assert!(player_ids.contains(&p1));
        assert!(player_ids.contains(&p2));

        // All should be GameStateUpdate messages
        for msg in &messages {
            match &msg.message {
                ServerMessage::GameStateUpdate { match_data } => {
                    assert_eq!(match_data.id, match_id);
                    assert_eq!(match_data.current_player, 2); // Turn should switch to player 2
                    assert!(match_data.in_progress);
                }
                _ => panic!("Expected GameStateUpdate message"),
            }
        }
    }

    #[tokio::test]
    async fn test_make_move_winning() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a game state where player 1 is about to win
        let mut game_state = GameState::new();
        // Player 1 has top row almost complete: X X _
        game_state.place_move(0, 1).unwrap(); // [0,0]
        game_state.place_move(3, 2).unwrap(); // [1,0]
        game_state.place_move(1, 1).unwrap(); // [0,1]
        game_state.place_move(4, 2).unwrap(); // [1,1]
        // Now player 1 can win by playing [0,2]

        let match_id = db.create_match(p1, p2, 1, &game_state.to_json()).await.unwrap();

        // Make the winning move as player 1
        let messages = handle_make_move_logic(p1, 0, 2, &db).await;

        // Should send GameStateUpdate and MatchEnded to both players
        assert_eq!(messages.len(), 4); // 2 GameStateUpdate + 2 MatchEnded

        // Verify we get the right message types
        let mut state_updates = 0;
        let mut match_ended = 0;

        for msg in &messages {
            match &msg.message {
                ServerMessage::GameStateUpdate { match_data } => {
                    assert_eq!(match_data.id, match_id);
                    assert!(!match_data.in_progress);
                    assert_eq!(match_data.outcome, Some(MatchOutcome::Player1Win));
                    state_updates += 1;
                }
                ServerMessage::MatchEnded { .. } => {
                    match_ended += 1;
                }
                _ => panic!("Unexpected message type"),
            }
        }

        assert_eq!(state_updates, 2);
        assert_eq!(match_ended, 2);
    }

    #[tokio::test]
    async fn test_disconnect_from_active_match() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create an active match
        let game_state = GameState::new();
        let match_id = db.create_match(p1, p2, 1, &game_state.to_json()).await.unwrap();

        // Player 1 disconnects
        let (messages, match_id_opt) = handle_disconnect_logic(p1, &db).await;

        // Should return opponent's ID and the match ID
        assert_eq!(match_id_opt, Some(match_id));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].player_id, p2);

        match &messages[0].message {
            ServerMessage::PlayerDisconnected { player_id } => {
                assert_eq!(*player_id, p1);
            }
            _ => panic!("Expected PlayerDisconnected message"),
        }
    }

    #[tokio::test]
    async fn test_disconnect_timeout() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create an active match
        let game_state = GameState::new();
        let match_id = db.create_match(p1, p2, 1, &game_state.to_json()).await.unwrap();

        // Timeout occurs
        let messages = handle_disconnect_timeout_logic(p1, match_id, &db).await;

        // Should send MatchEnded to opponent
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].player_id, p2);

        match &messages[0].message {
            ServerMessage::MatchEnded { .. } => {}
            _ => panic!("Expected MatchEnded message"),
        }

        // Match should be marked as draw (lowercase in DB)
        let match_record = db.get_match_by_id(match_id).await.unwrap();
        assert_eq!(match_record.in_progress, 0);
        assert_eq!(match_record.outcome.as_deref(), Some("draw"));
    }

    #[tokio::test]
    async fn test_join_matchmaking_creates_waiting_match() {
        let db = create_test_db().await;

        // Create a player
        let p1 = create_test_player(&db, "player1").await;

        // Join matchmaking
        let messages = handle_join_matchmaking_logic(p1, &db).await;

        // Should send WaitingForOpponent
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].player_id, p1);

        match &messages[0].message {
            ServerMessage::WaitingForOpponent => {}
            _ => panic!("Expected WaitingForOpponent message"),
        }
    }

    #[tokio::test]
    async fn test_join_matchmaking_finds_opponent() {
        let db = create_test_db().await;

        // Create two players
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Player 1 joins matchmaking (creates waiting match)
        let _ = handle_join_matchmaking_logic(p1, &db).await;

        // Player 2 joins matchmaking (should match with player 1)
        let messages = handle_join_matchmaking_logic(p2, &db).await;

        // Should send MatchFound to both players
        assert_eq!(messages.len(), 2);

        let player_ids: Vec<i64> = messages.iter().map(|m| m.player_id).collect();
        assert!(player_ids.contains(&p1));
        assert!(player_ids.contains(&p2));

        for msg in &messages {
            match &msg.message {
                ServerMessage::MatchFound { match_data } => {
                    assert_eq!(match_data.player1_id, p1);
                    assert_eq!(match_data.player2_id, p2);
                    assert!(match_data.in_progress);
                }
                _ => panic!("Expected MatchFound message"),
            }
        }
    }
}
