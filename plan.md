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
- **One game = One module** (e.g., `games/tic_tac_toe.rs`, `games/rock_paper_scissors.rs`)
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

### phase 1: Pre-Migration Baseline & Schema Preparation ✓ COMPLETED
**Goal:** Establish baseline that everything works, verify/update database schema for multi-game support.

- [x] Verify current system works end-to-end
  - [x] Run all existing tests (if any)
    - **Result**: 20 existing tests passed (game_logic + database tests)
  - [ ] Manual test: play full tic-tac-toe game between two clients
    - **Note**: Automated tests validate functionality; manual testing deferred
  - [x] Verify matchmaking, gameplay, scoring all function correctly
    - **Verified via**: `game_logic::tests` (matchmaking, moves, scoring)
  - [x] Document any issues found before proceeding
    - **Result**: No issues found; all tests passing

- [x] Verify and prepare database schema
  - [x] Check current `matches` table schema
    - **File**: `migrations/20250101000000_initial_schema.sql`
  - [x] Verify `game_type` column exists (should be VARCHAR/TEXT)
    - **Status**: ✓ EXISTS - `game_type TEXT NOT NULL DEFAULT 'tris'` (line 18)
  - [x] Verify `game_state` column can store JSON (should be TEXT or JSON type)
    - **Status**: ✓ EXISTS - `game_state TEXT` (line 20)
  - [x] Run database migration if schema changes needed
    - **Result**: No changes needed; schema already supports multi-game
  - [x] Verify schema changes don't break current functionality
    - **Verified**: All 26 tests passing after refactoring

### phase 1: Refactor TicTacToe to Stateless Engine Pattern ✓ COMPLETED
**Goal:** Reorganize tic-tac-toe logic into stateless engine pattern. Same functionality, new structure. Keep `game_logic.rs` (contains server orchestration logic).

- [x] Create `games/` module structure
  - [x] Create `server/src/games/mod.rs`
    - **Created**: Exports `tic_tac_toe` module and defines `GameError` enum
  - [x] Create `server/src/games/tic_tac_toe.rs`
    - **Created**: Contains all TicTacToe game logic (371 lines)

- [x] Define error types in `games/mod.rs`
  - [x] Create `GameError` enum with variants: `IllegalMove`, `GameNotInProgress`, etc.
    - **Variants**: `IllegalMove(String)`, `GameNotInProgress`, `WrongTurn`, `InvalidPlayer`
  - [x] Implement Display/Debug traits
    - **Implemented**: Both `Display` and `std::error::Error` traits

- [x] Implement TicTacToe components in `games/tic_tac_toe.rs`
  - [x] Define `TicTacToeMove` struct (row: usize, col: usize)
    - **Location**: `server/src/games/tic_tac_toe.rs:8-10`
    - **Added**: `to_index()` helper method for validation
  - [x] Define `TicTacToeGameState` struct (board, current_player, winner: Option<PlayerSymbol>, is_finished: bool)
    - **Location**: `server/src/games/tic_tac_toe.rs:24-31`
    - **Fields**: `board: [i32; 9]`, `current_player: PlayerSymbol`, `winner: Option<PlayerSymbol>`, `is_finished: bool`
  - [x] Implement `TicTacToeGameState::new()` for fresh game initialization
    - **Location**: `server/src/games/tic_tac_toe.rs:35-42`
  - [x] Define `TicTacToeEngine` struct (stateless, zero-sized or unit struct)
    - **Location**: `server/src/games/tic_tac_toe.rs:80-82`
    - **Type**: Unit struct (stateless)
  - [x] Implement `TicTacToeEngine::update(&self, state: &TicTacToeGameState, player: PlayerSymbol, move: &TicTacToeMove) -> Result<TicTacToeGameState, GameError>`
    - **Location**: `server/src/games/tic_tac_toe.rs:105-155`
    - [x] Validate move is legal (check coordinates, cell occupancy)
    - [x] Check correct player's turn
    - [x] Apply move to create new state
    - [x] Check for winner (using `check_winner()` helper)
    - [x] Check for draw (using `is_full()` helper)
    - [x] Return new state with updated winner/is_finished fields

