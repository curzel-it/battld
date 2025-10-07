# Rock-Paper-Scissors Game Trace

Complete trace of a RPS game from start to finish with all function calls, WebSocket messages, and state transitions.

**Scenario:**
- Player 1 (ID: 100) vs Player 2 (ID: 200)
- Round 1: P1=Rock, P2=Scissors → P1 wins
- Round 2: P1=Rock, P2=Paper → P2 wins
- Round 3: P1=Scissors, P2=Paper → P1 wins match (2-1)

---

## Phase 1: Authentication & Connection

### Player1:
```
┌─ client/src/auth.rs::try_auto_login()
│  • Loads auth token from config
│  • Token format: "100:signature_base64"
│
└─ client/src/state.rs::SessionState::connect_websocket()
   │
   └─ client/src/websocket.rs::WebSocketClient::connect(ws_url, token)
      • Opens WebSocket connection to server
      • Spawns 3 async tasks:
        1. Send task: forwards ClientMessages to server
        2. Receive task: receives ServerMessages from server
        3. Keepalive task: sends Ping every 30s
      │
      └─ SENDS WebSocket message:
         {
           "type": "authenticate",
           "token": "100:signature_base64"
         }
```

### Server:
```
┌─ server/src/websocket.rs::ws_handler()
│  • Accepts WebSocket upgrade
│
└─ server/src/websocket.rs::handle_socket()
   • Spawns send task for outgoing messages
   • Enters message loop
   │
   ├─ RECEIVES WebSocket message from Player1:
   │  {
   │    "type": "authenticate",
   │    "token": "100:signature_base64"
   │  }
   │  [WS RECV] Authenticate { token: "100:signature_base64" }
   │
   ├─ server/src/websocket.rs::authenticate_token(db, "100:signature_base64")
   │  • Parses token: player_id=100, signature
   │  • Loads player from database
   │  • Verifies RSA signature
   │  • Returns: Ok(100)
   │
   ├─ ConnectionRegistry::register(100, tx, abort_handle)
   │  • Registers player 100's connection
   │  • Stores message sender channel
   │  Console: "Registered WebSocket connection for player 100"
   │
   ├─ ConnectionRegistry::get_resumable_match(100)
   │  • Checks for pending resumable match
   │  • Returns: None
   │
   └─ SENDS WebSocket message to Player1:
      {
        "type": "auth_success",
        "player_id": 100
      }
      [WS SEND] AuthSuccess { player_id: 100 }
      Console: "Player 100 authenticated via WebSocket"
```

### Player1:
```
└─ client/src/websocket.rs (receive task)
   │
   ├─ RECEIVES WebSocket message:
   │  {
   │    "type": "auth_success",
   │    "player_id": 100
   │  }
   │  [RECV] AuthSuccess { player_id: 100 }
   │  File: client.log appended
   │
   └─ Stores message in server_messages buffer
```

### Player2:
```
• Player 2 connects and authenticates identically
• Player ID: 200
• Same flow: connect → authenticate → register → auth_success
```

---

## Phase 2: Joining Matchmaking

### Player1:
```
┌─ client/src/main.rs::main()
│  • User selects menu option "2" (Start RPS)
│
├─ client/src/main.rs::read_menu_choice()
│  • Returns: MenuChoice::StartRPS
│
├─ client/src/main.rs::start_game_flow(session, GameType::RockPaperScissors)
│  • Prints: "Starting Rock-Paper-Scissors matchmaking..."
│  │
│  └─ client/src/rps.rs::start_game(session, GameType::RockPaperScissors)
│     │
│     └─ WebSocketClient::send(ClientMessage::JoinMatchmaking)
│        │
│        └─ SENDS WebSocket message:
           {
             "type": "join_matchmaking",
             "game_type": "rps"
           }
           [SEND] JoinMatchmaking { game_type: RockPaperScissors }
           File: client.log appended
```

### Server:
```
└─ server/src/websocket.rs::handle_socket() (Player1's connection)
   │
   ├─ RECEIVES WebSocket message:
   │  {
   │    "type": "join_matchmaking",
   │    "game_type": "rps"
   │  }
   │  [WS RECV] JoinMatchmaking { game_type: RockPaperScissors }
   │
   └─ server/src/websocket.rs::handle_join_matchmaking(100, GameType::RockPaperScissors, db, registry)
      │
      └─ server/src/game_logic.rs::handle_join_matchmaking_logic(100, GameType::RockPaperScissors, db)
         │
         ├─ Database::get_active_match_for_player(100)
         │  • Queries: SELECT * FROM matches WHERE (player1_id = 100 OR player2_id = 100) AND in_progress = 1
         │  • Returns: None (no active match)
         │
         ├─ game_type.to_string() → "rps"
         │
         ├─ Database::find_waiting_match(100, "rps")
         │  • Queries: SELECT * FROM matches WHERE game_type = 'rps' AND player2_id IS NULL AND player1_id != 100
         │  • Returns: None (no waiting opponent)
         │
         ├─ Database::create_waiting_match(100, "rps")
         │  • Inserts: INSERT INTO matches (player1_id, game_type, in_progress) VALUES (100, 'rps', 1)
         │  • Returns: Ok(match_id=1)
         │  Console: "Player 100 created waiting match for game type: rps"
         │
         └─ Returns: [OutgoingMessage { player_id: 100, message: WaitingForOpponent }]
```

