# Battld Multi-Game Hub Migration Plan

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

### Phase 1: Infrastructure
- [ ] Design game engine trait/interface
- [ ] Refactor message types to support dynamic game state
- [ ] Create games module structure
- [ ] Update server to route messages to appropriate game engine

### Phase 2: Refactor Tic-Tac-Toe
- [ ] Extract tic-tac-toe logic into `games/tictactoe` module
- [ ] Implement game engine interface for tic-tac-toe
- [ ] Update tests

### Phase 3: Main Menu & Matchmaking
- [ ] Design main menu UI (game selection)
- [ ] Implement per-game matchmaking queues
- [ ] Update connection flow to support game selection

### Phase 4: Rock-Paper-Scissors
- [ ] Implement RPS game engine
- [ ] Create RPS game state and logic
- [ ] Add RPS to game selection menu

### Phase 5: Testing & Polish
- [ ] Test both games end-to-end
- [ ] Update documentation
- [ ] Performance testing with multiple concurrent games

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

#### GameType enum
```rust
enum GameType {
    TicTacToe,
    RockPaperScissors,
    // Future games...
}
```

#### Match struct changes
```rust
struct Match {
    game: GameType,
    state: serde_json::Value,  // Game-specific state
    // ... existing fields
}
```

#### Game Engine trait (rough sketch)
```rust
trait GameEngine {
    fn handle_move(&mut self, player_id: String, move_data: Value) -> Result<Vec<ServerMessage>>;
    fn get_state(&self) -> Value;
    fn is_finished(&self) -> bool;
    fn get_winner(&self) -> Option<String>;
}
```

