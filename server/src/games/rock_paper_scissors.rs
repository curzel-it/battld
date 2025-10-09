use battld_common::games::{players::PlayerSymbol, rock_paper_scissors::{RockPaperScissorsGameState, RockPaperScissorsMove}};

use super::GameError;

/// Stateless RockPaperScissors game engine
pub struct RockPaperScissorsEngine;

impl RockPaperScissorsEngine {
    /// Update the game state with a player's move
    pub fn update(
        &self,
        state: &RockPaperScissorsGameState,
        player: PlayerSymbol,
        move_choice: RockPaperScissorsMove,
    ) -> Result<RockPaperScissorsGameState, GameError> {
        // Check game is not already finished
        if state.is_finished() {
            return Err(GameError::GameNotInProgress);
        }

        // Get current round (last in the list)
        let current_round_idx = state.rounds.len() - 1;
        let current_round = &state.rounds[current_round_idx];

        // Check if player has already submitted a move for this round
        let player_already_moved = match player {
            1 => current_round.0.is_some(),
            2 => current_round.1.is_some(),
            _ => return Err(GameError::InvalidPlayer),
        };

        if player_already_moved {
            return Err(GameError::IllegalMove(
                "You have already submitted a move for this round".to_string(),
            ));
        }

        // Create new state with the move
        let mut new_state = state.clone();
        let new_round = match player {
            1 => (Some(move_choice), current_round.1),
            2 => (current_round.0, Some(move_choice)),
            _ => return Err(GameError::InvalidPlayer),
        };

        new_state.rounds[current_round_idx] = new_round;

        // If both players have now submitted moves, check if we need a new round
        if let (Some(_), Some(_)) = new_round {
            // Both moves are in - round is complete
            // Check if game is finished
            if !new_state.is_finished() {
                // Game continues - add a new round
                new_state.rounds.push((None, None));
            }
        }

        Ok(new_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rock_paper_scissors_move_beats() {
        // Rock beats scissors
        assert_eq!(RockPaperScissorsMove::Rock.beats(&RockPaperScissorsMove::Scissors), Some(RockPaperScissorsMove::Rock));
        assert_eq!(RockPaperScissorsMove::Scissors.beats(&RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Rock));

        // Paper beats rock
        assert_eq!(RockPaperScissorsMove::Paper.beats(&RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Paper));
        assert_eq!(RockPaperScissorsMove::Rock.beats(&RockPaperScissorsMove::Paper), Some(RockPaperScissorsMove::Paper));

        // Scissors beats paper
        assert_eq!(RockPaperScissorsMove::Scissors.beats(&RockPaperScissorsMove::Paper), Some(RockPaperScissorsMove::Scissors));
        assert_eq!(RockPaperScissorsMove::Paper.beats(&RockPaperScissorsMove::Scissors), Some(RockPaperScissorsMove::Scissors));

        // Draws
        assert_eq!(RockPaperScissorsMove::Rock.beats(&RockPaperScissorsMove::Rock), None);
        assert_eq!(RockPaperScissorsMove::Paper.beats(&RockPaperScissorsMove::Paper), None);
        assert_eq!(RockPaperScissorsMove::Scissors.beats(&RockPaperScissorsMove::Scissors), None);
    }

    #[test]
    fn test_new_game_state() {
        let state = RockPaperScissorsGameState::new();
        assert_eq!(state.rounds.len(), 1);
        assert_eq!(state.rounds[0], (None, None));
        assert_eq!(state.current_round(), 1);
        assert_eq!(state.get_score(), (0, 0));
        assert!(!state.is_finished());
        assert_eq!(state.get_winner(), None);
    }

    #[test]
    fn test_valid_move_player1() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        let new_state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();

        assert_eq!(new_state.rounds[0].0, Some(RockPaperScissorsMove::Rock));
        assert_eq!(new_state.rounds[0].1, None);
        assert!(!new_state.is_finished());
    }

    #[test]
    fn test_valid_move_player2() {
        let mut state = RockPaperScissorsGameState::new();
        state.rounds[0].0 = Some(RockPaperScissorsMove::Rock); // Player 1 already moved

        let engine = RockPaperScissorsEngine;
        let new_state = engine.update(&state, 2, RockPaperScissorsMove::Scissors).unwrap();

        assert_eq!(new_state.rounds[0].0, Some(RockPaperScissorsMove::Rock));
        assert_eq!(new_state.rounds[0].1, Some(RockPaperScissorsMove::Scissors));
        // Round complete, player 1 won, new round should be added
        assert_eq!(new_state.rounds.len(), 2);
        assert_eq!(new_state.rounds[1], (None, None));
        assert!(!new_state.is_finished());
    }

    #[test]
    fn test_duplicate_move_rejected() {
        let mut state = RockPaperScissorsGameState::new();
        state.rounds[0].0 = Some(RockPaperScissorsMove::Rock); // Player 1 already moved

        let engine = RockPaperScissorsEngine;
        let result = engine.update(&state, 1, RockPaperScissorsMove::Paper);

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }

    #[test]
    fn test_round_completion_creates_new_round() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        // Player 1 moves
        let state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        assert_eq!(state.rounds.len(), 1);

        // Player 2 moves - round completes
        let state = engine.update(&state, 2, RockPaperScissorsMove::Scissors).unwrap();
        assert_eq!(state.rounds.len(), 2); // New round added
        assert_eq!(state.rounds[1], (None, None));
    }