### Server (continued):
```
   └─ ConnectionRegistry::send_messages([...])
      │
      └─ SENDS WebSocket message to Player1:
         {
           "type": "waiting_for_opponent"
         }
         [WS SEND] WaitingForOpponent
```

### Player1:
```
└─ client/src/rps.rs::start_game() (waiting in loop)
   │
   ├─ WebSocketClient::get_messages()
   │  • Returns: [WaitingForOpponent]
   │
   ├─ Match on WaitingForOpponent:
   │  • Continue waiting (no action)
   │
   └─ tokio::time::sleep(200ms) → loop continues
```

### Player2:
```
• Player 2 joins matchmaking with GameType::RockPaperScissors
• Same flow as Player 1 initially...

┌─ client/src/rps.rs::start_game(session, GameType::RockPaperScissors)
│  │
│  └─ SENDS WebSocket message:
     {
       "type": "join_matchmaking",
       "game_type": "rps"
     }
     [SEND] JoinMatchmaking { game_type: RockPaperScissors }
```

### Server:
```
└─ server/src/websocket.rs::handle_socket() (Player2's connection)
   │
   ├─ RECEIVES WebSocket message:
   │  {
   │    "type": "join_matchmaking",
   │    "game_type": "rps"
   │  }
   │  [WS RECV] JoinMatchmaking { game_type: RockPaperScissors }
   │
   └─ server/src/game_logic.rs::handle_join_matchmaking_logic(200, GameType::RockPaperScissors, db)
      │
      ├─ Database::get_active_match_for_player(200)
      │  • Returns: None
      │
      ├─ Database::find_waiting_match(200, "rps")
      │  • Queries: SELECT * FROM matches WHERE game_type = 'rps' AND player2_id IS NULL AND player1_id != 200
      │  • Returns: Some(MatchRecord { id: 1, player1_id: 100, player2_id: None, game_type: "rps", ... })
      │  • MATCH FOUND!
      │
      ├─ rand::thread_rng().gen_bool(0.5)
      │  • Randomizes who goes first
      │  • Result: first_player = 1
      │
      ├─ Initialize game state for GameType::RockPaperScissors:
      │  • server/src/games/rock_paper_scissors.rs::RPSGameState::new()
      │  • Returns: RPSGameState { rounds: [(None, None)] }
      │  • Serializes to JSON: {"rounds":[[null,null]]}
      │
      ├─ Database::join_waiting_match(match_id=1, player2_id=200, first_player=1, game_state_json)
      │  • Updates: UPDATE matches SET player2_id = 200, current_player = 1, game_state = '{"rounds":[[null,null]]}' WHERE id = 1
      │  • Returns: Ok(())
      │  Console: "Matching player 200 with waiting player 100 for game type: rps"
      │
      ├─ Database::get_match_by_id(1)
      │  • Queries: SELECT * FROM matches WHERE id = 1
      │  • Returns: MatchRecord with both players
      │
      ├─ MatchRecord::to_match()
      │  • Deserializes game_state JSON to serde_json::Value
      │  • Parses game_type "rps" → GameType::RockPaperScissors
      │  • Creates Match struct:
      │    Match {
      │      id: 1,
      │      player1_id: 100,
      │      player2_id: 200,
      │      in_progress: true,
      │      outcome: None,
      │      game_type: RockPaperScissors,
      │      current_player: 1,
      │      game_state: {"rounds":[[null,null]]}
      │    }
      │
      └─ Returns: [
           OutgoingMessage {
             player_id: 100,
             message: MatchFound { match_data: <Match> }
           },
           OutgoingMessage {
             player_id: 200,
             message: MatchFound { match_data: <Match> }
           }
         ]
```

### Server (continued):
```
   └─ ConnectionRegistry::send_messages([...])
      │
      ├─ SENDS WebSocket message to Player1:
      │  {
      │    "type": "match_found",
      │    "match_data": {
      │      "id": 1,
      │      "player1_id": 100,
      │      "player2_id": 200,
      │      "in_progress": true,
      │      "outcome": null,
      │      "game_type": "rps",
      │      "current_player": 1,
      │      "game_state": {"rounds":[[null,null]]}
      │    }
      │  }
      │  [WS SEND] MatchFound { match_data: ... }
      │
      └─ SENDS WebSocket message to Player2:
         (same message as Player1)
         [WS SEND] MatchFound { match_data: ... }
```

### Player1:
```
└─ client/src/rps.rs::start_game() (waiting loop)
   │
   ├─ WebSocketClient::get_messages()
   │  • Returns: [MatchFound { match_data }]
   │  [RECV] MatchFound { match_data: ... }
   │  File: client.log appended
   │
   ├─ Match on MatchFound:
   │  • Breaks out of loop
   │  • game_match = match_data
   │  • Prints: "Match found!"
   │
   └─ client/src/rps.rs::run_game_loop(session, game_match)
```

### Player2:
```
└─ client/src/rps.rs::start_game() (waiting loop)
   • Same as Player1
   • Receives MatchFound
   • Enters run_game_loop(session, game_match)
```

