use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use rustyline::DefaultEditor;
use std::io::{self, Write};
use battld_common::*;
use crate::state::SessionState;
use crate::websocket::WebSocketClient;
use std::fs::OpenOptions;

/// Helper to extract GameState from Match for TicTacToe
fn get_tic_tac_toe_state(game_match: &Match) -> Result<GameState, Box<dyn std::error::Error>> {
    serde_json::from_value(game_match.game_state.clone())
        .map_err(|e| format!("Failed to deserialize game state: {e}").into())
}

pub async fn start_game(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure WebSocket connection
    if session.ws_client.is_none() {
        session.connect_websocket().await?;
    }

    let ws_client = session.ws_client.as_ref().unwrap();

    // Join matchmaking with specified game type
    ws_client.send(ClientMessage::JoinMatchmaking { game_type })?;

    // Wait for match
    let game_match = 'outer: loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            match msg {
                ServerMessage::WaitingForOpponent => {
                    // Continue waiting
                }
                ServerMessage::MatchFound { match_data } => {
                    clear_screen()?;
                    println!("{}", "Match found!".green().bold());
                    break 'outer match_data;
                }
                ServerMessage::GameStateUpdate { match_data } => {
                    clear_screen()?;
                    println!("{}", "Match found!".green().bold());
                    break 'outer match_data;
                }
                ServerMessage::Error { message } => {
                    return Err(format!("Error: {message}").into());
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    };

    // Enter gameplay loop
    run_game_loop(session, game_match).await
}

pub async fn resume_game(session: &SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    clear_screen()?;
    println!("{}", "Match resumed!".green().bold());

    // Enter gameplay loop
    run_game_loop(session, game_match).await
}