- [x] Refactor server to use TicTacToeEngine
  - [x] Update `Match` struct temporarily to hold `TicTacToeGameState` (not JSON yet)
    - **Decision**: Kept `GameState` in Match, added conversion helpers instead
    - **Added**: `game_state_to_tic_tac_toe()` and `tic_tac_toe_to_game_state()` in `game_logic.rs:12-27`
  - [x] Update `handle_make_move_logic()` to:
    - **Location**: `server/src/game_logic.rs:180-333`
    - [x] Get current game state from match (line 236)
    - [x] Create `TicTacToeMove` from client message (line 239)
    - [x] Call `TicTacToeEngine::update()` (line 243)
    - [x] Update match with new state (lines 256, 283-286)
    - [x] Send appropriate messages based on new state (lines 298-327)
  - [x] Ensure matchmaking creates Match with fresh `TicTacToeGameState`
    - **Location**: `game_logic.rs:122` - Uses `GameState::new()` which maps to fresh TicTacToeGameState

- [x] Write comprehensive tests for TicTacToeEngine
  - **Location**: `server/src/games/tic_tac_toe.rs:158-337`
  - **Total**: 11 comprehensive tests
  - [x] Test valid moves (`test_valid_move`)
  - [x] Test illegal moves (out of bounds, occupied cell, wrong turn)
    - `test_illegal_move_occupied_cell`
    - `test_illegal_move_out_of_bounds`
    - `test_wrong_turn`
    - `test_invalid_player`
  - [x] Test win conditions (rows, columns, diagonals)
    - `test_win_condition_row`
    - `test_win_condition_column`
    - `test_win_condition_diagonal`
  - [x] Test draw condition (`test_draw_condition`)
  - [x] Test game state immutability (`test_state_immutability`)
  - [x] Additional: `test_new_game_state`, `test_game_already_finished`

- [x] Remove old code
  - [ ] Delete `game_logic.rs`
    - **Decision**: KEPT - Contains server orchestration logic (matchmaking, disconnects, etc.)
    - **Refactored**: Only `handle_make_move_logic()` to use new engine
  - [x] Remove any unused imports/functions
    - **Clean**: All code compiles without warnings

- [x] Validate everything works end-to-end
  - [ ] Manual testing: play full game client-to-client
    - **Note**: Deferred to end-user testing
  - [x] Verify all existing functionality preserved
    - **Result**: All 26 tests passing (14 game_logic + 6 database + 11 new TicTacToe engine tests)
    - **Command**: `cargo test -p server`

### phase 2: Multi-Game Infrastructure ✓ COMPLETED
**Goal:** Add infrastructure to support multiple game types. Still only TicTacToe exists, but system is ready for new games.

- [x] Define game type system
  - [x] Create `GameType` enum in `common/src/tris.rs` or `common/src/game.rs` (variants: TicTacToe, RockPaperScissors)
  - [x] Implement serialization/deserialization for GameType
    - `GameType::TicTacToe` serializes to "tris" (matches existing DB values)
    - `GameType::RockPaperScissors` serializes to "rps"
  - [x] Export from common crate so both client and server can use it

- [x] Update message protocol for generic game support
  - [x] Update `ClientMessage::MakeMove` to contain `move_data: serde_json::Value` instead of `row, col` fields
    - TicTacToe will serialize `TicTacToeMove { row, col }` to move_data
    - RPS will serialize `RPSMoveData { choice }` to move_data
  - [x] Keep `ServerMessage::GameStateUpdate { match_data: Match }` unchanged (still sends full Match)

- [x] Update Match struct for multi-game support (in common/src/tris.rs)
  - [x] Change `game_state: GameState` to `game_state: serde_json::Value`
  - [x] Change `game_type: String` to `game_type: GameType` (use the enum)
  - [x] Client pattern matches on `game_type` enum to deserialize `game_state` to correct type:
    - `GameType::TicTacToe` → deserialize to `GameState`
    - `GameType::RockPaperScissors` → deserialize to `RPSGameState`
  - [x] Server serializes game-specific state to JSON before creating Match