---

## Phase 3: Round 1 - Player 1 makes move

### Player1:
```
┌─ client/src/rps.rs::run_game_loop(session, game_match)
│  • my_player_id = 100
│  • my_number = 1 (since player1_id == 100)
│
├─ Loop iteration 1:
│  │
│  ├─ client/src/ui.rs::clear_screen()
│  │
│  ├─ client/src/rps.rs::display_rps_match(game_match, my_number=1)
│  │  │
│  │  ├─ get_rps_state(game_match)
│  │  │  • Deserializes game_state JSON to RPSGameState
│  │  │  • Returns: RPSGameState { rounds: [(None, None)] }
│  │  │
│  │  ├─ state.get_score()
│  │  │  • Iterates through rounds
│  │  │  • No completed rounds yet
│  │  │  • Returns: (0, 0)
│  │  │
│  │  └─ Prints:
│  │     ═══════════════════════════════════════
│  │         ROCK  ·  PAPER  ·  SCISSORS
│  │     ═══════════════════════════════════════
│  │
│  │     Score: You 0 - 0 Opponent (First to 2 wins)
│  │
│  ├─ Check if game is over:
│  │  • game_match.in_progress == true → continue
│  │
│  ├─ Determine if my move is submitted:
│  │  • current_round_idx = 0
│  │  • current_round = (None, None)
│  │  • my_number = 1 → check position 0
│  │  • my_move_submitted = false
│  │
│  ├─ Need to make a move:
│  │  • Prints:
│  │    Your turn! Choose your move:
│  │      1. Rock
│  │      2. Paper
│  │      3. Scissors
│  │
│  │    Enter choice (1-3): _
│  │
│  ├─ client/src/rps.rs::read_rps_input()
│  │  • Uses rustyline to read input
│  │  • User types: 1
│  │  • Validates: 1 is valid
│  │  • Returns: "rock"
│  │
│  ├─ Create move_data JSON:
│  │  • move_data = {"choice": "rock"}
│  │
│  ├─ WebSocketClient::send(ClientMessage::MakeMove { move_data })
│  │  │
│  │  └─ SENDS WebSocket message:
│  │     {
│  │       "type": "make_move",
│  │       "move_data": {"choice": "rock"}
│  │     }
│  │     [SEND] MakeMove { move_data: {"choice":"rock"} }
│  │     File: client.log appended
│  │
│  └─ Prints: "Move submitted! Waiting for opponent..."
```