async fn run_game_loop(session: &SessionState, mut game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    let ws_client = session.ws_client.as_ref().unwrap();
    let player_id = session.player_id.unwrap();
    let am_player1 = game_match.player1_id == player_id;
    let my_number = if am_player1 { 1 } else { 2 };
    let mut status_message: Option<String> = None;

    loop {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
            let _ = writeln!(file, "[GAME LOOP] Top of loop, in_progress: {}, outcome: {:?}", game_match.in_progress, game_match.outcome);
        }

        clear_screen()?;
        display_match(&game_match, player_id);

        // Display status message if any
        if let Some(msg) = &status_message {
            println!("\n{msg}");
        }

        // Check if game is finished
        if !game_match.in_progress {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                let _ = writeln!(file, "[GAME LOOP] Game finished, checking if disconnect-caused...");
            }

            // Check if this is a disconnect-caused draw
            // A draw from disconnect will have outcome=Draw but the board won't be full
            let is_disconnect_draw = if let Some(MatchOutcome::Draw) = game_match.outcome {
                // Check if board is full or has a winner
                let state = get_tic_tac_toe_state(&game_match)?;
                let board_full = state.is_full();
                let has_winner = state.check_winner().is_some();

                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                    let _ = writeln!(file, "[GAME LOOP] Draw detected: board_full={board_full}, has_winner={has_winner}");
                }

                // If it's a draw but board isn't full and no winner, it's a disconnect
                !board_full && !has_winner
            } else {
                false
            };

            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                let _ = writeln!(file, "[GAME LOOP] is_disconnect_draw: {is_disconnect_draw}");
            }

            if is_disconnect_draw {
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                    let _ = writeln!(file, "[GAME LOOP] Showing disconnect message");
                }
                clear_screen()?;
                println!("\n{}", "Connection dropped, press any key to go back to main menu".yellow());
                wait_for_keypress()?;
                return Err("Connection dropped".into());
            }

            println!("\n{}", "Press any key to return to menu...".dimmed());
            wait_for_keypress()?;
            return Ok(());
        }

        // Check if it's our turn
        if game_match.current_player == my_number {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                let _ = writeln!(file, "[GAME LOOP] Our turn, prompting for input");
            }

            println!("\n{}", "Your turn! Enter move (row col):".cyan());
            print!("> ");
            io::stdout().flush()?;

            let (row, col) = read_game_input()?;
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                let _ = writeln!(file, "[GAME LOOP] User entered move: ({row}, {col})");
            }

            // Send move with generic move_data
            let move_data = serde_json::json!({"row": row, "col": col});
            ws_client.send(ClientMessage::MakeMove { move_data })?;

            // Wait for response
            let (new_match, new_status) = wait_for_game_update(ws_client).await?;
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                let _ = writeln!(file, "[GAME LOOP] Got response, in_progress: {}, outcome: {:?}", new_match.in_progress, new_match.outcome);
            }
            game_match = new_match;
            status_message = new_status;
        } else {
            println!("\n{}", "Opponent's turn, waiting for their move...".dimmed());

            // Enable raw mode to catch CTRL+C
            terminal::enable_raw_mode()?;

            // Poll for messages and update display continuously
            'wait_loop: loop {
                // Check if connection dropped while waiting
                if !ws_client.is_connected().await {
                    terminal::disable_raw_mode()?;
                    clear_screen()?;
                    println!("\n{}", "Connection dropped, press any key to go back to main menu".yellow());
                    wait_for_keypress()?;
                    return Err("Connection dropped".into());
                }

                // Check for CTRL+C during the wait
                if event::poll(std::time::Duration::from_millis(200))? {
                    if let Event::Key(key) = event::read()? {
                        // Ignore CTRL+C during gameplay
                        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                            // Do nothing, continue waiting
                        }
                    }
                }

                // Check for new messages
                let messages = ws_client.get_messages().await;

                for msg in messages {
                    match msg {
                        ServerMessage::GameStateUpdate { match_data } => {
                            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("client.log") {
                                let _ = writeln!(file, "[OPPONENT WAIT] Got GameStateUpdate, in_progress: {}, outcome: {:?}", match_data.in_progress, match_data.outcome);
                            }
                            game_match = match_data;
                            terminal::disable_raw_mode()?;
                            break 'wait_loop;
                        }
                        ServerMessage::PlayerDisconnected { player_id: disconnected_id } => {
                            terminal::disable_raw_mode()?;
                            clear_screen()?;
                            display_match(&game_match, player_id);
                            println!("\n{}", format!("Opponent (Player {disconnected_id}) disconnected!").yellow().bold());
                            println!("{}", "Waiting for them to reconnect (10 seconds)...".dimmed());

                            // Re-enable raw mode and continue waiting
                            terminal::enable_raw_mode()?;
                        }
                        ServerMessage::Error { message } => {
                            terminal::disable_raw_mode()?;
                            return Err(format!("Error: {message}").into());
                        }
                        ServerMessage::AuthSuccess { player_id } => {
                            println!("AuthSuccess: {player_id}");
                        }
                        ServerMessage::AuthFailed { reason } => {
                            println!("AuthFailed: {reason}");
                        }
                        ServerMessage::WaitingForOpponent => {
                            println!("WaitingForOpponent");
                        }
                        ServerMessage::MatchFound { match_data } => {
                            println!("MatchFound: {match_data:#?}");
                        }
                        ServerMessage::ResumableMatch { match_data } => {
                            println!("ResumableMatch: {match_data:#?}");
                        }
                        ServerMessage::Pong => {
                            println!("Pong");
                        }
                        ServerMessage::MatchEnded { reason } => {
                            let reason_str = match reason {
                                battld_common::MatchEndReason::Ended => "ended",
                                battld_common::MatchEndReason::Disconnection => "disconnection",
                            };
                            terminal::disable_raw_mode()?;
                            clear_screen()?;
                            println!("\n{}", format!("Match ended ({reason_str}). Press any key to go back to main menu").yellow());
                            io::stdout().flush()?;
                            ws_client.close().await;
                            wait_for_keypress()?;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

fn read_game_input() -> io::Result<(usize, usize)> {
    use rustyline::config::Configurer;

    let mut rl = DefaultEditor::new().map_err(io::Error::other)?;
    rl.set_max_history_size(0).map_err(io::Error::other)?;

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();

                // Try to parse as "row col"
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(row), Ok(col)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                        return Ok((row, col));
                    }
                }

                // Invalid input, re-prompt
                println!("{}", "Invalid input. Enter 'row col':".red());
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // CTRL+C pressed - ignore it and continue the loop
                continue;
            }
            Err(_) => {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "Input cancelled"));
            }
        }
    }
}

