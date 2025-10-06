# Battld Multi-Game Hub Migration Plan

**Note:** Backwards compatibility is not a concern for this plan as the project is not public. Database will just be dropped and re-created if we do any changes.

## Overview
Transitioning Battld from a single tic-tac-toe game to a hub supporting multiple multiplayer terminal games.

## Design Principles

### 1. Architecture
- **Message Protocol**: Keep existing WebSocket message types, make `ServerMessage::GameStateUpdate` dynamic (sacrifice type-safety for flexibility)
- **Game Engines**: Each game has its own "engine" module that:
  - Handles client messages specific to that game
  - Emits server messages (GameStateUpdate, etc.)
  - Contains all game-specific logic
- **Separation**: Game logic completely isolated from server/networking logic

### 2. Game Organization
- **One game = One module** (e.g., `games/tictactoe.rs`, `games/rockpaperscissors.rs`)
- Server orchestrates but doesn't know game rules
- Designed for N games (not just 2)

### 3. Matchmaking
- **Separate queues per game type**
- Players select game from main menu
- Each game type has its own waiting queue
- Match occurs when 2 players are in same game queue

### 4. Scoring
- **Unified score across all games**
- Player rating/ELO considers matches from all game types equally
- Single leaderboard

## Initial Games

### Tic-Tac-Toe (existing)
- Keep current implementation
- Refactor into new module structure

### Rock-Paper-Scissors (new)
- Best of 3 format
- After both players select: "showdown" reveals moves
- Immediate next round after showdown (if needed)
- Match ends when one player wins 2 rounds

## Technical Changes Needed

### phase 1: Pre-Migration Baseline & Schema Preparation
**Goal:** Establish baseline that everything works, verify/update database schema for multi-game support.

- [ ] Verify current system works end-to-end
  - [ ] Run all existing tests (if any)
  - [ ] Manual test: play full tic-tac-toe game between two clients
  - [ ] Verify matchmaking, gameplay, scoring all function correctly
  - [ ] Document any issues found before proceeding

- [ ] Verify and prepare database schema
  - [ ] Check current `matches` table schema
  - [ ] Verify `game_type` column exists (should be VARCHAR/TEXT)
    - If missing: add `game_type` column with default value "tris"
    - If exists: verify current data uses "tris" for tic-tac-toe
  - [ ] Verify `game_state` column can store JSON (should be TEXT or JSON type)
  - [ ] Run database migration if schema changes needed
  - [ ] Verify schema changes don't break current functionality

### phase 1: Refactor TicTacToe to Stateless Engine Pattern
**Goal:** Reorganize tic-tac-toe logic into stateless engine pattern. Same functionality, new structure. Remove `game_logic.rs`.

- [ ] Create `games/` module structure
  - [ ] Create `server/src/games/mod.rs`
  - [ ] Create `server/src/games/tictactoe.rs`

- [ ] Define error types in `games/mod.rs`
  - [ ] Create `GameError` enum with variants: `IllegalMove`, `GameNotInProgress`, etc.
  - [ ] Implement Display/Debug traits

- [ ] Implement TicTacToe components in `games/tictactoe.rs`
  - [ ] Define `TicTacToeMove` struct (row: usize, col: usize)
  - [ ] Define `TicTacToeGameState` struct (board, current_player, winner: Option<PlayerSymbol>, is_finished: bool)
  - [ ] Implement `TicTacToeGameState::new()` for fresh game initialization
  - [ ] Define `TicTacToeEngine` struct (stateless, zero-sized or unit struct)
  - [ ] Implement `TicTacToeEngine::update(&self, state: &TicTacToeGameState, player: PlayerSymbol, move: &TicTacToeMove) -> Result<TicTacToeGameState, GameError>`
    - Validate move is legal
    - Check correct player's turn
    - Apply move to create new state
    - Check for winner
    - Check for draw
    - Return new state with updated winner/is_finished fields

