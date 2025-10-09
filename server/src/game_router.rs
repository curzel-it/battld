use crate::games::{tic_tac_toe::*, rock_paper_scissors::*, GameError};
use battld_common::{GameType, Match, MatchOutcome, RPSGameState, RPSMove};
use serde_json::Value as JsonValue;
use rand::Rng;

/// Result of processing a game move
pub struct GameMoveResult {
    pub new_state: JsonValue,
    pub is_finished: bool,
    pub outcome: Option<MatchOutcome>,
}

/// Routes game moves to the appropriate game engine based on game type
pub fn handle_game_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    match game_match.game_type {
        GameType::TicTacToe => handle_tic_tac_toe_move(game_match, player_id, move_data),
        GameType::RockPaperScissors => handle_rps_move(game_match, player_id, move_data),
    }
}

/// Redact match data for a specific player based on game type
pub fn redact_match_for_player(match_data: &Match, player_id: i64) -> Match {
    // Determine which player number this is (1 or 2)
    let player_num = if player_id == match_data.player1_id {
        1
    } else if player_id == match_data.player2_id {
        2
    } else {
        return match_data.clone(); // Not a player in this match
    };

    // Route to appropriate game redaction logic
    let redacted_state = match match_data.game_type {
        GameType::TicTacToe => {
            // Deserialize, redact, and serialize TicTacToe state
            match serde_json::from_value::<TicTacToeGameState>(match_data.game_state.clone()) {
                Ok(state) => {
                    let redacted = state.redact_for_player(player_num);
                    serde_json::to_value(&redacted).unwrap_or(match_data.game_state.clone())
                }
                Err(_) => match_data.game_state.clone(),
            }
        }
        GameType::RockPaperScissors => {
            // Deserialize, redact, and serialize RPS state
            match serde_json::from_value::<RPSGameState>(match_data.game_state.clone()) {
                Ok(state) => {
                    let redacted = state.redact_for_player(player_num);
                    serde_json::to_value(&redacted).unwrap_or(match_data.game_state.clone())
                }
                Err(_) => match_data.game_state.clone(),
            }
        }
    };

    // Create a new Match with redacted game state
    Match {
        id: match_data.id,
        player1_id: match_data.player1_id,
        player2_id: match_data.player2_id,
        in_progress: match_data.in_progress,
        outcome: match_data.outcome.clone(),
        game_type: match_data.game_type.clone(),
        game_state: redacted_state,
    }
}

/// Initialize a new game state for a given game type
/// Returns the serialized game state as a JSON string
pub fn initialize_game_state(game_type: &GameType) -> String {
    // Randomize who goes first
    let first_player = {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.5) { 1 } else { 2 }
    };

    match game_type {
        GameType::TicTacToe => {
            let mut state = TicTacToeGameState::new();
            state.current_player = first_player;
            serde_json::to_string(&state).unwrap()
        }
        GameType::RockPaperScissors => {
            let state = RPSGameState::new();
            serde_json::to_string(&state).unwrap()
        }
    }
}

fn handle_tic_tac_toe_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    // Deserialize the current game state from JSON
    let current_state: TicTacToeGameState = serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| GameError::IllegalMove(format!("Invalid game state: {e}")))?;

    // Deserialize the move data
    let tic_tac_toe_move: TicTacToeMove = serde_json::from_value(move_data)
        .map_err(|e| GameError::IllegalMove(format!("Invalid move data: {e}")))?;

    // Determine which player symbol this player is
    let player_symbol = if player_id == game_match.player1_id {
        1
    } else if player_id == game_match.player2_id {
        2
    } else {
        return Err(GameError::InvalidPlayer);
    };

    // Call the TicTacToe engine to process the move
    let engine = TicTacToeEngine;
    let new_state = engine.update(&current_state, player_symbol, &tic_tac_toe_move)?;

    // Serialize the new state back to JSON
    let new_state_json = serde_json::to_value(&new_state)
        .map_err(|e| GameError::IllegalMove(format!("Failed to serialize state: {e}")))?;

    // Determine outcome if game is finished
    let outcome = if new_state.is_finished {
        match new_state.winner {
            Some(1) => Some(MatchOutcome::Player1Win),
            Some(2) => Some(MatchOutcome::Player2Win),
            _ => Some(MatchOutcome::Draw),
        }
    } else {
        None
    };

    Ok(GameMoveResult {
        new_state: new_state_json,
        is_finished: new_state.is_finished,
        outcome,
    })
}

