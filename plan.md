# Briscola Implementation Plan

## Overview
Implement Briscola (Italian card game) following the same architecture pattern as Rock Paper Scissors.

## File Structure
Three main files will be created:
- `common/src/games/briscola.rs` - Shared game state and data structures
- `server/src/games/briscola.rs` - Game engine and logic
- `client/src/games/briscola.rs` - Client UI and game loop

Integration points:
- `common/src/games/game_type.rs` - Add `Briscola` variant to `GameType` enum
- `common/src/games/mod.rs` - Export `pub mod briscola;`
- `server/src/games/mod.rs` - Export `pub mod briscola;`
- `server/src/game_router.rs` - Add briscola handlers to `handle_game_move`, `redact_match_for_player`, and `initialize_game_state`
- `client/src/games/mod.rs` - Export `pub mod briscola;`
- `client/src/main.rs` - Add menu option, routing in `start_game_flow` and resume logic

## Phase 1: Common Structures (`common/src/games/briscola.rs`)

### 1.1 Card System
Define the fundamental card structures:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Suit {
    Bastoni,
    Coppe,
    Denari,
    Spade,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Rank {
    Two,    //  0 Points
    Four,   //  0 Points
    Five,   //  0 Points
    Six,    //  0 Points
    Seven,  //  0 Points
    Jack,   //  2 Points
    Knight, //  3 Points
    King,   //  4 Points
    Three,  // 10 Points
    Ace,    // 11 Points
}

// A single card
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

// Card or redacted (for hiding opponent's cards)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CardView {
    Visible(Card),
    Redacted,
}
```

### 1.2 Move Definition
```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BriscolaMove {
    PlayCard { card_index: usize }, // Index in player's hand (0-2, or fewer near end of game)
}
```

### 1.3 Game State

**Key Answer Points:**
- PlayerSymbol is a type alias for i32 (defined in `common/src/games/players.rs`)
- Hands are `Vec<Card>` (not `Vec<Option<Card>>`)
- Opponent hand should be empty Vec when redacted
- Deck should be empty Vec when redacted, but we need to add a field for cards_remaining_in_deck
- Trump card goes into the last player's hand who draws when deck becomes empty
- cards_remaining_in_deck represents deck.len() only (trump card counted separately)

```rust
use crate::games::players::PlayerSymbol;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoundState {
    AwaitingFirstCard,   // Waiting for first player to play
    AwaitingSecondCard,  // Waiting for second player to play
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BriscolaGameState {
    // Player hands - will be Vec<Card> for own hand, empty Vec for opponent
    pub player1_hand: Vec<Card>,
    pub player2_hand: Vec<Card>,

    // Cards currently on the table: (card, player_who_played_it)
    pub table: Vec<(Card, PlayerSymbol)>,

    // The deck of remaining cards (shuffled once at game start)
    // This will be empty Vec when sent to clients
    pub deck: Vec<Card>,

    // Number of cards left in deck (visible to both players)
    pub cards_remaining_in_deck: usize,

    // The trump card (visible to both players, None after it's drawn)
    pub trump_card: Option<Card>,

    // Collected cards (for scoring)
    pub player1_pile: Vec<Card>,
    pub player2_pile: Vec<Card>,

    // Whose turn is it
    pub current_player: PlayerSymbol,

    // Current round state
    pub round_state: RoundState,
}
```

### 1.4 Key Methods on BriscolaGameState

**Key Answer Points:**
- Redaction should create a deep clone (following RPS pattern)
- Opponent hand becomes empty Vec
- Deck becomes empty Vec (cards_remaining_in_deck shows count)

```rust
impl BriscolaGameState {
    pub fn new() -> Self { /* Create initial state */ }

    // Redact opponent's hand and deck (similar to RPS redacting unrevealed moves)
    pub fn redact_for_player(&self, player: PlayerSymbol) -> Self {
        let mut redacted = self.clone();

        // Hide opponent's hand (replace with empty Vec)
        if player == 1 {
            redacted.player2_hand = Vec::new();
        } else {
            redacted.player1_hand = Vec::new();
        }

        // Hide deck (replace with empty Vec, but keep cards_remaining_in_deck)
        redacted.deck = Vec::new();

        // Keep everything else visible (table, trump, piles, own hand, cards_remaining_in_deck)
        redacted
    }

    // Calculate score from collected piles
    pub fn get_score(&self) -> (u8, u8) {
        // Sum point values of cards in each pile
        // Ace=11, Three=10, King=4, Knight=3, Jack=2, others=0
    }

    // Check if game is finished
    pub fn is_finished(&self) -> bool {
        // All 40 cards have been played
        // (both hands empty and deck is empty and no trump card left)
        self.player1_hand.is_empty() &&
        self.player2_hand.is_empty() &&
        self.deck.is_empty() &&
        self.trump_card.is_none()
    }

    // Get the winner (if finished)
    pub fn get_winner(&self) -> Option<PlayerSymbol> {
        // Compare scores, return higher score's player
        // Can be None if tie (valid result per answer #26)
        let (p1_score, p2_score) = self.get_score();
        if p1_score > p2_score {
            Some(1)
        } else if p2_score > p1_score {
            Some(2)
        } else {
            None // Tie
        }
    }

    // Helper: Get point value of a card
    fn card_points(card: &Card) -> u8 {
        match card.rank {
            Rank::Ace => 11,      // Asso
            Rank::Three => 10,    // Tre
            Rank::King => 4,      // Re
            Rank::Knight => 3,    // Cavallo
            Rank::Jack => 2,      // Fante
            _ => 0,               // 2, 4, 5, 6, 7 have no points
        }
    }
}
```

## Phase 2: Server Engine (`server/src/games/briscola.rs`)

### 2.1 Game Engine Structure
```rust
pub struct BriscolaGameEngine;

impl BriscolaGameEngine {
    // Create a new game with shuffled deck
    pub fn new() -> BriscolaGameState {
        // 1. Create 40-card deck (4 suits × 10 ranks)
        // 2. Shuffle deck using rand::thread_rng()
        // 3. Deal 3 cards to each player (pop from deck)
        // 4. Set trump card (pop next card from deck, this stays visible)
        // 5. Remaining deck stored in state.deck
        // 6. Initialize empty piles
        // 7. Set player 1 as current player
        // 8. Set round_state to AwaitingFirstCard
    }

    // Update game state with a player's move
    pub fn update(
        &self,
        state: &BriscolaGameState,
        player: PlayerSymbol,
        move_choice: BriscolaMove,
    ) -> Result<BriscolaGameState, GameError> {
        // See detailed logic below
    }
}
```

### 2.2 Update Logic (Core Game Logic)

```rust
pub fn update(...) -> Result<BriscolaGameState, GameError> {
    // 1. Validate game is in progress
    if state.is_finished() {
        return Err(GameError::GameNotInProgress);
    }

    // 2. Validate it's the player's turn
    if state.current_player != player {
        return Err(GameError::WrongTurn);
    }

    // 3. Validate player number
    if player != 1 && player != 2 {
        return Err(GameError::InvalidPlayer);
    }

    // 4. Extract card index from move
    let BriscolaMove::PlayCard { card_index } = move_choice;

    // 5. Validate player has card at that index
    let hand = if player == 1 { &state.player1_hand } else { &state.player2_hand };
    if card_index >= hand.len() {
        return Err(GameError::IllegalMove("Invalid card index".to_string()));
    }

    // 6. Get the card being played
    let card = hand[card_index];

    // 7. Create new state and remove card from hand
    let mut new_state = state.clone();
    if player == 1 {
        new_state.player1_hand.remove(card_index);
    } else {
        new_state.player2_hand.remove(card_index);
    }

    // 8. Add card to table
    new_state.table.push((card, player));

    // 9. Handle based on round state
    match state.round_state {
        RoundState::AwaitingFirstCard => {
            // First card played, switch to waiting for second
            new_state.round_state = RoundState::AwaitingSecondCard;
            new_state.current_player = if player == 1 { 2 } else { 1 };
        }
        RoundState::AwaitingSecondCard => {
            // Second card played, resolve the round
            new_state = resolve_round(new_state)?;
        }
    }

    Ok(new_state)
}
```

### 2.3 Round Resolution Logic

```rust
fn resolve_round(mut state: BriscolaGameState) -> Result<BriscolaGameState, GameError> {
    // 1. Determine round winner
    let (first_card, first_player) = state.table[0];
    let (second_card, second_player) = state.table[1];

    let round_winner = determine_round_winner(
        first_card,
        second_card,
        state.trump_card.unwrap().suit,
        first_player,
    );

    // 2. Award both cards to winner's pile
    if round_winner == 1 {
        state.player1_pile.push(first_card);
        state.player1_pile.push(second_card);
    } else {
        state.player2_pile.push(first_card);
        state.player2_pile.push(second_card);
    }

    // 3. Clear table
    state.table.clear();

    // 4. Draw new cards (if deck not empty)
    if !state.deck.is_empty() {
        // Winner draws first
        draw_card_to_player(&mut state, round_winner);

        if !state.deck.is_empty() {
            // Loser draws second
            let other_player = if round_winner == 1 { 2 } else { 1 };
            draw_card_to_player(&mut state, other_player);
        }
    }

    // 5. Winner of round starts next round
    state.current_player = round_winner;
    state.round_state = RoundState::AwaitingFirstCard;

    Ok(state)
}

/// Determine the winner of a round based on Briscola rules (see answer #11)
///
/// Rules:
/// 1. If both cards are briscola (trump), higher rank wins
/// 2. If only one card is briscola, it wins
/// 3. If neither is briscola:
///    - If same suit as first card, higher rank wins
///    - If different suit, first card wins
///
/// Note: trump_suit is always available during gameplay. Trump card is drawn at
/// the end of a round, so determine_round_winner is never called when trump_card is None.
fn determine_round_winner(
    first_card: Card,
    second_card: Card,
    trump_suit: Suit,
    first_player: PlayerSymbol,
) -> PlayerSymbol {
    let first_is_trump = first_card.suit == trump_suit;
    let second_is_trump = second_card.suit == trump_suit;

    // Case 1: Both are trump - higher rank wins
    if first_is_trump && second_is_trump {
        if rank_value(first_card.rank) > rank_value(second_card.rank) {
            first_player
        } else {
            if first_player == 1 { 2 } else { 1 }
        }
    }
    // Case 2: Only first is trump - first wins
    else if first_is_trump {
        first_player
    }
    // Case 3: Only second is trump - second wins
    else if second_is_trump {
        if first_player == 1 { 2 } else { 1 }
    }
    // Case 4: Neither is trump
    else {
        // If same suit as first card, higher rank wins
        if first_card.suit == second_card.suit {
            if rank_value(first_card.rank) > rank_value(second_card.rank) {
                first_player
            } else {
                if first_player == 1 { 2 } else { 1 }
            }
        } else {
            // Different suits, neither trump - first card wins
            first_player
        }
    }
}

// Rank ordering for comparison (Ace is highest in playing strength)
fn rank_value(rank: Rank) -> u8 {
    match rank {
        Rank::Ace => 11,
        Rank::Three => 10,
        Rank::King => 9,
        Rank::Knight => 8,
        Rank::Jack => 7,
        Rank::Seven => 6,
        Rank::Six => 5,
        Rank::Five => 4,
        Rank::Four => 3,
        Rank::Two => 2,
    }
}

/// Draw a card to a player's hand
/// Handles drawing from deck and trump card (see answer #7 and #9)
///
/// Rules:
/// - If deck has cards, draw from deck
/// - If deck is empty but trump card exists, draw the trump card
/// - Update cards_remaining_in_deck counter
/// - cards_remaining_in_deck = deck.len() only (trump counted separately in UI)
fn draw_card_to_player(state: &mut BriscolaGameState, player: PlayerSymbol) {
    let card_to_draw = if !state.deck.is_empty() {
        // Draw from deck
        Some(state.deck.pop().unwrap())
    } else if let Some(trump) = state.trump_card.take() {
        // Deck is empty, draw the trump card
        Some(trump)
    } else {
        None
    };

    if let Some(card) = card_to_draw {
        // Add to appropriate player's hand
        if player == 1 {
            state.player1_hand.push(card);
        } else {
            state.player2_hand.push(card);
        }
        // Update counter: only counts deck cards, not trump
        state.cards_remaining_in_deck = state.deck.len();
    }
}
```

### 2.4 Deck Management

**Key Answer Points:**
- Initial deck: 40 cards → 3 to P1, 3 to P2, 1 trump = 33 remaining (answer #8)
- Trump card is drawn when deck is empty (answer #7, #9)
- When only 1 card left, winner draws it, loser draws trump (answer #9)
- Deck stored as Vec<Card> (not Vec<Option<Card>>)
- rand crate already in server Cargo.toml (answer #24)
- cards_remaining_in_deck = deck.len() only (33 initially, does not include trump)

The deck is maintained in `BriscolaGameState` and created once at game initialization:

```rust
// In BriscolaGameEngine::new():
fn create_and_shuffle_deck() -> Vec<Card> {
    let mut deck = Vec::new();

    // Create all 40 cards
    for suit in [Suit::Bastoni, Suit::Coppe, Suit::Denari, Suit::Spade] {
        for rank in [
            Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five,
            Rank::Six, Rank::Seven, Rank::Jack, Rank::Knight, Rank::King
        ] {
            deck.push(Card { suit, rank });
        }
    }

    // Shuffle using rand crate (already in server Cargo.toml)
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    deck.shuffle(&mut thread_rng());

    deck
}

// Initial game setup:
// 1. Create shuffled 40-card deck
// 2. Deal 3 cards to player 1
// 3. Deal 3 cards to player 2
// 4. Set trump card (next card from deck)
// 5. Remaining 33 cards stay in deck
// 6. cards_remaining_in_deck = 33 (deck.len() only, trump shown separately in UI)
```

## Phase 3: Client Implementation (`client/src/games/briscola.rs`)

**Key Answer Points:**
- Follow RPS/TicTacToe patterns exactly (answer #34)
- Use tokio::io for async stdin (answer #15)
- No command to show collected piles (answer #16)
- Card display: use suit names, not Unicode symbols (answer #18)
- Reconnecting players see state immediately (answer #28)
- GameStateUpdate sent every move (answer #30)
- Use println for debugging (answer #35)

### 3.1 UI State Machine

```rust
use battld_common::{games::{game_type::GameType, matches::{Match, MatchEndReason, MatchOutcome}, briscola::BriscolaGameState}, *};
use crate::state::SessionState;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use colored::*;

#[derive(Debug, Clone)]
enum BriscolaUiState {
    WaitingForOpponentToJoin,

    PlayingGame {
        match_data: Match,
        your_turn: bool,
        opponent_disconnected: bool,
    },

    WaitingForOpponentToReconnect {
        match_data: Match,
    },

    MatchEndedYouWon(Match),
    MatchEndedYouLost(Match),
    MatchEndedDraw(Match),
    MatchEndedOpponentDisconnected(Match),
}
```

### 3.2 Rendering Logic

```rust
impl BriscolaUiState {
    fn render(&self, my_player_number: i32) {
        match self {
            BriscolaUiState::PlayingGame { match_data, your_turn, .. } => {
                let game_state = parse_game_state(match_data);

                // Header
                println!("=== BRISCOLA ===");
                println!();

                // Scores
                let (p1_score, p2_score) = game_state.get_score();
                let (my_score, opp_score) = if my_player_number == 1 {
                    (p1_score, p2_score)
                } else {
                    (p2_score, p1_score)
                };
                println!("Score: You {} - {} Opponent", my_score, opp_score);
                println!();

                // Trump card and deck count
                if let Some(trump) = game_state.trump_card {
                    println!("Trump: {}", format_card(&trump));
                    println!("Deck: {} cards remaining", game_state.cards_remaining_in_deck);
                } else {
                    println!("Trump: (drawn)");
                    println!("Deck: {} cards remaining", game_state.cards_remaining_in_deck);
                }
                println!();

                // Table (cards played this round)
                if !game_state.table.is_empty() {
                    println!("On table:");
                    for (card, player) in &game_state.table {
                        let who = if *player == my_player_number { "You" } else { "Opponent" };
                        println!("  {} played {}", who, format_card(card));
                    }
                    println!();
                }

                // Opponent's hand (just count)
                let opp_hand = if my_player_number == 1 {
                    &game_state.player2_hand
                } else {
                    &game_state.player1_hand
                };
                println!("Opponent has {} cards", opp_hand.len());
                println!();

                // Your hand
                let my_hand = if my_player_number == 1 {
                    &game_state.player1_hand
                } else {
                    &game_state.player2_hand
                };
                println!("Your hand:");
                for (i, card) in my_hand.iter().enumerate() {
                    println!("  [{}] {}", i, format_card(card));
                }
                println!();

                // Input prompt
                if *your_turn {
                    println!("Your turn! Enter card index (0-{}):", my_hand.len() - 1);
                    print!("> ");
                } else {
                    println!("Waiting for opponent...");
                }
            }
            // ... other states
        }
    }
}

/// Format a card for display (answer #18: use names, not Unicode symbols)
fn format_card(card: &Card) -> String {
    let suit_str = match card.suit {
        Suit::Bastoni => "Bastoni",
        Suit::Coppe => "Coppe",
        Suit::Denari => "Denari",
        Suit::Spade => "Spade",
    };
    let rank_str = match card.rank {
        Rank::Ace => "A",
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Jack => "J",
        Rank::Knight => "C", // Cavallo
        Rank::King => "K",
    };
    format!("{} {}", rank_str, suit_str)
}
```

### 3.3 Game Loop

**Key Answer Points:**
- Move message format: JSON with card_index field (answer #29)
- Follow RPS pattern exactly (answer #34)
- Invalid card handling: return error, don't disconnect (answer #20)
- Disconnection handled like TicTacToe (answer #27)

Similar to RPS, with:
- WebSocket message handling for `GameStateUpdate`, `MatchEnded`, etc.
- User input handling for card selection (enter card index 0-2)
- State transitions based on messages
- Rendering on state changes
- Error handling similar to TicTacToe

```rust
async fn run_game_loop(
    ws_client: &crate::websocket::WebSocketClient,
    my_player_id: i64,
    initial_state: BriscolaUiState,
    initial_my_number: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Pattern follows rock_paper_scissors.rs exactly
    // Main loop using tokio::select! for async stdin and WebSocket messages
    // Handle PlayerDisconnected, MatchEnded, MatchFound, GameStateUpdate
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Connect WebSocket, send JoinMatchmaking, start game loop
}

pub async fn resume_game(session: &mut SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    // Reconnect to existing game, resume game loop
}
```

## Phase 4: Integration

### 4.1 Add to GameType Enum
**File:** `common/src/games/game_type.rs`

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GameType {
    TicTacToe,
    RockPaperScissors,
    Briscola,  // Add this
}

impl fmt::Display for GameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameType::TicTacToe => write!(f, "Tic-Tac-Toe"),
            GameType::RockPaperScissors => write!(f, "Rock-Paper-Scissors"),
            GameType::Briscola => write!(f, "Briscola"),  // Add this
        }
    }
}
```

### 4.2 Export Modules
**File:** `common/src/games/mod.rs`
```rust
pub mod rock_paper_scissors;
pub mod tic_tac_toe;
pub mod briscola;  // Add this line
pub mod game_type;
pub mod matches;
pub mod players;
```

**File:** `server/src/games/mod.rs`
```rust
pub mod tic_tac_toe;
pub mod rock_paper_scissors;
pub mod briscola;  // Add this line
```

**File:** `client/src/games/mod.rs`
```rust
pub mod rock_paper_scissors;
pub mod tic_tac_toe;
pub mod briscola;  // Add this line
```

### 4.3 Wire Up Server Router
**File:** `server/src/game_router.rs`

Add imports:
```rust
use crate::games::{tic_tac_toe::*, rock_paper_scissors::*, briscola::*, GameError};
use battld_common::games::{
    game_type::GameType,
    matches::{Match, MatchOutcome},
    rock_paper_scissors::{RockPaperScissorsGameState, RockPaperScissorsMove},
    briscola::BriscolaGameState,  // Add this
};
```

Update `handle_game_move`:
```rust
pub fn handle_game_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    match game_match.game_type {
        GameType::TicTacToe => handle_tic_tac_toe_move(game_match, player_id, move_data),
        GameType::RockPaperScissors => handle_rock_paper_scissors_move(game_match, player_id, move_data),
        GameType::Briscola => handle_briscola_move(game_match, player_id, move_data),  // Add this
    }
}
```

Add handler function (following RPS pattern):
```rust
fn handle_briscola_move(
    game_match: &Match,
    player_id: i64,
    move_data: JsonValue,
) -> Result<GameMoveResult, GameError> {
    // Deserialize current state
    let current_state: BriscolaGameState = serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| GameError::IllegalMove(format!("Invalid game state: {e}")))?;

    // Deserialize move - expects {"card_index": 0}
    #[derive(serde::Deserialize)]
    struct BriscolaMoveData {
        card_index: usize,
    }

    let move_data: BriscolaMoveData = serde_json::from_value(move_data)
        .map_err(|e| GameError::IllegalMove(format!("Invalid move data: {e}")))?;

    // Determine player symbol
    let player_symbol = if player_id == game_match.player1_id {
        1
    } else if player_id == game_match.player2_id {
        2
    } else {
        return Err(GameError::InvalidPlayer);
    };

    // Call engine
    let engine = BriscolaGameEngine;
    let new_state = engine.update(&current_state, player_symbol, BriscolaMove::PlayCard { card_index: move_data.card_index })?;

    // Serialize new state
    let new_state_json = serde_json::to_value(&new_state)
        .map_err(|e| GameError::IllegalMove(format!("Failed to serialize state: {e}")))?;

    // Determine outcome
    let outcome = if new_state.is_finished() {
        match new_state.get_winner() {
            Some(1) => Some(MatchOutcome::Player1Win),
            Some(2) => Some(MatchOutcome::Player2Win),
            _ => Some(MatchOutcome::Draw),  // Tie is valid
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
```

Update `redact_match_for_player`:
```rust
pub fn redact_match_for_player(match_data: &Match, player_id: i64) -> Match {
    let player_num = if player_id == match_data.player1_id { 1 }
        else if player_id == match_data.player2_id { 2 }
        else { return match_data.clone(); };

    let redacted_state = match match_data.game_type {
        GameType::TicTacToe => { /* existing code */ }
        GameType::RockPaperScissors => { /* existing code */ }
        GameType::Briscola => {  // Add this
            match serde_json::from_value::<BriscolaGameState>(match_data.game_state.clone()) {
                Ok(state) => {
                    let redacted = state.redact_for_player(player_num);
                    serde_json::to_value(&redacted).unwrap_or(match_data.game_state.clone())
                }
                Err(_) => match_data.game_state.clone(),
            }
        }
    };

    Match { /* ... with redacted_state ... */ }
}
```

Update `initialize_game_state`:
```rust
pub fn initialize_game_state(game_type: &GameType) -> String {
    let first_player = {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.5) { 1 } else { 2 }
    };

    match game_type {
        GameType::TicTacToe => { /* existing code */ }
        GameType::RockPaperScissors => { /* existing code */ }
        GameType::Briscola => {  // Add this
            let mut state = BriscolaGameEngine::new();
            state.current_player = first_player;
            serde_json::to_string(&state).unwrap()
        }
    }
}
```

### 4.4 Wire Up Client Router
**File:** `client/src/main.rs`

Add to imports:
```rust
use crate::games::{rock_paper_scissors, tic_tac_toe, briscola};
```

Add menu choice enum variant:
```rust
enum MenuChoice {
    StartTicTacToe,
    StartRockPaperScissors,
    StartBriscola,  // Add this
    Stats,
    Leaderboard,
    Exit,
}
```

Update menu display to show all options:
```rust
// Update menu items array to include Briscola as option 3
let menu_items = vec![
    "1. Play Tic-Tac-Toe",
    "2. Play Rock-Paper-Scissors",
    "3. Play Briscola",  // Add this line
    "4. View Stats",     // Was 3
    "5. View Leaderboard", // Was 4
    "6. Exit",           // Was 5
];
```

Add menu option in `read_menu_choice`:
```rust
// In the menu display and parsing code, update:
"1" => MenuChoice::StartTicTacToe,
"2" => MenuChoice::StartRockPaperScissors,
"3" => MenuChoice::StartBriscola,  // Add this
"4" => MenuChoice::Stats,          // Was "3"
"5" => MenuChoice::Leaderboard,    // Was "4"
"6" => MenuChoice::Exit,           // Was "5"
```

Add to main loop (around line 70-85):
```rust
MenuChoice::StartBriscola => {
    if let Err(e) = start_game_flow(&mut session, GameType::Briscola).await {
        println!("{}", format!("Game error: {e}").red());
        println!("\nPress any key to return to menu...");
        wait_for_keypress()?;
    }
}
```

Update `check_and_handle_resumable_match` (around line 204-211):
```rust
match game_match.game_type {
    GameType::TicTacToe => {
        tic_tac_toe::resume_game(session, game_match).await?;
    }
    GameType::RockPaperScissors => {
        rock_paper_scissors::resume_game(session, game_match).await?;
    }
    GameType::Briscola => {  // Add this
        briscola::resume_game(session, game_match).await?;
    }
}
```

Update `start_game_flow` (around line 243-246):
```rust
match game_type {
    GameType::TicTacToe => games::tic_tac_toe::start_game(session, game_type).await?,
    GameType::RockPaperScissors => games::rock_paper_scissors::start_game(session, game_type).await?,
    GameType::Briscola => games::briscola::start_game(session, game_type).await?,  // Add this
}
```

## Phase 5: Testing

**Key Answer Points:**
- Use fixed (non-random) decks for deterministic testing (answer #21)
- No existing test utilities to reuse (answer #22)
- Include serialization/deserialization round-trip tests (answer #23)
- Follow RPS test patterns (answer #34)

### 5.1 Unit Tests (server/src/games/briscola.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_points() {
        // Test point values for all ranks
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Ace }), 11);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Three }), 10);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::King }), 4);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Knight }), 3);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Jack }), 2);
        assert_eq!(BriscolaGameState::card_points(&Card { suit: Suit::Bastoni, rank: Rank::Two }), 0);
    }

    #[test]
    fn test_round_winner_both_trump() {
        // Use fixed deck - both trump, higher rank wins
        // Example from answer #11
    }

    #[test]
    fn test_round_winner_one_trump() {
        // Trump beats non-trump
        // Examples from answer #11
    }

    #[test]
    fn test_round_winner_same_suit_not_trump() {
        // Higher rank of same suit wins when neither is trump
        // Examples from answer #11
    }

    #[test]
    fn test_round_winner_different_suits_no_trump() {
        // First card wins when different suits, neither trump
        // Examples from answer #11
    }

    #[test]
    fn test_play_card_valid() {
        // Valid move updates state correctly
        // Use fixed deck for deterministic test
    }

    #[test]
    fn test_play_card_invalid_index() {
        // Returns error for invalid card index
    }

    #[test]
    fn test_play_card_wrong_turn() {
        // Returns GameError::WrongTurn when not player's turn
    }

    #[test]
    fn test_round_resolution() {
        // Cards awarded to winner's pile
        // New cards drawn from deck
        // Winner starts next round
        // Use fixed deck
    }

    #[test]
    fn test_trump_card_drawing() {
        // Test that when deck is empty, trump card is drawn
        // Test that winner draws last card, loser draws trump (answer #9)
    }

    #[test]
    fn test_game_finish() {
        // Game ends when all cards played (both hands empty, deck empty, trump drawn)
        // Winner determined by score
    }

    #[test]
    fn test_game_finish_with_tie() {
        // Test that tie returns None winner (answer #26)
    }

    #[test]
    fn test_redaction() {
        // Opponent's hand is empty Vec
        // Own hand is visible
        // Deck is empty Vec, but cards_remaining_in_deck shows count
        // Table, trump, piles visible to both
    }

    #[test]
    fn test_serialization_roundtrip() {
        // Serialize to JSON and deserialize back (answer #23)
        let mut state = BriscolaGameState::new();
        let json = serde_json::to_value(&state).unwrap();
        let deserialized: BriscolaGameState = serde_json::from_value(json).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_state_immutability() {
        // Original state unchanged after update (following RPS pattern)
    }
}
```

### 5.2 Integration Testing
- Start a game and play through to completion
- Test disconnection/reconnection (handled like TicTacToe per answer #27)
- Test invalid moves are rejected with error (don't disconnect per answer #20)
- Test reconnection shows state immediately (answer #28)

## Implementation Order

**Key Answer Point:**
- Implement sequentially, but anticipate where it makes sense (answer #32)

1. **Phase 1** - Common structures (foundation for everything else)
2. **Phase 2** - Server engine (core game logic)
3. **Phase 4.1-4.2** - Add to enums and wire up modules (enables compilation)
4. **Phase 4.3** - Wire up server router (enables server to handle briscola)
5. **Phase 3** - Client implementation (requires server to be working)
6. **Phase 4.4** - Wire up client router (enables client to play briscola)
7. **Phase 5** - Testing and refinement

## Key Design Decisions

### Deck Management
The deck is shuffled **once** at game initialization and stored in `BriscolaGameState.deck` as `Vec<Card>`:
- Ensures consistent, reproducible game state
- Allows the server to maintain the full deck
- Enables redaction: clients receive empty `Vec<Card>`
- Separate `cards_remaining_in_deck` field preserves visibility of how many cards remain
  - `cards_remaining_in_deck = deck.len()` (does NOT include trump card)
  - When deck has 1 card: `cards_remaining_in_deck = 1` (plus trump exists separately)
  - When deck is empty but trump exists: `cards_remaining_in_deck = 0`
  - After trump is drawn: `cards_remaining_in_deck = 0` and `trump_card = None`
- Cards are drawn by popping from the deck vector
- Trump card is drawn when deck becomes empty (answers #7, #9)
- When 1 card left in deck: winner draws it, loser draws trump (answer #9)

### Redaction Strategy
Similar to Rock Paper Scissors hiding unrevealed moves, Briscola hides:
- Opponent's hand (replace with empty `Vec<Card>`)
- Deck contents (replace with empty `Vec<Card>`, but keep `cards_remaining_in_deck`)
- Everything else remains visible: table cards, trump, piles, scores, current_player

### Trump Card Special Rules (Answers #7, #9)
1. Trump card is displayed separately and visible to both players
2. When deck is empty but trump exists, next drawer gets the trump card
3. Trump card is set to `None` after being drawn
4. This is the last card drawn in the game

## Key Briscola Rules Reference

- **Deck:** 40 cards (4 suits × 10 ranks)
- **Initial Deal:** 3 cards per player, 1 trump card, 33 remain in deck (answer #8)
- **Trump:** One card revealed, determines trump suit (briscola suit)
- **Gameplay:** Players alternate playing one card per round
- **Round Winner** (detailed rules in answer #11):
  1. Both briscola: higher rank wins
  2. One briscola: briscola wins
  3. Neither briscola, same suit: higher rank wins
  4. Neither briscola, different suits: first card wins
- **Card Drawing:** Winner draws first, then loser (if cards remain)
  - When deck empty: draw trump card (answer #7)
  - When 1 card left: winner draws it, loser draws trump (answer #9)
- **Rank Strength** (answer #10): Ace (11) > Three (10) > King (4) > Knight (3) > Jack (2) > Seven > Six > Five > Four > Two
- **Scoring:** Ace=11, Three=10, King=4, Knight=3, Jack=2, others=0
- **Game End:** All 40 cards played (both hands empty, deck empty, trump drawn)
- **Winner:** Highest total score from collected cards (tie is valid - answer #26)

---

## Summary of Key Clarifications from Answers

### Architecture & Patterns
- **PlayerSymbol**: Type alias for `i32` defined in `common/src/games/players.rs`
- **Pattern to follow**: Rock Paper Scissors and TicTacToe implementations exactly
- **Server dispatch**: Via `game_router.rs` functions: `handle_game_move`, `redact_match_for_player`, `initialize_game_state`
- **Client routing**: Via `client/src/main.rs` match statements for `start_game_flow` and resume logic
- **Match struct**: Contains `game_state: serde_json::Value` (generic JSON for all games)

### Data & Serialization
- **Hands**: `Vec<Card>` (not `Vec<Option<Card>>`)
- **Redaction**: Deep clone with opponent hand = empty Vec, deck = empty Vec
- **Cards remaining visibility**: Via `cards_remaining_in_deck: usize` field
  - `cards_remaining_in_deck = deck.len()` (trump NOT included in this count)
  - Initial value: 33
  - When deck empty but trump exists: 0
  - After trump drawn: 0 (and `trump_card = None`)
- **Trump card lifecycle**: Visible → drawn when deck empty → set to None
- **Move format**: JSON `{"card_index": 0}` (answer #29)
- **Dependencies**: rand and serde already in Cargo.toml (answer #24)

### Game Logic
- **Initial setup**: 40 cards → 3 to P1, 3 to P2, 1 trump, 33 in deck
- **Trump drawing**: Last card drawn, winner gets last deck card, loser gets trump
- **Round winner rules**: Comprehensive logic in answer #11 with 8 examples
- **Rank ordering**: Strength vs points (same numeric values but conceptually different)
- **Tie games**: Valid outcome returning None for winner

### Error Handling & Testing
- **GameError variants**: IllegalMove, GameNotInProgress, WrongTurn, InvalidPlayer (no additions)
- **Invalid moves**: Return error, don't disconnect (like TicTacToe)
- **Testing approach**: Fixed decks, serialization round-trips, follow RPS patterns
- **Disconnection**: Handle like TicTacToe with 10s grace period
- **Reconnection**: Immediate state visibility

### Client Implementation
- **Input library**: tokio::io async stdin (from RPS/TicTacToe)
- **Card display**: Use names (e.g., "A Bastoni"), not Unicode symbols
- **No pile viewing**: Don't implement command to show collected piles
- **Message triggers**: GameStateUpdate on every move
- **Debugging**: Use println for output

---

## Final Clarifications

1. **Client UI Display**: Pure text output, print server events and ask for input (follow TicTacToe/RPS exactly)
   - Show scores, deck count, trump card, table cards, and hand
   - Simple text-based rendering

2. **Error Messages**: No special error message handling needed - server panics are acceptable
   - Use generic error messages like "Invalid card index" or "Illegal move"
   - Focus on correct game logic rather than polished error messages

3. **Deck Counter Logic**: `cards_remaining_in_deck` does NOT include trump card
   - `cards_remaining_in_deck = deck.len()` (trump counted separately)
   - Initial value: 33
   - When 1 card in deck: `cards_remaining_in_deck = 1` (plus trump visible separately)
   - When deck empty: `cards_remaining_in_deck = 0` (trump may still be drawable)

4. **Menu Option**: Add "3. Play Briscola", update Stats to 4, Leaderboard to 5, Exit to 6

---

## Implementation Checklist

The plan is now **complete and ready for implementation**. Follow this checklist:

- [ ] Phase 1: Create `common/src/games/briscola.rs` with all data structures
- [ ] Phase 2: Create `server/src/games/briscola.rs` with game engine
- [ ] Phase 4.1: Add `Briscola` to `GameType` enum in `common/src/games/game_type.rs`
- [ ] Phase 4.2: Export modules in all three `mod.rs` files
- [ ] Phase 4.3: Wire up server router in `server/src/game_router.rs`
- [ ] Phase 3: Create `client/src/games/briscola.rs` with UI and game loop
- [ ] Phase 4.4: Wire up client router in `client/src/main.rs`
- [ ] Phase 5: Add unit tests to server implementation
- [ ] Integration test: Play a complete game
- [ ] Test disconnection/reconnection scenarios

**All architectural questions resolved. Ready to code!**