- [ ] Refactor server to use TicTacToeEngine
  - [ ] Update `Match` struct temporarily to hold `TicTacToeGameState` (not JSON yet)
  - [ ] Update `handle_make_move_logic()` to:
    - Get current game state from match
    - Create `TicTacToeMove` from client message
    - Call `TicTacToeEngine::update()`
    - Update match with new state
    - Send appropriate messages based on new state
  - [ ] Ensure matchmaking creates Match with fresh `TicTacToeGameState`

- [ ] Write comprehensive tests for TicTacToeEngine
  - [ ] Test valid moves
  - [ ] Test illegal moves (out of bounds, occupied cell, wrong turn)
  - [ ] Test win conditions (rows, columns, diagonals)
  - [ ] Test draw condition
  - [ ] Test game state immutability (old state unchanged after update)

- [ ] Remove old code
  - [ ] Delete `game_logic.rs`
  - [ ] Remove any unused imports/functions

- [ ] Validate everything works end-to-end
  - [ ] Manual testing: play full game client-to-client
  - [ ] Verify all existing functionality preserved

### phase 2: Multi-Game Infrastructure
**Goal:** Add infrastructure to support multiple game types. Still only TicTacToe exists, but system is ready for new games.

- [ ] Define game type system
  - [ ] Create `GameType` enum in `common/src/tris.rs` or `common/src/game.rs` (variants: TicTacToe, RockPaperScissors)
  - [ ] Implement serialization/deserialization for GameType
    - `GameType::TicTacToe` serializes to "tris" (matches existing DB values)
    - `GameType::RockPaperScissors` serializes to "rps"
  - [ ] Export from common crate so both client and server can use it

- [ ] Update message protocol for generic game support
  - [ ] Update `ClientMessage::MakeMove` to contain `move_data: serde_json::Value` instead of `row, col` fields
    - TicTacToe will serialize `TicTacToeMove { row, col }` to move_data
    - RPS will serialize `RPSMoveData { choice }` to move_data
  - [ ] Keep `ServerMessage::GameStateUpdate { match_data: Match }` unchanged (still sends full Match)

- [ ] Update Match struct for multi-game support (in common/src/tris.rs)
  - [ ] Change `game_state: GameState` to `game_state: serde_json::Value`
  - [ ] Change `game_type: String` to `game_type: GameType` (use the enum)
  - [ ] Client pattern matches on `game_type` enum to deserialize `game_state` to correct type:
    - `GameType::TicTacToe` → deserialize to `GameState`
    - `GameType::RockPaperScissors` → deserialize to `RPSGameState`
  - [ ] Server serializes game-specific state to JSON before creating Match

- [ ] Create game routing/dispatcher logic
  - [ ] Create `handle_game_move()` function that:
    - Takes Match, player_id, move_data JSON
    - Matches on `match.game_type` enum (GameType::TicTacToe, GameType::RockPaperScissors)
    - Deserializes state from JSON to appropriate type (e.g., TicTacToeGameState)
    - Deserializes move_data to appropriate type (e.g., TicTacToeMove)
    - Calls appropriate engine's update()
    - Serializes new state back to JSON
    - Returns updated state JSON and any server messages
  - [ ] Integrate router into `handle_make_move_logic()`

- [ ] Update matchmaking for game types
  - [ ] Add game type to matchmaking queue (even though only TicTacToe for now)
  - [ ] When creating Match, set `game_type: GameType::TicTacToe`
  - [ ] Queue structure: separate queue per game type (future-proof)
  - [ ] Database stores game_type as string (enum serializes to/from string)

- [ ] Update client message handling
  - [ ] Parse generic `move_data` JSON from client
  - [ ] Pass to game router/dispatcher

- [ ] Write tests for multi-game infrastructure
  - [ ] Test game type serialization/deserialization
  - [ ] Test game router with TicTacToe
  - [ ] Test Match with JSON state encoding/decoding
  - [ ] Test error handling for invalid game types