fn handle_rps_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    // Deserialize the current game state from JSON
    let current_state: RPSGameState = serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| GameError::IllegalMove(format!("Invalid game state: {e}")))?;

    // Deserialize the move data - expects {"choice": "rock"|"paper"|"scissors"}
    #[derive(serde::Deserialize)]
    struct RPSMoveData {
        choice: RPSMove,
    }

    let move_data: RPSMoveData = serde_json::from_value(move_data)
        .map_err(|e| GameError::IllegalMove(format!("Invalid move data: {e}")))?;

    // Determine which player symbol this player is
    let player_symbol = if player_id == game_match.player1_id {
        1
    } else if player_id == game_match.player2_id {
        2
    } else {
        return Err(GameError::InvalidPlayer);
    };

    // Call the RPS engine to process the move
    let engine = RPSEngine;
    let new_state = engine.update(&current_state, player_symbol, move_data.choice)?;

    // Serialize the new state back to JSON
    let new_state_json = serde_json::to_value(&new_state)
        .map_err(|e| GameError::IllegalMove(format!("Failed to serialize state: {e}")))?;

    // Determine outcome if game is finished
    let outcome = if new_state.is_finished() {
        match new_state.get_winner() {
            Some(1) => Some(MatchOutcome::Player1Win),
            Some(2) => Some(MatchOutcome::Player2Win),
            _ => Some(MatchOutcome::Draw), // Should not happen with "first to 2 wins" logic
        }
    } else {
        None
    };

    Ok(GameMoveResult {
        new_state: new_state_json,
        is_finished: new_state.is_finished(),
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use battld_common::GameType;

    #[test]
    fn test_tic_tac_toe_valid_move() {
        // Create initial TicTacToe state
        let initial_state = TicTacToeGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::TicTacToe,
            game_state: state_json,
        };

        // Player 1 makes a move
        let move_data = serde_json::json!({ "row": 0, "col": 0 });
        let result = handle_game_move(&game_match, 100, move_data).unwrap();

        assert!(!result.is_finished);
        assert!(result.outcome.is_none());

        // Verify the state was updated
        let new_state: TicTacToeGameState = serde_json::from_value(result.new_state).unwrap();
        assert_eq!(new_state.board[0], 1);
        assert_eq!(new_state.current_player, 2);
    }

    #[test]
    fn test_tic_tac_toe_invalid_player() {
        let initial_state = TicTacToeGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::TicTacToe,
            game_state: state_json,
        };

        // Invalid player ID tries to make a move
        let move_data = serde_json::json!({ "row": 0, "col": 0 });
        let result = handle_game_move(&game_match, 999, move_data);

        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }

    #[test]
    fn test_tic_tac_toe_wrong_turn() {
        let initial_state = TicTacToeGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::TicTacToe,
            game_state: state_json,
        };

        // Player 2 tries to move when it's Player 1's turn
        let move_data = serde_json::json!({ "row": 0, "col": 0 });
        let result = handle_game_move(&game_match, 200, move_data);

        assert!(matches!(result, Err(GameError::WrongTurn)));
    }

    #[test]
    fn test_rps_valid_move() {
        // Create initial RPS state
        let initial_state = RPSGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::RockPaperScissors,
            game_state: state_json,
        };

        // Player 1 makes a move
        let move_data = serde_json::json!({ "choice": "rock" });
        let result = handle_game_move(&game_match, 100, move_data).unwrap();

        assert!(!result.is_finished);
        assert!(result.outcome.is_none());

        // Verify the state was updated (player 1's move should be recorded)
        let new_state: RPSGameState = serde_json::from_value(result.new_state).unwrap();
        assert_eq!(new_state.rounds[0].0, Some(RPSMove::Rock));
        assert_eq!(new_state.rounds[0].1, None); // Player 2 hasn't moved yet
    }
}
