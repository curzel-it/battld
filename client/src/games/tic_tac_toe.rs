use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use colored::*;

#[derive(Debug, Clone)]
enum TicTacToeUiState {
    WaitingForOpponentToJoin,
    MyTurn(Match),
    OpponentTurn(Match),
    WaitingForOpponentToReconnect(Match),
    MatchEndedYouWon(Match),
    MatchEndedYouLost(Match),
    MatchEndedDraw(Match),
    MatchEndedOpponentDisconnected(Match),
}

impl TicTacToeUiState {
    fn render(&self, my_player_number: i32) {
        crate::ui::clear_screen().ok();

        match self {
            TicTacToeUiState::WaitingForOpponentToJoin => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                println!("{}", "  Waiting for opponent to join...".yellow());
                println!();
            }
            TicTacToeUiState::MyTurn(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  YOUR TURN".bright_green().bold());
                println!();
                println!("{}", "  Enter move as 'row col' (0-indexed, e.g., '1 2'):".dimmed());
                print!("  > ");
                io::stdout().flush().ok();
            }
            TicTacToeUiState::OpponentTurn(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  Waiting for opponent's move...".yellow());
                println!();
            }
            TicTacToeUiState::WaitingForOpponentToReconnect(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  Opponent disconnected. Waiting for reconnection...".yellow());
                println!();
            }
            TicTacToeUiState::MatchEndedYouWon(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  YOU WON! 🎉".bright_green().bold());
                println!();
            }
            TicTacToeUiState::MatchEndedYouLost(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  You lost.".red());
                println!();
            }
            TicTacToeUiState::MatchEndedDraw(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  It's a draw!".yellow());
                println!();
            }
            TicTacToeUiState::MatchEndedOpponentDisconnected(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Tic-Tac-Toe".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player_number);
                println!();
                println!("{}", "  Match ended - Opponent disconnected.".yellow());
                println!();
            }
        }
    }
}