- [ ] Validate infrastructure
  - [ ] Play full TicTacToe game with new infrastructure
  - [ ] Verify all state transitions work correctly
  - [ ] Confirm no regressions from phase 1

### phase 3: Main Menu & Multi-Game Matchmaking
**Goal:** Update client menu and matchmaking to support multiple game types. Players can choose which game to play.

- [ ] Update common message types
  - [ ] Change `ClientMessage::JoinMatchmaking` to `JoinMatchmaking { game_type: GameType }`
  - [ ] Update serialization to include game_type field

- [ ] Update client main menu (client/src/main.rs)
  - [ ] Split "Start New Game" into two menu options:
    - "Start Tic-Tac-Toe Game"
    - "Start Rock-Paper-Scissors Game"
  - [ ] Update `MenuChoice` enum with `StartTicTacToe` and `StartRPS` variants
  - [ ] Update menu display and input handling

- [ ] Update client game flow (client/src/main.rs)
  - [ ] Modify `start_game_flow()` to accept `GameType` parameter
  - [ ] Pass GameType to `tris::start_game()` (or rename to generic `game::start_game()`)

- [ ] Update client game module (client/src/tris.rs)
  - [ ] Update `start_game()` to accept `GameType` parameter
  - [ ] Send `ClientMessage::JoinMatchmaking { game_type }` with selected game type
  - [ ] Rest of flow remains same (wait for opponent, match found, etc.)

- [ ] Update server matchmaking logic (server/src/game_logic.rs)
  - [ ] Update `handle_join_matchmaking_logic()` to accept `game_type: GameType` parameter
  - [ ] Implement per-game-type matchmaking queues (e.g., separate waiting matches per game type)
  - [ ] Update `find_waiting_match()` in database to filter by game_type
  - [ ] Update `create_waiting_match()` to store game_type
  - [ ] Ensure matches are only paired if both players selected same game type

- [ ] Update server WebSocket handler (server/src/websocket.rs)
  - [ ] Extract `game_type` from `ClientMessage::JoinMatchmaking { game_type }`
  - [ ] Pass game_type to `handle_join_matchmaking()`

- [ ] Update database for game-type aware matchmaking
  - [ ] Ensure `matches` table has `game_type` column (already exists as String in DB)
  - [ ] Update `find_waiting_match()` to accept `GameType` parameter, serialize to string for query
  - [ ] Update query to filter: `WHERE game_type = ? AND player2_id IS NULL`
  - [ ] Update `create_waiting_match()` to accept `GameType` parameter and serialize to string
  - [ ] Update `MatchRecord::to_match()` to deserialize string from DB back to `GameType` enum