### Server:
```
└─ server/src/websocket.rs::handle_socket() (Player1's connection)
   │
   ├─ RECEIVES WebSocket message:
   │  {
   │    "type": "make_move",
   │    "move_data": {"choice": "rock"}
   │  }
   │  [WS RECV] MakeMove { move_data: Object {"choice": String("rock")} }
   │
   └─ server/src/websocket.rs::handle_make_move(100, move_data, db, registry)
      │
      └─ server/src/game_logic.rs::handle_make_move_logic(100, {"choice":"rock"}, db)
         │
         ├─ Database::get_active_match_for_player(100)
         │  • Queries: SELECT * FROM matches WHERE (player1_id = 100 OR player2_id = 100) AND in_progress = 1
         │  • Returns: Some(MatchRecord { id: 1, ... })
         │
         ├─ MatchRecord::to_match()
         │  • Returns: Match {
         │      id: 1,
         │      player1_id: 100,
         │      player2_id: 200,
         │      game_type: RockPaperScissors,
         │      game_state: {"rounds":[[null,null]]},
         │      in_progress: true,
         │      current_player: 1,
         │      outcome: None
         │    }
         │
         ├─ Verify match is in progress:
         │  • in_progress == true ✓
         │
         ├─ server/src/game_router.rs::handle_game_move(game_match, player_id=100, move_data)
         │  │
         │  ├─ Match on game_type:
         │  │  • GameType::RockPaperScissors → handle_rps_move()
         │  │
         │  └─ server/src/game_router.rs::handle_rps_move(game_match, 100, move_data)
         │     │
         │     ├─ Deserialize current game state:
         │     │  • serde_json::from_value::<RPSGameState>(game_match.game_state)
         │     │  • Returns: RPSGameState { rounds: [(None, None)] }
         │     │
         │     ├─ Deserialize move data:
         │     │  • #[derive(Deserialize)] struct RPSMoveData { choice: RPSMove }
         │     │  • serde_json::from_value::<RPSMoveData>({"choice":"rock"})
         │     │  • Returns: RPSMoveData { choice: Rock }
         │     │
         │     ├─ Determine player symbol:
         │     │  • player_id (100) == player1_id (100) → player_symbol = 1
         │     │
         │     ├─ Call RPS engine:
         │     │  │
         │     │  └─ server/src/games/rock_paper_scissors.rs::RPSEngine::update(state, player=1, move_choice=Rock)
         │     │     │
         │     │     ├─ Check game not finished:
         │     │     │  • state.is_finished() == false ✓
         │     │     │
         │     │     ├─ Get current round:
         │     │     │  • current_round_idx = 0
         │     │     │  • current_round = (None, None)
         │     │     │
         │     │     ├─ Check player hasn't already moved:
         │     │     │  • player == 1 → check position 0
         │     │     │  • current_round.0 == None ✓
         │     │     │
         │     │     ├─ Create new state:
         │     │     │  • new_state = state.clone()
         │     │     │  • new_round = (Some(Rock), None)
         │     │     │  • new_state.rounds[0] = new_round
         │     │     │
         │     │     ├─ Check if both players moved:
         │     │     │  • new_round = (Some(Rock), None)
         │     │     │  • Only player 1 moved → don't add new round yet
         │     │     │
         │     │     └─ Returns: Ok(RPSGameState { rounds: [(Some(Rock), None)] })
         │     │
         │     ├─ Serialize new state to JSON:
         │     │  • new_state_json = {"rounds":[[{"Rock":null},null]]}
         │     │  • Note: RPSMove serializes as lowercase due to #[serde(rename_all = "lowercase")]
         │     │  • Actual JSON: {"rounds":[["rock",null]]}
         │     │
         │     ├─ Check if game is finished:
         │     │  • new_state.is_finished() → false
         │     │  • outcome = None
         │     │
         │     └─ Returns: Ok(GameMoveResult {
         │          new_state: {"rounds":[["rock",null]]},
         │          is_finished: false,
         │          outcome: None
         │        })
         │
         ├─ Back in handle_make_move_logic:
         │  │
         │  ├─ Determine next player:
         │  │  • current_player = 1 → next_player = 2
         │  │
         │  ├─ Update match in database:
         │  │  • Database::update_match(
         │  │      match_id=1,
         │  │      current_player=2,
         │  │      game_state='{"rounds":[["rock",null]]}',
         │  │      in_progress=true,
         │  │      outcome=None
         │  │    )
         │  │  • Updates: UPDATE matches SET current_player = 2, game_state = '{"rounds":[["rock",null]]}', in_progress = 1, outcome = NULL WHERE id = 1
         │  │  Console: "Player 100 made move. Match 1: in_progress=true, outcome=None"
         │  │
         │  ├─ Update Match struct:
         │  │  • game_match.game_state = {"rounds":[["rock",null]]}
         │  │  • game_match.current_player = 2
         │  │  • game_match.in_progress = true
         │  │  • game_match.outcome = None
         │  │
         │  └─ Returns: [
         │       OutgoingMessage {
         │         player_id: 100,
         │         message: GameStateUpdate { match_data: <updated_match> }
         │       },
         │       OutgoingMessage {
         │         player_id: 200,
         │         message: GameStateUpdate { match_data: <updated_match> }
         │       }
         │     ]
         │
         └─ ConnectionRegistry::send_messages([...])
            │
            ├─ SENDS WebSocket message to Player1:
            │  {
            │    "type": "game_state_update",
            │    "match_data": {
            │      "id": 1,
            │      "player1_id": 100,
            │      "player2_id": 200,
            │      "in_progress": true,
            │      "outcome": null,
            │      "game_type": "rps",
            │      "current_player": 2,
            │      "game_state": {"rounds":[["rock",null]]}
            │    }
            │  }
            │  [WS SEND] GameStateUpdate { match_data: ... }
            │
            └─ SENDS WebSocket message to Player2:
               (same message)
               [WS SEND] GameStateUpdate { match_data: ... }
```

### Player1:
```
└─ client/src/rps.rs::run_game_loop() (waiting for update)
   │
   ├─ client/src/rps.rs::wait_for_game_update(ws_client)
   │  │
   │  ├─ Loop waiting for GameStateUpdate or error:
   │  │  │
   │  │  ├─ WebSocketClient::get_messages()
   │  │  │  • Returns: [GameStateUpdate { match_data }]
   │  │  │  [RECV] GameStateUpdate { match_data: ... }
   │  │  │  File: client.log appended
   │  │  │
   │  │  └─ Match on GameStateUpdate:
   │  │     • Returns: (match_data, None)
   │  │
   │  └─ game_match = new match_data
   │     • game_state now: {"rounds":[["rock",null]]}
   │
   └─ Loop continues to next iteration
```

### Player2:
```
└─ client/src/rps.rs::run_game_loop() (still waiting after MatchFound)
   │
   ├─ client/src/rps.rs::wait_for_game_update(ws_client)
   │  • Receives GameStateUpdate
   │  • game_match updated: {"rounds":[["rock",null]]}
   │  [RECV] GameStateUpdate { match_data: ... }
   │  File: client.log appended
   │
   ├─ Loop continues:
   │  │
   │  ├─ clear_screen()
   │  │
   │  ├─ display_rps_match(game_match, my_number=2)
   │  │  • get_rps_state() → RPSGameState { rounds: [(Some("rock"), None)] }
   │  │  • get_score() → (0, 0) (round incomplete)
   │  │  • Prints:
   │  │    ═══════════════════════════════════════
   │  │        ROCK  ·  PAPER  ·  SCISSORS
   │  │    ═══════════════════════════════════════
   │  │
   │  │    Score: You 0 - 0 Opponent (First to 2 wins)
   │  │
   │  ├─ Check my move:
   │  │  • my_number = 2 → check position 1
   │  │  • current_round.1 = None
   │  │  • my_move_submitted = false
   │  │
   │  ├─ Prints:
   │  │  Your turn! Choose your move:
   │  │    1. Rock
   │  │    2. Paper
   │  │    3. Scissors
   │  │
   │  │  Enter choice (1-3): _
   │  │
   │  ├─ User types: 3
   │  │  • read_rps_input() → "scissors"
   │  │
   │  └─ SENDS WebSocket message:
   │     {
   │       "type": "make_move",
   │       "move_data": {"choice": "scissors"}
   │     }
   │     [SEND] MakeMove { move_data: {"choice":"scissors"} }
   │     File: client.log appended
```

