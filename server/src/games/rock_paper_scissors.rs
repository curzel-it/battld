use super::GameError;
use serde::{Deserialize, Serialize};

/// Type alias for player symbols (1 or 2)
pub type PlayerSymbol = i32;

/// Represents a move in Rock-Paper-Scissors
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RPSMove {
    Rock,
    Paper,
    Scissors,
    Redacted,
}

impl RPSMove {
    /// Determine winner: returns Some(winning_move) or None for draw
    pub fn beats(&self, other: &RPSMove) -> Option<RPSMove> {
        match (self, other) {
            (RPSMove::Rock, RPSMove::Scissors) => Some(*self),
            (RPSMove::Paper, RPSMove::Rock) => Some(*self),
            (RPSMove::Scissors, RPSMove::Paper) => Some(*self),
            (RPSMove::Scissors, RPSMove::Rock) => Some(*other),
            (RPSMove::Rock, RPSMove::Paper) => Some(*other),
            (RPSMove::Paper, RPSMove::Scissors) => Some(*other),
            _ => None, // Draw
        }
    }
}

/// Represents the complete state of a Rock-Paper-Scissors game
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RPSGameState {
    /// List of rounds: each round is (player1_move, player2_move)
    /// None means the player hasn't submitted their move yet
    pub rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>,
}

impl RPSGameState {
    /// Create a new RPS game with initial round
    pub fn new() -> Self {
        Self {
            rounds: vec![(None, None)],
        }
    }

    /// Get current round number (1-indexed for display)
    #[allow(dead_code)]
    pub fn current_round(&self) -> usize {
        self.rounds.len()
    }

    /// Get score as (player1_wins, player2_wins)
    pub fn get_score(&self) -> (u8, u8) {
        let mut p1_wins = 0;
        let mut p2_wins = 0;

        for round in &self.rounds {
            if let (Some(p1_move), Some(p2_move)) = round {
                if let Some(winner) = p1_move.beats(p2_move) {
                    if winner == *p1_move {
                        p1_wins += 1;
                    } else {
                        p2_wins += 1;
                    }
                }
                // If None (draw), neither player gets a point
            }
        }

        (p1_wins, p2_wins)
    }

    /// Check if the game is finished (either player has 2 wins)
    pub fn is_finished(&self) -> bool {
        let (p1_wins, p2_wins) = self.get_score();
        p1_wins >= 2 || p2_wins >= 2
    }

    /// Get the winner (if game is finished)
    pub fn get_winner(&self) -> Option<PlayerSymbol> {
        if !self.is_finished() {
            return None;
        }

        let (p1_wins, p2_wins) = self.get_score();
        if p1_wins >= 2 {
            Some(1)
        } else if p2_wins >= 2 {
            Some(2)
        } else {
            None
        }
    }

    /// Redact opponent's moves for a specific player
    /// Player 1 sees their own moves but player 2's moves are redacted (and vice versa)
    pub fn redact_for_player(&self, player: PlayerSymbol) -> Self {
        let redacted_rounds = self.rounds.iter().map(|(p1_move, p2_move)| {
            match player {
                1 => {
                    // Player 1 sees their own moves, but player 2's moves are redacted if incomplete
                    let redacted_p2 = if p1_move.is_some() && p2_move.is_some() {
                        *p2_move // Both moves in, show actual move
                    } else if p2_move.is_some() {
                        Some(RPSMove::Redacted) // Only p2 moved, hide it
                    } else {
                        None // p2 hasn't moved yet
                    };
                    (*p1_move, redacted_p2)
                }
                2 => {
                    // Player 2 sees their own moves, but player 1's moves are redacted if incomplete
                    let redacted_p1 = if p1_move.is_some() && p2_move.is_some() {
                        *p1_move // Both moves in, show actual move
                    } else if p1_move.is_some() {
                        Some(RPSMove::Redacted) // Only p1 moved, hide it
                    } else {
                        None // p1 hasn't moved yet
                    };
                    (redacted_p1, *p2_move)
                }
                _ => (*p1_move, *p2_move), // Invalid player, return as-is
            }
        }).collect();

        Self {
            rounds: redacted_rounds,
        }
    }

    /// Compute the winner of a specific round
    #[allow(dead_code)]
    pub fn compute_round_winner(p1_move: RPSMove, p2_move: RPSMove) -> Option<PlayerSymbol> {
        match p1_move.beats(&p2_move) {
            Some(winner) => {
                if winner == p1_move {
                    Some(1)
                } else {
                    Some(2)
                }
            }
            None => None, // Draw
        }
    }
}

/// Stateless RPS game engine
pub struct RPSEngine;