async fn wait_for_game_update(ws_client: &WebSocketClient) -> Result<(Match, Option<String>), Box<dyn std::error::Error>> {
    let mut opponent_disconnected = false;

    loop {
        // Check if connection is still alive
        if !ws_client.is_connected().await {
            // Connection dropped - show message and wait for keypress
            clear_screen().ok();
            println!("\n{}", "Connection dropped, press any key to go back to main menu".yellow());
            wait_for_keypress().ok();
            return Err("Connection dropped".into());
        }

        let messages = ws_client.get_messages().await;

        for msg in messages {
            match msg {
                ServerMessage::GameStateUpdate { match_data } => {
                    return Ok((match_data, None));
                }
                ServerMessage::PlayerDisconnected { player_id: disconnected_id } => {
                    if !opponent_disconnected {
                        opponent_disconnected = true;
                        clear_screen().ok();
                        println!("\n{}", format!("Opponent (Player {disconnected_id}) disconnected!").yellow().bold());
                        println!("{}", "Waiting for them to reconnect (10 seconds)...".dimmed());
                    }
                }
                ServerMessage::Error { message } => {
                    return Err(format!("Error: {message}").into());
                }
                ServerMessage::MatchEnded { reason } => {
                    let reason_str = match reason {
                        battld_common::MatchEndReason::Ended => "ended",
                        battld_common::MatchEndReason::Disconnection => "disconnection",
                    };
                    clear_screen().ok();
                    println!("\n{}", format!("Match ended ({reason_str}). Press any key to go back to main menu").yellow());
                    io::stdout().flush().ok();
                    ws_client.close().await;
                    wait_for_keypress().ok();
                    return Err("Match ended".into());
                }
                _ => {}
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

fn clear_screen() -> io::Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush()?;
    Ok(())
}

fn wait_for_keypress() -> io::Result<()> {
    terminal::enable_raw_mode()?;

    // Drain any existing events
    while event::poll(std::time::Duration::from_millis(0))? {
        event::read()?;
    }

    // Now wait for a new keypress (ignore CTRL+C)
    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Ignore CTRL+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    continue;
                }
                break Ok(());
            }
        }
    };

    terminal::disable_raw_mode()?;
    result
}

fn display_match(game_match: &Match, current_player_id: i64) {
    println!("\n{}", "─── Tic Tac Toe ───".bold());
    println!("Match ID: {}", game_match.id);

    let am_player1 = game_match.player1_id == current_player_id;
    let my_number = if am_player1 { 1 } else { 2 };
    let opponent_id = if am_player1 { game_match.player2_id } else { game_match.player1_id };

    println!("You (Player {my_number}) vs Player {opponent_id}");

    if game_match.in_progress {
        if game_match.current_player == my_number {
            println!("{} {}", "Your turn!".green().bold(), "(row col + enter)".dimmed());
        } else {
            println!("{}", "Opponent's turn".yellow());
        }
    } else if let Some(outcome) = &game_match.outcome {
        match outcome {
            MatchOutcome::Player1Win => {
                if am_player1 {
                    println!("{}", "You won!".green().bold());
                } else {
                    println!("{}", "You lost.".red());
                }
            },
            MatchOutcome::Player2Win => {
                if !am_player1 {
                    println!("{}", "You won!".green().bold());
                } else {
                    println!("{}", "You lost.".red());
                }
            },
            MatchOutcome::Draw => {
                println!("{}", "Draw!".yellow());
            }
        }
    }

    println!();
    let state = get_tic_tac_toe_state(game_match).unwrap_or_else(|_| GameState::new());
    display_board(&state, my_number);
}

fn display_board(state: &GameState, my_number: i32) {
    println!("     0   1   2");
    println!();

    for row in 0..3 {
        print!(" {row}  ");
        for col in 0..3 {
            let idx = row * 3 + col;
            let cell = state.cells[idx];
            let symbol = match cell {
                0 => " ".to_string(),
                n if n == my_number => "X".green().bold().to_string(),
                _ => "O".red().bold().to_string(),
            };
            let pipe = if col != 2 { "│" } else { "" };
            print!(" {symbol} {pipe}");
        }
        println!();
        if row < 2 {
            println!("    ───┼───┼─── ");
        }
    }

    println!();
}