---

## Phase 4: Round 1 - Player 2 completes round

### Server:
```
└─ server/src/websocket.rs::handle_socket() (Player2's connection)
   │
   ├─ RECEIVES WebSocket message:
   │  {
   │    "type": "make_move",
   │    "move_data": {"choice": "scissors"}
   │  }
   │  [WS RECV] MakeMove { move_data: Object {"choice": String("scissors")} }
   │
   └─ server/src/game_logic.rs::handle_make_move_logic(200, {"choice":"scissors"}, db)
      │
      ├─ Get active match:
      │  • Returns: Match with game_state: {"rounds":[["rock",null]]}
      │
      ├─ server/src/game_router.rs::handle_rps_move(game_match, 200, move_data)
      │  │
      │  ├─ Deserialize state:
      │  │  • RPSGameState { rounds: [(Some(Rock), None)] }
      │  │
      │  ├─ Deserialize move:
      │  │  • RPSMoveData { choice: Scissors }
      │  │
      │  ├─ Determine player symbol:
      │  │  • player_id (200) == player2_id (200) → player_symbol = 2
      │  │
      │  └─ server/src/games/rock_paper_scissors.rs::RPSEngine::update(state, player=2, move_choice=Scissors)
      │     │
      │     ├─ Check game not finished: ✓
      │     │
      │     ├─ Get current round:
      │     │  • current_round = (Some(Rock), None)
      │     │
      │     ├─ Check player hasn't already moved:
      │     │  • player == 2 → check position 1
      │     │  • current_round.1 == None ✓
      │     │
      │     ├─ Apply move:
      │     │  • new_round = (Some(Rock), Some(Scissors))
      │     │  • new_state.rounds[0] = new_round
      │     │
      │     ├─ Check if both players moved:
      │     │  • new_round = (Some(Rock), Some(Scissors))
      │     │  • BOTH PLAYERS MOVED! Round complete
      │     │
      │     ├─ Check if game is finished:
      │     │  • new_state.get_score()
      │     │  │  │
      │     │  │  ├─ For round (Rock, Scissors):
      │     │  │  │  • Rock.beats(Scissors)
      │     │  │  │  │  │
      │     │  │  │  │  └─ match (Rock, Scissors):
      │     │  │  │  │     • (Rock, Scissors) => Some(Rock)
      │     │  │  │  │
      │     │  │  │  • winner == p1_move → p1_wins += 1
      │     │  │  │
      │     │  │  └─ Returns: (1, 0)
      │     │  │
      │     │  • is_finished(): p1_wins (1) < 2 && p2_wins (0) < 2
      │     │  • Returns: false
      │     │
      │     ├─ Game not finished, add new round:
      │     │  • new_state.rounds.push((None, None))
      │     │  • new_state.rounds = [(Some(Rock), Some(Scissors)), (None, None)]
      │     │
      │     └─ Returns: Ok(RPSGameState { rounds: [...] })
      │
      ├─ Serialize new state:
      │  • {"rounds":[["rock","scissors"],[null,null]]}
      │
      ├─ Check if finished:
      │  • new_state.is_finished() → false
      │  • outcome = None
      │
      ├─ Update database:
      │  • current_player = 2 → next_player = 1
      │  • UPDATE matches SET current_player = 1, game_state = '{"rounds":[["rock","scissors"],[null,null]]}' ...
      │  Console: "Player 200 made move. Match 1: in_progress=true, outcome=None"
      │
      └─ SENDS to both players:
         {
           "type": "game_state_update",
           "match_data": {
             "id": 1,
             "game_state": {"rounds":[["rock","scissors"],[null,null]]},
             "current_player": 1,
             ...
           }
         }
         [WS SEND] GameStateUpdate { match_data: ... } (x2)
```

### Player1:
```
└─ client/src/rps.rs::run_game_loop()
   │
   ├─ Receives GameStateUpdate
   │  • game_state: {"rounds":[["rock","scissors"],[null,null]]}
   │  [RECV] GameStateUpdate { match_data: ... }
   │
   ├─ Loop iteration 2:
   │  │
   │  ├─ display_rps_match(game_match, my_number=1)
   │  │  │
   │  │  ├─ get_score() → (1, 0)
   │  │  │  • Player 1 has 1 win
   │  │  │
   │  │  ├─ Prints:
   │  │  │  Score: You 1 - 0 Opponent (First to 2 wins)
   │  │  │
   │  │  │  Round History:
   │  │  │    Round 1: Rock vs Scissors - You won
   │  │  │
   │  │  └─ (Round 1 shows: my_move=Rock, opp_move=Scissors, result="You won")
   │  │
   │  ├─ Check current round:
   │  │  • current_round = (None, None)
   │  │  • my_move_submitted = false
   │  │
   │  ├─ Prompt for Round 2 move:
   │  │  • User types: 1 (Rock again)
   │  │
   │  └─ SENDS:
   │     {
   │       "type": "make_move",
   │       "move_data": {"choice": "rock"}
   │     }
   │     [SEND] MakeMove { move_data: {"choice":"rock"} }
```