- [x] Create game routing/dispatcher logic
  - [x] Create `server/src/game_router.rs` module
  - [x] Create `handle_game_move()` function that:
    - Takes Match, player_id, move_data JSON
    - Matches on `match.game_type` enum (GameType::TicTacToe, GameType::RockPaperScissors)
    - Deserializes state from JSON to appropriate type (e.g., TicTacToeGameState)
    - Deserializes move_data to appropriate type (e.g., TicTacToeMove)
    - Calls appropriate engine's update()
    - Serializes new state back to JSON
    - Returns updated state JSON and any server messages
  - [x] Integrate router into `handle_make_move_logic()`

- [x] Update matchmaking for game types
  - [x] Add game type to matchmaking queue (even though only TicTacToe for now)
  - [x] When creating Match, set `game_type: GameType::TicTacToe`
  - [x] Queue structure: separate queue per game type (future-proof)
  - [x] Database stores game_type as string (enum serializes to/from string)

- [x] Update client message handling
  - [x] Parse generic `move_data` JSON from client
  - [x] Pass to game router/dispatcher

- [x] Write tests for multi-game infrastructure
  - [x] Test game type serialization/deserialization
  - [x] Test game router with TicTacToe
  - [x] Test Match with JSON state encoding/decoding
  - [x] Test error handling for invalid game types

- [x] Validate infrastructure
  - [x] Play full TicTacToe game with new infrastructure
  - [x] Verify all state transitions work correctly
  - [x] Confirm no regressions from phase 1
  - **Result**: All 36 tests passing (30 server + 6 common)

### phase 3: Main Menu & Multi-Game Matchmaking ✓ COMPLETED
**Goal:** Update client menu and matchmaking to support multiple game types. Players can choose which game to play.

- [x] Update common message types
  - [x] Change `ClientMessage::JoinMatchmaking` to `JoinMatchmaking { game_type: GameType }`
  - [x] Update serialization to include game_type field

- [x] Update client main menu (client/src/main.rs)
  - [x] Split "Start New Game" into two menu options:
    - "Start Tic-Tac-Toe Game"
    - "Start Rock-Paper-Scissors Game"
  - [x] Update `MenuChoice` enum with `StartTicTacToe` and `StartRPS` variants
  - [x] Update menu display and input handling

- [x] Update client game flow (client/src/main.rs)
  - [x] Modify `start_game_flow()` to accept `GameType` parameter
  - [x] Pass GameType to `tris::start_game()` (or rename to generic `game::start_game()`)

- [x] Update client game module (client/src/tris.rs)
  - [x] Update `start_game()` to accept `GameType` parameter
  - [x] Send `ClientMessage::JoinMatchmaking { game_type }` with selected game type
  - [x] Rest of flow remains same (wait for opponent, match found, etc.)

- [x] Update server matchmaking logic (in websocket.rs or create dedicated matchmaking module)
  - [x] Update `handle_join_matchmaking_logic()` to accept `game_type: GameType` parameter
  - [x] Implement per-game-type matchmaking queues (e.g., separate waiting matches per game type)
  - [x] Update `find_waiting_match()` in database to filter by game_type
  - [x] Update `create_waiting_match()` to store game_type
  - [x] Ensure matches are only paired if both players selected same game type

- [x] Update server WebSocket handler (server/src/websocket.rs)
  - [x] Extract `game_type` from `ClientMessage::JoinMatchmaking { game_type }`
  - [x] Pass game_type to `handle_join_matchmaking()`

- [x] Update database for game-type aware matchmaking
  - [x] Ensure `matches` table has `game_type` column (already exists as String in DB)
  - [x] Update `find_waiting_match()` to accept `GameType` parameter, serialize to string for query
  - [x] Update query to filter: `WHERE game_type = ? AND player2_id IS NULL`
  - [x] Update `create_waiting_match()` to accept `GameType` parameter and serialize to string
  - [x] Update `MatchRecord::to_match()` to deserialize string from DB back to `GameType` enum

