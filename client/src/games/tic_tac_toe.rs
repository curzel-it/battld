use battld_common::{games::{game_type::GameType, matches::{Match, MatchEndReason, MatchOutcome}, tic_tac_toe::GameState}, *};
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
                print!(" {cell_str} ");
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

fn handle_player_disconnected(
    player_id: i64,
    my_player_id: i64,
    ui_state: &TicTacToeUiState,
    opponent_disconnected: &mut bool,
    waiting_for_input: bool,
    _my_number: i32,
) -> Option<TicTacToeUiState> {
    if player_id == my_player_id {
        return None;
    }

    *opponent_disconnected = true;

    if !waiting_for_input {
        if let TicTacToeUiState::OpponentTurn(match_data) = ui_state {
            return Some(TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone()));
        }
    }

    None
}

fn handle_match_ended(
    reason: &MatchEndReason,
    ui_state: &TicTacToeUiState,
    my_number: Option<i32>,
) -> TicTacToeUiState {
    let final_match = match ui_state {
        TicTacToeUiState::MyTurn(m) |
        TicTacToeUiState::OpponentTurn(m) |
        TicTacToeUiState::WaitingForOpponentToReconnect(m) => m.clone(),
        _ => return ui_state.clone(),
    };

    match reason {
        MatchEndReason::Disconnection => {
            TicTacToeUiState::MatchEndedOpponentDisconnected(final_match)
        }
        MatchEndReason::Ended => {
            determine_match_end_state(&final_match, my_number)
        }
    }
}

fn determine_match_end_state(match_data: &Match, my_number: Option<i32>) -> TicTacToeUiState {
    if let Some(outcome) = &match_data.outcome {
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
    }
}

fn handle_match_found_or_update(
    match_data: &Match,
    my_player_id: i64,
    my_number: &mut Option<i32>,
    ui_state: &TicTacToeUiState,
    opponent_disconnected: bool,
) -> Result<Option<TicTacToeUiState>, Box<dyn std::error::Error>> {
    // Determine player number
    if my_number.is_none() {
        *my_number = Some(if match_data.player1_id == my_player_id { 1 } else { 2 });
    }

    // Check if match has ended
    if !match_data.in_progress {
        return Ok(Some(determine_match_end_state(match_data, *my_number)));
    }

    // Parse game state to determine whose turn it is
    let game_state = serde_json::from_value::<GameState>(match_data.game_state.clone())?;

    let was_opponent_turn = matches!(
        ui_state,
        TicTacToeUiState::OpponentTurn(_) |
        TicTacToeUiState::WaitingForOpponentToReconnect(_) |
        TicTacToeUiState::WaitingForOpponentToJoin
    );

    let new_state = if game_state.current_player == my_number.unwrap() && !game_state.is_finished {
        // If transitioning from opponent's turn to my turn, drain stdin buffer
        if was_opponent_turn {
            crate::ui::drain_stdin_buffer();
        }
        TicTacToeUiState::MyTurn(match_data.clone())
    } else if opponent_disconnected {
        TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone())
    } else {
        TicTacToeUiState::OpponentTurn(match_data.clone())
    };

    Ok(Some(new_state))
}

fn handle_user_input(
    input: &str,
    ui_state: &TicTacToeUiState,
    opponent_disconnected: bool,
    ws_client: &crate::websocket::WebSocketClient,
) -> Result<Option<TicTacToeUiState>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() != 2 {
        println!("{}", "Invalid input format. Use 'row col' (e.g., '1 2')".red());
        print!("  > ");
        io::stdout().flush()?;
        return Ok(None);
    }

    let (Ok(row), Ok(col)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) else {
        println!("{}", "Invalid input format. Use two numbers separated by space.".red());
        print!("  > ");
        io::stdout().flush()?;
        return Ok(None);
    };

    // Validate bounds
    if row >= 3 || col >= 3 {
        println!("{}", "Invalid move. Row and column must be between 0 and 2.".red());
        print!("  > ");
        io::stdout().flush()?;
        return Ok(None);
    }

    // Validate cell is not already occupied
    if let TicTacToeUiState::MyTurn(match_data) = ui_state {
        if let Ok(game_state) = serde_json::from_value::<GameState>(match_data.game_state.clone()) {
            let index = row * 3 + col;
            if game_state.board[index] != 0 {
                println!("{}", "Invalid move. That cell is already occupied.".red());
                print!("  > ");
                io::stdout().flush()?;
                return Ok(None);
            }
        }
    }

    let move_data = serde_json::json!({
        "row": row,
        "col": col
    });
    ws_client.send(ClientMessage::MakeMove { move_data })?;

    if let TicTacToeUiState::MyTurn(match_data) = ui_state {
        let new_state = if opponent_disconnected {
            TicTacToeUiState::WaitingForOpponentToReconnect(match_data.clone())
        } else {
            TicTacToeUiState::OpponentTurn(match_data.clone())
        };
        Ok(Some(new_state))
    } else {
        Ok(None)
    }
}