### Player2:
```
└─ client/src/rps.rs::run_game_loop()
   │
   ├─ Receives GameStateUpdate (same as Player1)
   │  [RECV] GameStateUpdate { match_data: ... }
   │
   ├─ display_rps_match(game_match, my_number=2)
   │  • Prints:
   │    Score: You 0 - 1 Opponent (First to 2 wins)
   │
   │    Round History:
   │      Round 1: Scissors vs Rock - You lost
   │
   ├─ Prompt for Round 2 move:
   │  • User types: 2 (Paper)
   │
   └─ SENDS:
      {
        "type": "make_move",
        "move_data": {"choice": "paper"}
      }
      [SEND] MakeMove { move_data: {"choice":"paper"} }
```

---

## Phase 5: Round 2 completes (Player 2 wins)

### Server:
```
• Player 1 move: Rock → state becomes {"rounds":[["rock","scissors"],["rock",null]]}
• Player 2 move: Paper → state becomes {"rounds":[["rock","scissors"],["rock","paper"],[null,null]]}

└─ RPSEngine::update() for Player 2's move:
   │
   ├─ Round 2: (Rock, Paper)
   │  • Paper.beats(Rock) → Some(Paper)
   │  • Winner is player 2
   │
   ├─ get_score() → (1, 1)
   │  • Round 1: P1 wins
   │  • Round 2: P2 wins
   │  • Score tied 1-1
   │
   ├─ is_finished() → false (need 2 wins)
   │
   ├─ Add round 3:
   │  • rounds = [["rock","scissors"], ["rock","paper"], [null,null]]
   │
   └─ SENDS GameStateUpdate to both players with new state
      [WS SEND] GameStateUpdate { match_data: ... } (x2)
```

### Player1:
```
• Displays Round History:
  Round 1: Rock vs Scissors - You won
  Round 2: Rock vs Paper - You lost
  Score: You 1 - 1 Opponent

• Prompts for Round 3:
  User types: 3 (Scissors)
  SENDS: {"type":"make_move","move_data":{"choice":"scissors"}}
  [SEND] MakeMove { move_data: {"choice":"scissors"} }
```

### Player2:
```
• Displays Round History:
  Round 1: Scissors vs Rock - You lost
  Round 2: Paper vs Rock - You won
  Score: You 1 - 1 Opponent

• Prompts for Round 3:
  User types: 2 (Paper)
  SENDS: {"type":"make_move","move_data":{"choice":"paper"}}
  [SEND] MakeMove { move_data: {"choice":"paper"} }
```

---

## Phase 6: Round 3 completes - Match ends (Player 1 wins)

### Server:
```
└─ server/src/game_logic.rs::handle_make_move_logic(200, {"choice":"paper"}, db)
   (Assuming Player 2 moves second)
   │
   ├─ Current state: {"rounds":[["rock","scissors"],["rock","paper"],["scissors",null]]}
   │
   └─ server/src/games/rock_paper_scissors.rs::RPSEngine::update(state, player=2, Paper)
      │
      ├─ Apply move:
      │  • Round 3 becomes: (Some(Scissors), Some(Paper))
      │
      ├─ Compute round winner:
      │  • Scissors.beats(Paper) → Some(Scissors)
      │  • Player 1 wins Round 3
      │
      ├─ get_score():
      │  • Round 1: P1 wins (1-0)
      │  • Round 2: P2 wins (1-1)
      │  • Round 3: P1 wins (2-1)
      │  • Returns: (2, 1)
      │
      ├─ is_finished():
      │  • p1_wins (2) >= 2 → TRUE
      │  • GAME OVER!
      │
      ├─ Do NOT add new round (game finished)
      │  • final rounds: [["rock","scissors"], ["rock","paper"], ["scissors","paper"]]
      │
      ├─ get_winner() → Some(1)
      │
      └─ Returns: RPSGameState {
           rounds: [["rock","scissors"], ["rock","paper"], ["scissors","paper"]]
         }
```