- [ ] Write tests for multi-game matchmaking
  - [ ] Test TicTacToe matchmaking (players selecting TicTacToe get matched)
  - [ ] Test RPS matchmaking (players selecting RPS get matched)
  - [ ] Test cross-game isolation (TicTacToe player doesn't match with RPS player)
  - [ ] Test menu choice parsing

- [ ] Validate end-to-end
  - [ ] Start two clients, both select TicTacToe → should match
  - [ ] Start two clients, both select RPS → should match (will fail game logic, expected)
  - [ ] Start two clients, one TicTacToe one RPS → should NOT match
  - [ ] Post-game returns to main menu correctly

### phase 4: Rock-Paper-Scissors Implementation
**Goal:** Implement complete Rock-Paper-Scissors game with best-of-3 format and simultaneous move submission.

#### Game Rules & Flow
- **Format**: Best of 3 rounds - first player to win 2 rounds wins the match
- **Move submission**: Both players submit moves independently; server waits for both before computing result
- **Early end**: If one player wins rounds 1 & 2, round 3 never starts
- **Input**: Players type 1 (rock), 2 (paper), or 3 (scissors)
- **Move locking**: Once submitted, move cannot be changed for that round
- **Disconnection handling**: Submitted moves are preserved; 10s reconnect timer applies; player may see next round result upon reconnect

#### Game State Structure
```rust
// RPS game state in JSON: list of rounds, each round is tuple of moves
// Example: [(Some("rock"), Some("scissors")), (Some("paper"), None)]
// - List length determines current round (implicit)
// - None means player hasn't submitted move yet
// - Initial state: [(None, None)]
// - Moves stored as strings: "rock", "paper", "scissors"
// - Tuple serializes using serde default (array format)
// - Player numbering: same as tic-tac-toe (player 1, player 2 as i32)

struct RPSGameState {
    rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>,  // (player1_move, player2_move)
}

enum RPSMove {
    Rock,      // serializes as "rock"
    Paper,     // serializes as "paper"
    Scissors,  // serializes as "scissors"
}

// Client message for RPS move submission
struct RPSMoveData {
    choice: RPSMove,  // The move choice (rock/paper/scissors)
}
```

#### Client UI States
1. **"First round choice"**: Initial prompt to select rock/paper/scissors
2. **"First round move sent, waiting for opponent"**: After submitting round 1 move
3. **"Previous round result and choice"**: Shows round N result + prompt for round N+1 move
4. **"Move for nth round sent, waiting for opponent"**: After submitting move for round 2+
5. **"Final result - press any key to return to menu"**: Match ended, show winner

#### UI Display Requirements
- **Score display**: Always show "Score: You X - Opponent Y" during active rounds
- **Round history**: Show results of completed rounds (e.g., "Round 1: Rock vs Scissors - You won")
- **Plain text output**: No animations, instant display of results

#### Client State Detection Logic
Client determines UI state by examining game state:
- Check if current round exists in list
- Check if my move for current round is Some or None
- If Some: show "waiting for opponent" state
- If None: show "choice" state with previous round results (if any)

#### Implementation Tasks

- [ ] Define RPS types in `games/rockpaperscissors.rs`
  - [ ] Create `RPSMove` enum (Rock, Paper, Scissors) with serde serialization
  - [ ] Create `RPSGameState` struct with `rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>`
  - [ ] Implement `RPSGameState::new()` to return initial state `[(None, None)]`
  - [ ] Implement helper methods:
    - `current_round() -> usize` (returns rounds.len())
    - `get_score() -> (u8, u8)` (count wins for each player)
    - `is_finished() -> bool` (either player has 2 wins)
    - `get_winner() -> Option<PlayerSymbol>` (who has 2 wins)
    - `compute_round_winner(p1_move: RPSMove, p2_move: RPSMove) -> Option<PlayerSymbol>`

- [ ] Implement RPS engine in `games/rockpaperscissors.rs`
  - [ ] Define `RPSEngine` struct (stateless)
  - [ ] Implement `RPSEngine::update(&self, state: &RPSGameState, player: PlayerSymbol, move_choice: RPSMove) -> Result<RPSGameState, GameError>`
    - Check game is not finished
    - Get current round tuple from state
    - Check player hasn't already submitted move for this round (their slot is None)
    - Set player's move in current round tuple
    - If both moves now present:
      - Compute round winner
      - Check if match is finished (someone has 2 wins)
      - If not finished: append new round `(None, None)` to list
    - Return new state

- [ ] Add RPS to game router (server/src/game_logic.rs or new dispatcher)
  - [ ] Update `handle_game_move()` to match on `GameType::RockPaperScissors`
  - [ ] Deserialize move_data to RPSMove
  - [ ] Call RPSEngine::update()
  - [ ] Serialize new state back to JSON

- [ ] Create RPS client UI module (client/src/rps.rs)
  - [ ] Implement `start_game()` (similar to tris::start_game)
  - [ ] Implement `resume_game()` for resumable matches
  - [ ] Implement `run_game_loop()` with RPS-specific rendering
  - [ ] Implement `display_rps_match()` to show:
    - Current score (wins per player)
    - History of completed rounds with moves and results
    - Current round prompt or waiting state
  - [ ] Implement `read_rps_input()` to accept 1/2/3 for rock/paper/scissors
  - [ ] Handle GameStateUpdate messages and determine UI state from game state

- [ ] Update client main menu flow (client/src/main.rs)
  - [ ] Route `MenuChoice::StartRPS` to `rps::start_game()`
  - [ ] Pass `GameType::RockPaperScissors` to game flow

- [ ] Write tests for RPS engine
  - [ ] Test valid move submission (first player, second player)
  - [ ] Test round winner computation (rock beats scissors, etc.)
  - [ ] Test round completion (both moves in → new round created)
  - [ ] Test match end conditions (2 wins → no new round)
  - [ ] Test duplicate move rejection (player tries to submit twice in same round)
  - [ ] Test game already finished rejection

- [ ] Client-side validation
  - [ ] Validate input is 1, 2, or 3 before sending to server
  - [ ] Re-prompt on invalid input (don't send to server)

- [ ] Integration testing
  - [ ] Play full RPS match between two clients
  - [ ] Test disconnection during round (move preserved)
  - [ ] Test reconnection (resume shows correct state)
  - [ ] Verify match outcome updates player scores (uses same system as TicTacToe - see database.rs:235)
  - [ ] Test early match end (2-0 score)

#### Scoring System (Automatic - No Changes Needed)
- RPS uses same `MatchOutcome` enum as TicTacToe: `Player1Win`, `Player2Win`, `Draw`
- These serialize to "p1_win", "p2_win", "draw" strings
- `update_player_scores_from_match()` is game-agnostic, reads outcome string:
  - "p1_win": player1 +3, player2 -1
  - "p2_win": player1 -1, player2 +3
  - "draw": both +1
- No RPS-specific scoring logic required

## Architecture Details

### Match Structure
- `Match` struct will hold:
  - `game: GameType` - enum indicating which game (TicTacToe, RockPaperScissors, etc.)
  - `state: serde_json::Value` - dynamic game state (type safety not a concern)
  - Existing fields (players, etc.)
- Since a player can only be in one game at a time, no conflicts

### Game Engine Lifecycle
- Same as current: server creates/destroys game instances per match
- Game engine receives match state and player actions
- Game engine updates state and returns server messages

### Client Changes
- Light refactoring required
- New game UIs are additive (not replacing existing tic-tac-toe UI)
- Client handles different game types via pattern matching on `game` field

### Matchmaking Details
- Queue visibility: Not shown to players (out of scope)
- Queue cancellation: Out of scope for this phase
- Post-match flow: Same as now (out of scope for changes)

## Implementation Details

### Message Flow
1. Client connects → Server presents main menu
2. Client selects game type → Added to that game's queue
3. Match found → Server creates Match with appropriate `GameType`
4. Game engine processes moves → Updates `state` JSON
5. Server broadcasts `GameStateUpdate` with dynamic state
6. Client renders based on `game` type

### Data Structures

#### GameType enum (in common crate)
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
enum GameType {
    #[serde(rename = "tris")]
    TicTacToe,
    #[serde(rename = "rps")]
    RockPaperScissors,
    // Future games...
}
```

#### Match struct changes (in common crate)
```rust
struct Match {
    game_type: GameType,         // Enum (not string)
    game_state: serde_json::Value,  // Game-specific state (was GameState)
    // ... existing fields (player1_id, player2_id, etc.)
}
```

#### Game Engine pattern (stateless, not a trait - just a pattern)
```rust
// Engines are stateless structs/units that transform state immutably
// Example for TicTacToe:

struct TicTacToeEngine;

impl TicTacToeEngine {
    // Pure function: takes old state, returns new state
    // Does not mutate anything, no side effects
    fn update(&self,
              state: &TicTacToeGameState,
              player: PlayerSymbol,
              game_move: &TicTacToeMove)
              -> Result<TicTacToeGameState, GameError> {
        // Validate, compute new state, return it
        // Old state remains unchanged
    }
}

// No shared trait needed - each game has its own engine with its own types
// Server dispatcher pattern-matches on GameType to call the right engine
```