fn render_game_board(match_data: &Match, my_player_number: i32) {
    if let Ok(game_state) = serde_json::from_value::<GameState>(match_data.game_state.clone()) {
        println!("  You are: {}", if my_player_number == 1 { "X".bright_blue() } else { "O".bright_magenta() });
        println!();

        // Render the board (3x3 grid from flat array)
        for row in 0..3 {
            print!("  ");
            for col in 0..3 {
                let idx = row * 3 + col;
                let cell = game_state.board[idx];
                let cell_str = match cell {
                    0 => "·".dimmed().to_string(),
                    1 => "X".bright_blue().to_string(),
                    2 => "O".bright_magenta().to_string(),
                    _ => " ".to_string(),
                };
                print!(" {} ", cell_str);
                if col < 2 {
                    print!("{}", "|".dimmed());
                }
            }
            println!();
            if row < 2 {
                println!("  {}", "---+---+---".dimmed());
            }
        }
    }
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    let mut my_number: Option<i32> = None;
    let mut ui_state = TicTacToeUiState::WaitingForOpponentToJoin;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    // Initial render
    ui_state.render(my_number.unwrap_or(1));

    // Main game loop
    loop {
        let waiting_for_input = matches!(ui_state, TicTacToeUiState::MyTurn(_));

        tokio::select! {
            // Poll for incoming WebSocket messages
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    // Handle errors
                    if let ServerMessage::Error { message } = &msg {
                        println!("\n{}", format!("Error: {}", message).red());
                        io::stdout().flush()?;
                        continue;
                    }

                    // Update state based on message
                    match &msg {
                        ServerMessage::PlayerDisconnected { player_id } => {
                            // Check if it's the opponent
                            if *player_id != my_player_id {
                                opponent_disconnected = true;

                                // Only transition to WaitingForOpponentToReconnect if not MyTurn
                                if !waiting_for_input {
                                    if let TicTacToeUiState::OpponentTurn(match_data) = &ui_state {
                                        ui_state = TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone());
                                        ui_state.render(my_number.unwrap_or(1));
                                    }
                                }
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            // Get the last match data from current state
                            let final_match = match &ui_state {
                                TicTacToeUiState::MyTurn(m) |
                                TicTacToeUiState::OpponentTurn(m) |
                                TicTacToeUiState::WaitingForOpponentToReconnect(m) => m.clone(),
                                _ => {
                                    println!("\n{}", "Match ended".yellow());
                                    println!("\nPress any key to return to main menu...");
                                    io::stdout().flush()?;
                                    crate::wait_for_keypress()?;
                                    return Ok(());
                                }
                            };

                            ui_state = match reason {
                                MatchEndReason::Disconnection => {
                                    TicTacToeUiState::MatchEndedOpponentDisconnected(final_match)
                                }
                                MatchEndReason::Ended => {
                                    // Check outcome
                                    if let Some(outcome) = &final_match.outcome {
                                        match outcome {
                                            MatchOutcome::Player1Win => {
                                                if my_number == Some(1) {
                                                    TicTacToeUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    TicTacToeUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Player2Win => {
                                                if my_number == Some(2) {
                                                    TicTacToeUiState::MatchEndedYouWon(final_match)
                                                } else {
                                                    TicTacToeUiState::MatchEndedYouLost(final_match)
                                                }
                                            }
                                            MatchOutcome::Draw => {
                                                TicTacToeUiState::MatchEndedDraw(final_match)
                                            }
                                        }
                                    } else {
                                        TicTacToeUiState::MatchEndedDraw(final_match)
                                    }
                                }
                            };

                            ui_state.render(my_number.unwrap_or(1));
                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                            // Determine which player we are (1 or 2)
                            if my_number.is_none() {
                                my_number = Some(if match_data.player1_id == my_player_id { 1 } else { 2 });
                            }

                            // Check if match has ended (in the match data itself)
                            if !match_data.in_progress {
                                ui_state = if let Some(outcome) = &match_data.outcome {
                                    match outcome {
                                        MatchOutcome::Player1Win => {
                                            if my_number == Some(1) {
                                                TicTacToeUiState::MatchEndedYouWon(match_data.clone())
                                            } else {
                                                TicTacToeUiState::MatchEndedYouLost(match_data.clone())
                                            }
                                        }
                                        MatchOutcome::Player2Win => {
                                            if my_number == Some(2) {
                                                TicTacToeUiState::MatchEndedYouWon(match_data.clone())
                                            } else {
                                                TicTacToeUiState::MatchEndedYouLost(match_data.clone())
                                            }
                                        }
                                        MatchOutcome::Draw => {
                                            TicTacToeUiState::MatchEndedDraw(match_data.clone())
                                        }
                                    }
                                } else {
                                    TicTacToeUiState::MatchEndedDraw(match_data.clone())
                                };

                                ui_state.render(my_number.unwrap());
                                println!("\nPress any key to return to main menu...");
                                io::stdout().flush()?;
                                crate::wait_for_keypress()?;
                                return Ok(());
                            }

                            // Parse game state to determine whose turn it is
                            if let Ok(game_state) = serde_json::from_value::<GameState>(match_data.game_state.clone()) {
                                if game_state.current_player == my_number.unwrap() && !game_state.is_finished {
                                    ui_state = TicTacToeUiState::MyTurn(match_data.clone());
                                } else if opponent_disconnected {
                                    ui_state = TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone());
                                } else {
                                    ui_state = TicTacToeUiState::OpponentTurn(match_data.clone());
                                }

                                // Reset opponent_disconnected flag when we receive a game state update
                                // (means they reconnected)
                                if opponent_disconnected && !matches!(ui_state, TicTacToeUiState::WaitingForOpponentToReconnect(_)) {
                                    opponent_disconnected = false;
                                }

                                ui_state.render(my_number.unwrap());
                            }
                        }
                        _ => {}
                    }
                }
            },

            // Poll for user input (only when in MyTurn state)
            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                if let Ok(_) = result {
                    let parts: Vec<&str> = input_line.trim().split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(row), Ok(col)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            // Send move
                            let move_data = serde_json::json!({
                                "row": row,
                                "col": col
                            });
                            ws_client.send(ClientMessage::MakeMove { move_data })?;

                            // Transition to OpponentTurn or WaitingForOpponentToReconnect
                            if let TicTacToeUiState::MyTurn(match_data) = &ui_state {
                                ui_state = if opponent_disconnected {
                                    TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone())
                                } else {
                                    TicTacToeUiState::OpponentTurn(match_data.clone())
                                };
                                ui_state.render(my_number.unwrap());
                            }
                        } else {
                            println!("{}", "Invalid input format. Use two numbers separated by space.".red());
                            print!("  > ");
                            io::stdout().flush()?;
                        }
                    } else {
                        println!("{}", "Invalid input format. Use 'row col' (e.g., '1 2')".red());
                        print!("  > ");
                        io::stdout().flush()?;
                    }
                    input_line.clear();
                }
            }
        }
    }
}

pub async fn resume_game(_session: &SessionState, _game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resume game not implemented in simple message printer mode");
    Ok(())
}