### Server (continued):
```
   ├─ Back in game_router::handle_rps_move():
   │  │
   │  ├─ Serialize state:
   │  │  • {"rounds":[["rock","scissors"],["rock","paper"],["scissors","paper"]]}
   │  │
   │  ├─ Check if finished:
   │  │  • new_state.is_finished() → true
   │  │  • new_state.get_winner() → Some(1)
   │  │  • outcome = Some(MatchOutcome::Player1Win)
   │  │
   │  └─ Returns: GameMoveResult {
   │       new_state: {...},
   │       is_finished: true,
   │       outcome: Some(Player1Win)
   │     }
   │
   └─ Back in handle_make_move_logic():
      │
      ├─ Update database:
      │  • Database::update_match(
      │      match_id=1,
      │      current_player=1,
      │      game_state='{"rounds":[["rock","scissors"],["rock","paper"],["scissors","paper"]]}',
      │      in_progress=false,  ← CHANGED
      │      outcome="p1_win"     ← CHANGED
      │    )
      │  Console: "Player 200 made move. Match 1: in_progress=false, outcome=Some(Player1Win)"
      │
      ├─ Update player scores:
      │  • Database::update_player_scores_from_match(match_record)
      │  │  │
      │  │  └─ server/src/database.rs::update_player_scores_from_match()
      │  │     │
      │  │     ├─ Parse outcome: "p1_win" → MatchOutcome::Player1Win
      │  │     │
      │  │     ├─ UPDATE players SET score = score + 3 WHERE id = 100
      │  │     │  • Player 1 gains 3 points
      │  │     │
      │  │     └─ UPDATE players SET score = score - 1 WHERE id = 200
      │  │        • Player 2 loses 1 point
      │  │
      │  └─ Console: "Updated player scores for match 1: Player 100 won"
      │
      └─ Returns: [
           OutgoingMessage {
             player_id: 100,
             message: GameStateUpdate {
               match_data: Match {
                 in_progress: false,
                 outcome: Some(Player1Win),
                 game_state: {"rounds":[...]},
                 ...
               }
             }
           },
           OutgoingMessage {
             player_id: 200,
             message: GameStateUpdate { ... }
           },
           OutgoingMessage {
             player_id: 100,
             message: MatchEnded { reason: Ended }
           },
           OutgoingMessage {
             player_id: 200,
             message: MatchEnded { reason: Ended }
           }
         ]
```

### Server (sends messages):
```
└─ ConnectionRegistry::send_messages([...])
   │
   ├─ SENDS to Player1:
   │  1. {
   │       "type": "game_state_update",
   │       "match_data": {
   │         "id": 1,
   │         "in_progress": false,
   │         "outcome": "p1_win",
   │         "game_state": {"rounds":[["rock","scissors"],["rock","paper"],["scissors","paper"]]},
   │         ...
   │       }
   │     }
   │     [WS SEND] GameStateUpdate { match_data: ... }
   │
   │  2. {
   │       "type": "match_ended",
   │       "reason": "ended"
   │     }
   │     [WS SEND] MatchEnded { reason: Ended }
   │
   └─ SENDS to Player2:
      (same messages)
      [WS SEND] GameStateUpdate { match_data: ... }
      [WS SEND] MatchEnded { reason: Ended }
```

### Player1:
```
└─ client/src/rps.rs::run_game_loop()
   │
   ├─ Receives GameStateUpdate:
   │  • game_match.in_progress = false
   │  • game_match.outcome = Some(Player1Win)
   │  [RECV] GameStateUpdate { match_data: ... }
   │  File: client.log appended
   │
   ├─ Receives MatchEnded:
   │  [RECV] MatchEnded { reason: Ended }
   │  File: client.log appended
   │
   ├─ Loop iteration:
   │  │
   │  ├─ display_rps_match(game_match, my_number=1)
   │  │  • Prints final state:
   │  │    Score: You 2 - 1 Opponent (First to 2 wins)
   │  │
   │  │    Round History:
   │  │      Round 1: Rock vs Scissors - You won
   │  │      Round 2: Rock vs Paper - You lost
   │  │      Round 3: Scissors vs Paper - You won
   │  │
   │  ├─ Check if game is over:
   │  │  • game_match.in_progress == false
   │  │  • GAME OVER!
   │  │
   │  ├─ Match on outcome:
   │  │  • MatchOutcome::Player1Win && my_number == 1
   │  │  • Prints:
   │  │
   │  │    Match ended!
   │  │    You won the match! 🎉
   │  │
   │  │    Press any key to return to menu...
   │  │
   │  └─ client/src/main.rs::wait_for_keypress()
   │     • Enters raw terminal mode
   │     • Waits for any key press
   │     • User presses any key
   │     • Returns Ok(())
   │
   └─ Returns from run_game_loop() → Returns from start_game()
      • Back to main menu
```

### Player2:
```
└─ client/src/rps.rs::run_game_loop()
   │
   ├─ Receives same messages as Player1
   │  [RECV] GameStateUpdate { match_data: ... }
   │  [RECV] MatchEnded { reason: Ended }
   │
   ├─ display_rps_match(game_match, my_number=2)
   │  • Prints:
   │    Score: You 1 - 2 Opponent (First to 2 wins)
   │
   │    Round History:
   │      Round 1: Scissors vs Rock - You lost
   │      Round 2: Paper vs Rock - You won
   │      Round 3: Paper vs Scissors - You lost
   │
   ├─ Match on outcome:
   │  • MatchOutcome::Player1Win && my_number == 2
   │  • Prints:
   │
   │    Match ended!
   │    You lost the match.
   │
   │    Press any key to return to menu...
   │
   └─ Returns to main menu
```

---

## Summary of Key Functions Called

