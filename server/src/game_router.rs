use crate::games::{tictactoe::*, GameError};
use battld_common::{GameType, Match, MatchOutcome};
use serde_json::Value as JsonValue;

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
        GameType::TicTacToe => handle_tictactoe_move(game_match, player_id, move_data),
        GameType::RockPaperScissors => {
            // Not yet implemented
            Err(GameError::IllegalMove(
                "Rock-Paper-Scissors not yet implemented".to_string(),
            ))
        }
    }
}

fn handle_tictactoe_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    // Deserialize the current game state from JSON
    let current_state: TicTacToeGameState = serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| GameError::IllegalMove(format!("Invalid game state: {}", e)))?;

    // Deserialize the move data
    let tictactoe_move: TicTacToeMove = serde_json::from_value(move_data)
        .map_err(|e| GameError::IllegalMove(format!("Invalid move data: {}", e)))?;

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
    let new_state = engine.update(&current_state, player_symbol, &tictactoe_move)?;

    // Serialize the new state back to JSON
    let new_state_json = serde_json::to_value(&new_state)
        .map_err(|e| GameError::IllegalMove(format!("Failed to serialize state: {}", e)))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use battld_common::GameType;

    #[test]
    fn test_tictactoe_valid_move() {
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
            current_player: 1,
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
    fn test_tictactoe_invalid_player() {
        let initial_state = TicTacToeGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::TicTacToe,
            current_player: 1,
            game_state: state_json,
        };

        // Invalid player ID tries to make a move
        let move_data = serde_json::json!({ "row": 0, "col": 0 });
        let result = handle_game_move(&game_match, 999, move_data);

        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }

    #[test]
    fn test_tictactoe_wrong_turn() {
        let initial_state = TicTacToeGameState::new();
        let state_json = serde_json::to_value(&initial_state).unwrap();

        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::TicTacToe,
            current_player: 1,
            game_state: state_json,
        };

        // Player 2 tries to move when it's Player 1's turn
        let move_data = serde_json::json!({ "row": 0, "col": 0 });
        let result = handle_game_move(&game_match, 200, move_data);

        assert!(matches!(result, Err(GameError::WrongTurn)));
    }

    #[test]
    fn test_rps_not_implemented() {
        let game_match = Match {
            id: 1,
            player1_id: 100,
            player2_id: 200,
            in_progress: true,
            outcome: None,
            game_type: GameType::RockPaperScissors,
            current_player: 1,
            game_state: serde_json::json!({}),
        };

        let move_data = serde_json::json!({ "choice": "rock" });
        let result = handle_game_move(&game_match, 100, move_data);

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }
}