    #[test]
    fn test_draw_round() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        // Both players choose rock - draw
        let state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Rock).unwrap();

        assert_eq!(state.get_score(), (0, 0)); // No one gets a point
        assert_eq!(state.rounds.len(), 2); // New round added
        assert!(!state.is_finished());
    }

    #[test]
    fn test_player1_wins_2_0() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        // Round 1: Player 1 wins (rock beats scissors)
        let state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));
        assert!(!state.is_finished());

        // Round 2: Player 1 wins (paper beats rock)
        let state = engine.update(&state, 1, RockPaperScissorsMove::Paper).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Rock).unwrap();
        assert_eq!(state.get_score(), (2, 0));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(1));

        // No new round should be added when game is finished
        assert_eq!(state.rounds.len(), 2);
    }

    #[test]
    fn test_player2_wins_2_1() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        // Round 1: Player 1 wins
        let state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));

        // Round 2: Player 2 wins
        let state = engine.update(&state, 1, RockPaperScissorsMove::Scissors).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Rock).unwrap();
        assert_eq!(state.get_score(), (1, 1));

        // Round 3: Player 2 wins (gets 2 total)
        let state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        let state = engine.update(&state, 2, RockPaperScissorsMove::Paper).unwrap();
        assert_eq!(state.get_score(), (1, 2));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(2));
    }

    #[test]
    fn test_game_already_finished() {
        let mut state = RockPaperScissorsGameState::new();
        // Manually create a finished game (2-0)
        state.rounds = vec![
            (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Scissors)),
            (Some(RockPaperScissorsMove::Paper), Some(RockPaperScissorsMove::Rock)),
        ];

        assert!(state.is_finished());

        let engine = RockPaperScissorsEngine;
        let result = engine.update(&state, 1, RockPaperScissorsMove::Rock);

        assert!(matches!(result, Err(GameError::GameNotInProgress)));
    }

    #[test]
    fn test_invalid_player() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        let result = engine.update(&state, 3, RockPaperScissorsMove::Rock);
        assert!(matches!(result, Err(GameError::InvalidPlayer)));

        let result = engine.update(&state, 0, RockPaperScissorsMove::Rock);
        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }

    #[test]
    fn test_state_immutability() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        let original_rounds = state.rounds.clone();

        // Make a move
        let _new_state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();

        // Original state should be unchanged
        assert_eq!(state.rounds, original_rounds);
        assert_eq!(state.rounds[0], (None, None));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut state = RockPaperScissorsGameState::new();
        state.rounds = vec![
            (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Scissors)),
            (Some(RockPaperScissorsMove::Paper), None),
        ];

        // Serialize to JSON
        let json = serde_json::to_value(&state).unwrap();

        // Deserialize back
        let deserialized: RockPaperScissorsGameState = serde_json::from_value(json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_many_draw_rounds() {
        let state = RockPaperScissorsGameState::new();
        let engine = RockPaperScissorsEngine;

        // Create 5 draw rounds
        let mut state = state;
        for _ in 0..5 {
            state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
            state = engine.update(&state, 2, RockPaperScissorsMove::Rock).unwrap();
        }

        assert_eq!(state.get_score(), (0, 0));
        assert_eq!(state.rounds.len(), 6); // 5 completed + 1 new
        assert!(!state.is_finished());

        // Now player 1 wins 2 rounds
        state = engine.update(&state, 1, RockPaperScissorsMove::Rock).unwrap();
        state = engine.update(&state, 2, RockPaperScissorsMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));

        state = engine.update(&state, 1, RockPaperScissorsMove::Paper).unwrap();
        state = engine.update(&state, 2, RockPaperScissorsMove::Rock).unwrap();
        assert_eq!(state.get_score(), (2, 0));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(1));
    }

    #[test]
    fn test_redact_for_player1() {
        let mut state = RockPaperScissorsGameState::new();

        // Round 1: Both players moved (completed round)
        state.rounds[0] = (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Paper));

        // Round 2: Player 1 moved, player 2 hasn't
        state.rounds.push((Some(RockPaperScissorsMove::Scissors), None));

        // Round 3: Player 2 moved, player 1 hasn't
        state.rounds.push((None, Some(RockPaperScissorsMove::Rock)));

        // Round 4: No one moved yet
        state.rounds.push((None, None));

        let redacted = state.redact_for_player(1);

        // Round 1: Both moved, so both should be visible
        assert_eq!(redacted.rounds[0], (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Paper)));

        // Round 2: Player 1 moved but player 2 hasn't, p1 sees their move
        assert_eq!(redacted.rounds[1], (Some(RockPaperScissorsMove::Scissors), None));

        // Round 3: Player 2 moved but player 1 hasn't, p2's move should be redacted
        assert_eq!(redacted.rounds[2], (None, Some(RockPaperScissorsMove::Redacted)));

        // Round 4: No one moved, both None
        assert_eq!(redacted.rounds[3], (None, None));
    }

    #[test]
    fn test_redact_for_player2() {
        let mut state = RockPaperScissorsGameState::new();

        // Round 1: Both players moved (completed round)
        state.rounds[0] = (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Paper));

        // Round 2: Player 1 moved, player 2 hasn't
        state.rounds.push((Some(RockPaperScissorsMove::Scissors), None));

        // Round 3: Player 2 moved, player 1 hasn't
        state.rounds.push((None, Some(RockPaperScissorsMove::Rock)));

        // Round 4: No one moved yet
        state.rounds.push((None, None));

        let redacted = state.redact_for_player(2);

        // Round 1: Both moved, so both should be visible
        assert_eq!(redacted.rounds[0], (Some(RockPaperScissorsMove::Rock), Some(RockPaperScissorsMove::Paper)));

        // Round 2: Player 1 moved but player 2 hasn't, p1's move should be redacted
        assert_eq!(redacted.rounds[1], (Some(RockPaperScissorsMove::Redacted), None));

        // Round 3: Player 2 moved but player 1 hasn't, p2 sees their move
        assert_eq!(redacted.rounds[2], (None, Some(RockPaperScissorsMove::Rock)));

        // Round 4: No one moved, both None
        assert_eq!(redacted.rounds[3], (None, None));
    }
}
