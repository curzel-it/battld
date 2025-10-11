# Hello Claude!

## Handling a Task
1. Create a plan.md file with a comprehensive list of what you need to change, where, code links and code snippets (especially of data structures)
2. Ask me questions if some things are uncertain
3. Update the plan with my answers
4. Implement and test frequently
5. Review and cleanup
6. Enjoy!

## Style and other guideliens
Stick to style and preferences existing in the current codebase.
After implementing any changes run a code review on all edited files.
Run `cargo clippy` for hints on what can be improved.
Before marking the task as completed, make sure you remove all unnecessary comments.

## Testing
All the code you write should be (at least somewhat) unit-testable, in particular game engines.
Make sure to implement all tests that it makes sense to have

Test your implementation by building both client and server to spot compilation errors (`cargo build --bin server` and `cargo build --bin client` respectively).
Remember to run `cargo test` frequently to make sure you did not introduce breaking changes.

## How to add a new game

### About this section
- This is a comprehensive list of everything you will need to do to implement a new game in Battld.
- We have references to <game_name> and <GameName> to allow for easier copy-pasting of rules, what we want is to actually have a decent name in there (such as rock_paper_scissors and RockPaperScissors).

### Files to create
As for all our games, the file structure is the following:
- common/src/games/<game_name>.rs: <GameName>Move, <GameName>GameState, and similar structs
- server/src/games/<game_name>.rs: <GameName>Engine, server-side logic
- client/src/games/<game_name>.rs: <GameName>UiState, client-side rendering and input logic

### Routing
Battld is a hub for lots of different games, the following files will need to be updated as they handle "game routing":
- server/src/game_router.rs
- client/src/main.rs 

### Server and Client communication
Client and server communicate via websockets using `ServerMessage` and `ClientMessage` respectively.
You are expected to use these without changes, as they contain generic types to handle different game moves and such.

### Match State
Defined in common/src/games/matches.rs, Match is the data structure that defines and contains the state of a game (be it ongoing or ended).
You will notice this property in particular `pub game_state: serde_json::Value`, which is generic - we use this to store *GameState structs (such as RockPaperScissorsGameState or BriscolaGameState).
Your definition of <GameName>GameState will need to take everything the server and the client need for running the game in all of its parts (sometimes the ui changes a bit during early vs late rounds, you will need to take that into account).

### Client
Implement the simplest possible version of the ui.
If a textual interface makes sense stick to that with simple input.

Depending on the kind of game you are implementing, read one of the following for inspiration:
- client/src/games/rock_paper_scissors.rs uses a text-only interface
- client/src/games/tic_tac_toe.rs uses a simple ascii-art grid
- client/src/games/briscola.rs uses ascii-art to draw cards on the screen

In general, if the game needs a board and has a lot of units (or cards) it's better to do at least some ascii art.