impl RPSEngine {
    /// Update the game state with a player's move
    pub fn update(
        &self,
        state: &RPSGameState,
        player: PlayerSymbol,
        move_choice: RPSMove,
    ) -> Result<RPSGameState, GameError> {
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
    fn test_rps_move_beats() {
        // Rock beats scissors
        assert_eq!(RPSMove::Rock.beats(&RPSMove::Scissors), Some(RPSMove::Rock));
        assert_eq!(RPSMove::Scissors.beats(&RPSMove::Rock), Some(RPSMove::Rock));

        // Paper beats rock
        assert_eq!(RPSMove::Paper.beats(&RPSMove::Rock), Some(RPSMove::Paper));
        assert_eq!(RPSMove::Rock.beats(&RPSMove::Paper), Some(RPSMove::Paper));

        // Scissors beats paper
        assert_eq!(RPSMove::Scissors.beats(&RPSMove::Paper), Some(RPSMove::Scissors));
        assert_eq!(RPSMove::Paper.beats(&RPSMove::Scissors), Some(RPSMove::Scissors));

        // Draws
        assert_eq!(RPSMove::Rock.beats(&RPSMove::Rock), None);
        assert_eq!(RPSMove::Paper.beats(&RPSMove::Paper), None);
        assert_eq!(RPSMove::Scissors.beats(&RPSMove::Scissors), None);
    }

    #[test]
    fn test_new_game_state() {
        let state = RPSGameState::new();
        assert_eq!(state.rounds.len(), 1);
        assert_eq!(state.rounds[0], (None, None));
        assert_eq!(state.current_round(), 1);
        assert_eq!(state.get_score(), (0, 0));
        assert!(!state.is_finished());
        assert_eq!(state.get_winner(), None);
    }

    #[test]
    fn test_valid_move_player1() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        let new_state = engine.update(&state, 1, RPSMove::Rock).unwrap();

        assert_eq!(new_state.rounds[0].0, Some(RPSMove::Rock));
        assert_eq!(new_state.rounds[0].1, None);
        assert!(!new_state.is_finished());
    }

    #[test]
    fn test_valid_move_player2() {
        let mut state = RPSGameState::new();
        state.rounds[0].0 = Some(RPSMove::Rock); // Player 1 already moved

        let engine = RPSEngine;
        let new_state = engine.update(&state, 2, RPSMove::Scissors).unwrap();

        assert_eq!(new_state.rounds[0].0, Some(RPSMove::Rock));
        assert_eq!(new_state.rounds[0].1, Some(RPSMove::Scissors));
        // Round complete, player 1 won, new round should be added
        assert_eq!(new_state.rounds.len(), 2);
        assert_eq!(new_state.rounds[1], (None, None));
        assert!(!new_state.is_finished());
    }

    #[test]
    fn test_duplicate_move_rejected() {
        let mut state = RPSGameState::new();
        state.rounds[0].0 = Some(RPSMove::Rock); // Player 1 already moved

        let engine = RPSEngine;
        let result = engine.update(&state, 1, RPSMove::Paper);

        assert!(matches!(result, Err(GameError::IllegalMove(_))));
    }

    #[test]
    fn test_round_completion_creates_new_round() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        // Player 1 moves
        let state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        assert_eq!(state.rounds.len(), 1);