### Client (Player1 & Player2):
```
1. client/src/auth.rs::try_auto_login()
2. client/src/websocket.rs::WebSocketClient::connect()
3. client/src/main.rs::read_menu_choice()
4. client/src/main.rs::start_game_flow()
5. client/src/rps.rs::start_game()
6. client/src/rps.rs::run_game_loop()
7. client/src/ui.rs::clear_screen()
8. client/src/rps.rs::display_rps_match()
9. client/src/rps.rs::get_rps_state()
10. client/src/rps.rs::RPSGameState::get_score()
11. client/src/rps.rs::RPSGameState::compute_round_winner()
12. client/src/rps.rs::read_rps_input()
13. client/src/websocket.rs::WebSocketClient::send()
14. client/src/rps.rs::wait_for_game_update()
15. client/src/websocket.rs::WebSocketClient::get_messages()
16. client/src/main.rs::wait_for_keypress()
```

### Server:
```
1. server/src/websocket.rs::ws_handler()
2. server/src/websocket.rs::handle_socket()
3. server/src/websocket.rs::authenticate_token()
4. server/src/websocket.rs::ConnectionRegistry::register()
5. server/src/websocket.rs::handle_join_matchmaking()
6. server/src/game_logic.rs::handle_join_matchmaking_logic()
7. server/src/database.rs::Database::get_active_match_for_player()
8. server/src/database.rs::Database::find_waiting_match()
9. server/src/database.rs::Database::create_waiting_match()
10. server/src/database.rs::Database::join_waiting_match()
11. server/src/games/rock_paper_scissors.rs::RPSGameState::new()
12. server/src/database.rs::Database::get_match_by_id()
13. server/src/database.rs::MatchRecord::to_match()
14. server/src/websocket.rs::ConnectionRegistry::send_messages()
15. server/src/websocket.rs::ConnectionRegistry::send_to_player()
16. server/src/websocket.rs::handle_make_move()
17. server/src/game_logic.rs::handle_make_move_logic()
18. server/src/game_router.rs::handle_game_move()
19. server/src/game_router.rs::handle_rps_move()
20. server/src/games/rock_paper_scissors.rs::RPSEngine::update()
21. server/src/games/rock_paper_scissors.rs::RPSGameState::is_finished()
22. server/src/games/rock_paper_scissors.rs::RPSGameState::get_score()
23. server/src/games/rock_paper_scissors.rs::RPSGameState::get_winner()
24. server/src/games/rock_paper_scissors.rs::RPSMove::beats()
25. server/src/database.rs::Database::update_match()
26. server/src/database.rs::Database::update_player_scores_from_match()
```

---

## Database State Changes

### Initial:
```sql
-- Players table
id  | name     | score
----+----------+------
100 | Player1  | 1000
200 | Player2  | 1000

-- Matches table (empty)
```

### After Player1 joins matchmaking:
```sql
INSERT INTO matches (player1_id, game_type, in_progress)
VALUES (100, 'rps', 1);

-- Result:
id | player1_id | player2_id | game_type | game_state | in_progress | outcome | current_player
---+------------+------------+-----------+------------+-------------+---------+----------------
1  | 100        | NULL       | rps       | NULL       | 1           | NULL    | NULL
```

### After Player2 joins (match found):
```sql
UPDATE matches
SET player2_id = 200,
    current_player = 1,
    game_state = '{"rounds":[[null,null]]}'
WHERE id = 1;

-- Result:
1 | 100 | 200 | rps | {"rounds":[[null,null]]} | 1 | NULL | 1
```

### After each move:
```sql
-- Player1 move (Round 1):
UPDATE matches SET current_player = 2, game_state = '{"rounds":[["rock",null]]}' WHERE id = 1;

-- Player2 move (Round 1 complete):
UPDATE matches SET current_player = 1, game_state = '{"rounds":[["rock","scissors"],[null,null]]}' WHERE id = 1;

-- Player1 move (Round 2):
UPDATE matches SET current_player = 2, game_state = '{"rounds":[["rock","scissors"],["rock",null]]}' WHERE id = 1;

-- Player2 move (Round 2 complete):
UPDATE matches SET current_player = 1, game_state = '{"rounds":[["rock","scissors"],["rock","paper"],[null,null]]}' WHERE id = 1;

-- Player1 move (Round 3):
UPDATE matches SET current_player = 2, game_state = '{"rounds":[["rock","scissors"],["rock","paper"],["scissors",null]]}' WHERE id = 1;

-- Player2 move (Round 3 complete - GAME OVER):
UPDATE matches
SET current_player = 1,
    game_state = '{"rounds":[["rock","scissors"],["rock","paper"],["scissors","paper"]]}',
    in_progress = 0,
    outcome = 'p1_win'
WHERE id = 1;
```

### Final state after score update:
```sql
-- Update scores
UPDATE players SET score = score + 3 WHERE id = 100;  -- Winner gets +3
UPDATE players SET score = score - 1 WHERE id = 200;  -- Loser gets -1

-- Final state:
-- Players
id  | name     | score
----+----------+------
100 | Player1  | 1003
200 | Player2  | 999

-- Matches
id | player1_id | player2_id | game_type | game_state                                                          | in_progress | outcome  | current_player
---+------------+------------+-----------+---------------------------------------------------------------------+-------------+----------+----------------
1  | 100        | 200        | rps       | {"rounds":[["rock","scissors"],["rock","paper"],["scissors","paper"]]} | 0           | p1_win   | 1
```