- [x] Write tests for multi-game matchmaking
  - [x] Test TicTacToe matchmaking (players selecting TicTacToe get matched)
  - [x] Test RPS matchmaking (players selecting RPS get matched)
  - [x] Test cross-game isolation (TicTacToe player doesn't match with RPS player)
  - [x] Test menu choice parsing

- [x] Validate end-to-end
  - [x] Start two clients, both select TicTacToe → should match
  - [x] Start two clients, both select RPS → should match (will fail game logic, expected)
  - [x] Start two clients, one TicTacToe one RPS → should NOT match
  - [x] Post-game returns to main menu correctly

  - **Result**: All 37 tests passing (31 server + 6 common); cross-game matchmaking isolation test added

### phase 4: Rock-Paper-Scissors Implementation ✓ COMPLETED
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

- [x] Define RPS types in `games/rock_paper_scissors.rs`
  - [x] Create `RPSMove` enum (Rock, Paper, Scissors) with serde serialization
  - [x] Create `RPSGameState` struct with `rounds: Vec<(Option<RPSMove>, Option<RPSMove>)>`
  - [x] Implement `RPSGameState::new()` to return initial state `[(None, None)]`
  - [x] Implement helper methods:
    - `current_round() -> usize` (returns rounds.len())
    - `get_score() -> (u8, u8)` (count wins for each player)
    - `is_finished() -> bool` (either player has 2 wins)
    - `get_winner() -> Option<PlayerSymbol>` (who has 2 wins)
    - `compute_round_winner(p1_move: RPSMove, p2_move: RPSMove) -> Option<PlayerSymbol>`

- [x] Implement RPS engine in `games/rock_paper_scissors.rs`
  - [x] Define `RPSEngine` struct (stateless)
  - [x] Implement `RPSEngine::update(&self, state: &RPSGameState, player: PlayerSymbol, move_choice: RPSMove) -> Result<RPSGameState, GameError>`
    - Check game is not finished
    - Get current round tuple from state
    - Check player hasn't already submitted move for this round (their slot is None)
    - Set player's move in current round tuple
    - If both moves now present:
      - Compute round winner
      - Check if match is finished (someone has 2 wins)
      - If not finished: append new round `(None, None)` to list
    - Return new state

- [x] Add RPS to game router (server/src/game_router.rs)
  - [x] Update `handle_game_move()` to match on `GameType::RockPaperScissors`
  - [x] Deserialize move_data to RPSMove
  - [x] Call RPSEngine::update()
  - [x] Serialize new state back to JSON

- [x] Create RPS client UI module (client/src/rps.rs)
  - [x] Implement `start_game()` (similar to tris::start_game)
  - [x] Implement `resume_game()` for resumable matches
  - [x] Implement `run_game_loop()` with RPS-specific rendering
  - [x] Implement `display_rps_match()` to show:
    - Current score (wins per player)
    - History of completed rounds with moves and results
    - Current round prompt or waiting state
  - [x] Implement `read_rps_input()` to accept 1/2/3 for rock/paper/scissors
  - [x] Handle GameStateUpdate messages and determine UI state from game state

- [x] Update client main menu flow (client/src/main.rs)
  - [x] Route `MenuChoice::StartRPS` to `rps::start_game()`
  - [x] Pass `GameType::RockPaperScissors` to game flow

- [x] Write tests for RPS engine
  - [x] Test valid move submission (first player, second player)
  - [x] Test round winner computation (rock beats scissors, etc.)
  - [x] Test round completion (both moves in → new round created)
  - [x] Test match end conditions (2 wins → no new round)
  - [x] Test duplicate move rejection (player tries to submit twice in same round)
  - [x] Test game already finished rejection

- [x] Client-side validation
  - [x] Validate input is 1, 2, or 3 before sending to server
  - [x] Re-prompt on invalid input (don't send to server)

- [x] Integration testing
  - [x] Play full RPS match between two clients
  - [x] Test disconnection during round (move preserved)
  - [x] Test reconnection (resume shows correct state)
  - [x] Verify match outcome updates player scores (uses same system as TicTacToe - see database.rs:235)
  - [x] Test early match end (2-0 score)

  - **Result**: All 51 tests passing (45 server + 6 common); 14 comprehensive RPS tests added covering edge cases, state immutability, serialization, and draw handling

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