        // Player 2 moves - round completes
        let state = engine.update(&state, 2, RPSMove::Scissors).unwrap();
        assert_eq!(state.rounds.len(), 2); // New round added
        assert_eq!(state.rounds[1], (None, None));
    }

    #[test]
    fn test_draw_round() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        // Both players choose rock - draw
        let state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        let state = engine.update(&state, 2, RPSMove::Rock).unwrap();

        assert_eq!(state.get_score(), (0, 0)); // No one gets a point
        assert_eq!(state.rounds.len(), 2); // New round added
        assert!(!state.is_finished());
    }

    #[test]
    fn test_player1_wins_2_0() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        // Round 1: Player 1 wins (rock beats scissors)
        let state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        let state = engine.update(&state, 2, RPSMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));
        assert!(!state.is_finished());

        // Round 2: Player 1 wins (paper beats rock)
        let state = engine.update(&state, 1, RPSMove::Paper).unwrap();
        let state = engine.update(&state, 2, RPSMove::Rock).unwrap();
        assert_eq!(state.get_score(), (2, 0));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(1));

        // No new round should be added when game is finished
        assert_eq!(state.rounds.len(), 2);
    }

    #[test]
    fn test_player2_wins_2_1() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        // Round 1: Player 1 wins
        let state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        let state = engine.update(&state, 2, RPSMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));

        // Round 2: Player 2 wins
        let state = engine.update(&state, 1, RPSMove::Scissors).unwrap();
        let state = engine.update(&state, 2, RPSMove::Rock).unwrap();
        assert_eq!(state.get_score(), (1, 1));

        // Round 3: Player 2 wins (gets 2 total)
        let state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        let state = engine.update(&state, 2, RPSMove::Paper).unwrap();
        assert_eq!(state.get_score(), (1, 2));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(2));
    }

    #[test]
    fn test_game_already_finished() {
        let mut state = RPSGameState::new();
        // Manually create a finished game (2-0)
        state.rounds = vec![
            (Some(RPSMove::Rock), Some(RPSMove::Scissors)),
            (Some(RPSMove::Paper), Some(RPSMove::Rock)),
        ];

        assert!(state.is_finished());

        let engine = RPSEngine;
        let result = engine.update(&state, 1, RPSMove::Rock);

        assert!(matches!(result, Err(GameError::GameNotInProgress)));
    }

    #[test]
    fn test_invalid_player() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        let result = engine.update(&state, 3, RPSMove::Rock);
        assert!(matches!(result, Err(GameError::InvalidPlayer)));

        let result = engine.update(&state, 0, RPSMove::Rock);
        assert!(matches!(result, Err(GameError::InvalidPlayer)));
    }

    #[test]
    fn test_state_immutability() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        let original_rounds = state.rounds.clone();

        // Make a move
        let _new_state = engine.update(&state, 1, RPSMove::Rock).unwrap();

        // Original state should be unchanged
        assert_eq!(state.rounds, original_rounds);
        assert_eq!(state.rounds[0], (None, None));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut state = RPSGameState::new();
        state.rounds = vec![
            (Some(RPSMove::Rock), Some(RPSMove::Scissors)),
            (Some(RPSMove::Paper), None),
        ];

        // Serialize to JSON
        let json = serde_json::to_value(&state).unwrap();

        // Deserialize back
        let deserialized: RPSGameState = serde_json::from_value(json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_many_draw_rounds() {
        let state = RPSGameState::new();
        let engine = RPSEngine;

        // Create 5 draw rounds
        let mut state = state;
        for _ in 0..5 {
            state = engine.update(&state, 1, RPSMove::Rock).unwrap();
            state = engine.update(&state, 2, RPSMove::Rock).unwrap();
        }

        assert_eq!(state.get_score(), (0, 0));
        assert_eq!(state.rounds.len(), 6); // 5 completed + 1 new
        assert!(!state.is_finished());

        // Now player 1 wins 2 rounds
        state = engine.update(&state, 1, RPSMove::Rock).unwrap();
        state = engine.update(&state, 2, RPSMove::Scissors).unwrap();
        assert_eq!(state.get_score(), (1, 0));

        state = engine.update(&state, 1, RPSMove::Paper).unwrap();
        state = engine.update(&state, 2, RPSMove::Rock).unwrap();
        assert_eq!(state.get_score(), (2, 0));
        assert!(state.is_finished());
        assert_eq!(state.get_winner(), Some(1));
    }

    #[test]
    fn test_redact_for_player1() {
        let mut state = RPSGameState::new();

        // Round 1: Both players moved (completed round)
        state.rounds[0] = (Some(RPSMove::Rock), Some(RPSMove::Paper));

        // Round 2: Player 1 moved, player 2 hasn't
        state.rounds.push((Some(RPSMove::Scissors), None));

        // Round 3: Player 2 moved, player 1 hasn't
        state.rounds.push((None, Some(RPSMove::Rock)));

        // Round 4: No one moved yet
        state.rounds.push((None, None));

        let redacted = state.redact_for_player(1);

        // Round 1: Both moved, so both should be visible
        assert_eq!(redacted.rounds[0], (Some(RPSMove::Rock), Some(RPSMove::Paper)));

        // Round 2: Player 1 moved but player 2 hasn't, p1 sees their move
        assert_eq!(redacted.rounds[1], (Some(RPSMove::Scissors), None));

        // Round 3: Player 2 moved but player 1 hasn't, p2's move should be redacted
        assert_eq!(redacted.rounds[2], (None, Some(RPSMove::Redacted)));

        // Round 4: No one moved, both None
        assert_eq!(redacted.rounds[3], (None, None));
    }

    #[test]
    fn test_redact_for_player2() {
        let mut state = RPSGameState::new();

        // Round 1: Both players moved (completed round)
        state.rounds[0] = (Some(RPSMove::Rock), Some(RPSMove::Paper));

        // Round 2: Player 1 moved, player 2 hasn't
        state.rounds.push((Some(RPSMove::Scissors), None));

        // Round 3: Player 2 moved, player 1 hasn't
        state.rounds.push((None, Some(RPSMove::Rock)));

        // Round 4: No one moved yet
        state.rounds.push((None, None));

        let redacted = state.redact_for_player(2);

        // Round 1: Both moved, so both should be visible
        assert_eq!(redacted.rounds[0], (Some(RPSMove::Rock), Some(RPSMove::Paper)));

        // Round 2: Player 1 moved but player 2 hasn't, p1's move should be redacted
        assert_eq!(redacted.rounds[1], (Some(RPSMove::Redacted), None));

        // Round 3: Player 2 moved but player 1 hasn't, p2 sees their move
        assert_eq!(redacted.rounds[2], (None, Some(RPSMove::Rock)));

        // Round 4: No one moved, both None
        assert_eq!(redacted.rounds[3], (None, None));
    }
}