async fn run_game_loop(
    ws_client: &crate::websocket::WebSocketClient,
    my_player_id: i64,
    initial_state: TicTacToeUiState,
    initial_my_number: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut my_number = initial_my_number;
    let mut ui_state = initial_state;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    // Initial render
    ui_state.render(my_number.unwrap_or(1));

    loop {
        let waiting_for_input = matches!(ui_state, TicTacToeUiState::MyTurn(_));

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    if let ServerMessage::Error { message } = &msg {
                        println!("\n{}", format!("Error: {message}").red());
                        io::stdout().flush()?;
                        continue;
                    }

                    match &msg {
                        ServerMessage::PlayerDisconnected { player_id } => {
                            if let Some(new_state) = handle_player_disconnected(
                                *player_id,
                                my_player_id,
                                &ui_state,
                                &mut opponent_disconnected,
                                waiting_for_input,
                                my_number.unwrap_or(1),
                            ) {
                                ui_state = new_state;
                                ui_state.render(my_number.unwrap_or(1));
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            ui_state = handle_match_ended(reason, &ui_state, my_number);
                            ui_state.render(my_number.unwrap_or(1));
                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::ui::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                            if let Ok(Some(new_state)) = handle_match_found_or_update(
                                match_data,
                                my_player_id,
                                &mut my_number,
                                &ui_state,
                                opponent_disconnected,
                            ) {
                                let should_exit = matches!(
                                    new_state,
                                    TicTacToeUiState::MatchEndedYouWon(_) |
                                    TicTacToeUiState::MatchEndedYouLost(_) |
                                    TicTacToeUiState::MatchEndedDraw(_) |
                                    TicTacToeUiState::MatchEndedOpponentDisconnected(_)
                                );

                                // Reset opponent_disconnected flag if not in waiting state
                                if opponent_disconnected && !matches!(new_state, TicTacToeUiState::WaitingForOpponentToReconnect(_)) {
                                    opponent_disconnected = false;
                                }

                                ui_state = new_state;
                                ui_state.render(my_number.unwrap());

                                if should_exit {
                                    println!("\nPress any key to return to main menu...");
                                    io::stdout().flush()?;
                                    crate::ui::wait_for_keypress()?;
                                    return Ok(());
                                }

                                input_line.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }

            result = stdin_reader.read_line(&mut input_line), if waiting_for_input => {
                if result.is_ok() {
                    let trimmed = input_line.trim().to_string();
                    input_line.clear();

                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(Some(new_state)) = handle_user_input(
                        &trimmed,
                        &ui_state,
                        opponent_disconnected,
                        ws_client,
                    ) {
                        ui_state = new_state;
                        ui_state.render(my_number.unwrap());
                    }
                }
            }
        }
    }
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    run_game_loop(
        ws_client,
        my_player_id,
        TicTacToeUiState::WaitingForOpponentToJoin,
        None,
    ).await
}

pub async fn resume_game(session: &SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    let ws_client = session.ws_client.as_ref().ok_or("Not connected to WebSocket")?;
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    let my_number = Some(if game_match.player1_id == my_player_id { 1 } else { 2 });

    let initial_state = if let Ok(game_state) = serde_json::from_value::<GameState>(game_match.game_state.clone()) {
        if game_state.current_player == my_number.unwrap() && !game_state.is_finished {
            TicTacToeUiState::MyTurn(game_match.clone())
        } else {
            TicTacToeUiState::OpponentTurn(game_match.clone())
        }
    } else {
        TicTacToeUiState::OpponentTurn(game_match.clone())
    };

    run_game_loop(ws_client, my_player_id, initial_state, my_number).await
}
