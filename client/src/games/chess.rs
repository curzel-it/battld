use battld_common::games::{
    chess::{ChessGameState, ChessPosition, ChessPiece, ChessPieceState, Player},
    game_type::GameType,
    matches::{Match, MatchEndReason, MatchOutcome},
};
use battld_common::*;
use crate::state::SessionState;
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use colored::*;

#[derive(Debug, Clone)]
enum ChessUiState {
    WaitingForOpponentToJoin,
    MyTurn(Match),
    OpponentTurn(Match),
    WaitingForOpponentToReconnect(Match),
    MatchEndedYouWon(Match),
    MatchEndedYouLost(Match),
    MatchEndedDraw(Match),
    MatchEndedOpponentDisconnected(Match),
}

impl ChessUiState {
    fn render(&self, my_player: Player) {
        crate::ui::clear_screen().ok();

        match self {
            ChessUiState::WaitingForOpponentToJoin => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                println!("{}", "  Waiting for opponent to join...".yellow());
                println!();
            }
            ChessUiState::MyTurn(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  YOUR TURN".bright_green().bold());
                println!();
                println!("{}", "  Enter move (e.g., 'e2 e4'):".dimmed());
                print!("  > ");
                io::stdout().flush().ok();
            }
            ChessUiState::OpponentTurn(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  Waiting for opponent's move...".yellow());
                println!();
            }
            ChessUiState::WaitingForOpponentToReconnect(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  Opponent disconnected. Waiting for reconnection...".yellow());
                println!();
            }
            ChessUiState::MatchEndedYouWon(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  YOU WON!".bright_green().bold());
                println!();
            }
            ChessUiState::MatchEndedYouLost(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  You lost.".red());
                println!();
            }
            ChessUiState::MatchEndedDraw(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  It's a draw!".yellow());
                println!();
            }
            ChessUiState::MatchEndedOpponentDisconnected(match_data) => {
                println!("\n{}", "=".repeat(50));
                println!("{}", "  Chess".bright_cyan().bold());
                println!("{}", "=".repeat(50));
                println!();
                render_game_board(match_data, my_player);
                println!();
                println!("{}", "  Match ended - Opponent disconnected.".yellow());
                println!();
            }
        }
    }
}

fn get_piece_symbol(piece: &ChessPieceState) -> &str {
    return match (piece.player, piece.piece) {
        (Player::White, ChessPiece::Pawn) => "♙",
        (Player::White, ChessPiece::Rook) => "♖",
        (Player::White, ChessPiece::Knight) => "♘",
        (Player::White, ChessPiece::Bishop) => "♗",
        (Player::White, ChessPiece::Queen) => "♕",
        (Player::White, ChessPiece::King) => "♔",
        (Player::Black, ChessPiece::Pawn) => "♟",
        (Player::Black, ChessPiece::Rook) => "♜",
        (Player::Black, ChessPiece::Knight) => "♞",
        (Player::Black, ChessPiece::Bishop) => "♝",
        (Player::Black, ChessPiece::Queen) => "♛",
        (Player::Black, ChessPiece::King) => "♚",
    };
}

fn render_game_board(match_data: &Match, my_player: Player) {
    if let Ok(game_state) = serde_json::from_value::<ChessGameState>(match_data.game_state.clone()) {
        println!("  You are: {}", if my_player == Player::White {
            "White (♙)".white()
        } else {
            "Black (♟)".bright_black()
        });

        if let Some(check_player) = game_state.check_state {
            if check_player == my_player {
                println!("  {}", "CHECK!".red().bold());
            } else {
                println!("  {}", "Opponent in check".yellow());
            }
        }

        println!();
        println!("  {}", "a b c d e f g h".dimmed());

        for row in (0..8).rev() {
            print!("{} ", format!("{}", row + 1).dimmed());
            for col in 0..8 {
                let pos = ChessPosition::new(row, col).unwrap();
                if let Some(piece) = game_state.get_piece(pos) {
                    print!("{} ", get_piece_symbol(piece));
                } else {
                    print!("{} ", "·".dimmed());
                }
            }
            println!("{}", format!("{}", row + 1).dimmed());
        }

        println!("  {}", "a b c d e f g h".dimmed());
    }
}

fn handle_player_disconnected(
    player_id: i64,
    my_player_id: i64,
    ui_state: &ChessUiState,
    opponent_disconnected: &mut bool,
    waiting_for_input: bool,
) -> Option<ChessUiState> {
    if player_id == my_player_id {
        return None;
    }

    *opponent_disconnected = true;

    if !waiting_for_input {
        if let ChessUiState::OpponentTurn(match_data) = ui_state {
            return Some(ChessUiState::WaitingForOpponentToReconnect(match_data.clone()));
        }
    }

    None
}

fn handle_match_ended(
    reason: &MatchEndReason,
    ui_state: &ChessUiState,
    my_player: Option<Player>,
) -> ChessUiState {
    let final_match = match ui_state {
        ChessUiState::MyTurn(m) |
        ChessUiState::OpponentTurn(m) |
        ChessUiState::WaitingForOpponentToReconnect(m) => m.clone(),
        _ => return ui_state.clone(),
    };

    match reason {
        MatchEndReason::Disconnection => {
            ChessUiState::MatchEndedOpponentDisconnected(final_match)
        }
        MatchEndReason::Ended => {
            determine_match_end_state(&final_match, my_player)
        }
    }
}

fn determine_match_end_state(match_data: &Match, my_player: Option<Player>) -> ChessUiState {
    if let Some(outcome) = &match_data.outcome {
        match outcome {
            MatchOutcome::Player1Win => {
                if my_player == Some(Player::White) {
                    ChessUiState::MatchEndedYouWon(match_data.clone())
                } else {
                    ChessUiState::MatchEndedYouLost(match_data.clone())
                }
            }
            MatchOutcome::Player2Win => {
                if my_player == Some(Player::Black) {
                    ChessUiState::MatchEndedYouWon(match_data.clone())
                } else {
                    ChessUiState::MatchEndedYouLost(match_data.clone())
                }
            }
            MatchOutcome::Draw => {
                ChessUiState::MatchEndedDraw(match_data.clone())
            }
        }
    } else {
        ChessUiState::MatchEndedDraw(match_data.clone())
    }
}

fn handle_match_found_or_update(
    match_data: &Match,
    my_player_id: i64,
    my_player: &mut Option<Player>,
    ui_state: &ChessUiState,
    opponent_disconnected: bool,
) -> Result<Option<ChessUiState>, Box<dyn std::error::Error>> {
    if my_player.is_none() {
        *my_player = Some(if match_data.player1_id == my_player_id {
            Player::White
        } else {
            Player::Black
        });
    }

    if !match_data.in_progress {
        return Ok(Some(determine_match_end_state(match_data, *my_player)));
    }

    let game_state = serde_json::from_value::<ChessGameState>(match_data.game_state.clone())?;

    let was_opponent_turn = matches!(
        ui_state,
        ChessUiState::OpponentTurn(_) |
        ChessUiState::WaitingForOpponentToReconnect(_) |
        ChessUiState::WaitingForOpponentToJoin
    );

    let new_state = if game_state.current_turn == my_player.unwrap() && !game_state.is_finished() {
        if was_opponent_turn {
            crate::ui::drain_stdin_buffer();
        }
        ChessUiState::MyTurn(match_data.clone())
    } else if opponent_disconnected {
        ChessUiState::WaitingForOpponentToReconnect(match_data.clone())
    } else {
        ChessUiState::OpponentTurn(match_data.clone())
    };

    Ok(Some(new_state))
}

fn handle_user_input(
    input: &str,
    ui_state: &ChessUiState,
    opponent_disconnected: bool,
    ws_client: &crate::websocket::WebSocketClient,
    my_player: Player,
) -> Result<Option<ChessUiState>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() != 2 {
        println!("{}", "Invalid input format. Use 'from to' (e.g., 'e2 e4')".red());
        print!("  > ");
        io::stdout().flush()?;
        return Ok(None);
    }

    let from = ChessPosition::from_algebraic(parts[0]);
    let to = ChessPosition::from_algebraic(parts[1]);

    if from.is_none() || to.is_none() {
        println!("{}", "Invalid position format. Use algebraic notation (e.g., 'e2', 'e4')".red());
        print!("  > ");
        io::stdout().flush()?;
        return Ok(None);
    }

    let from = from.unwrap();
    let to = to.unwrap();
    let chess_move = battld_common::games::chess::ChessMove { from, to };

    if let ChessUiState::MyTurn(match_data) = ui_state {
        if let Ok(game_state) = serde_json::from_value::<ChessGameState>(match_data.game_state.clone()) {
            match game_state.is_valid_move(&chess_move, my_player) {
                Ok(true) => {},
                Ok(false) => {
                    println!("{}", "Invalid move for that piece.".red());
                    print!("  > ");
                    io::stdout().flush()?;
                    return Ok(None);
                }
                Err(msg) => {
                    println!("{}", format!("Invalid move: {msg}").red());
                    print!("  > ");
                    io::stdout().flush()?;
                    return Ok(None);
                }
            }
        }

        let move_data = serde_json::json!({
            "from": from,
            "to": to
        });

        ws_client.send(ClientMessage::MakeMove { move_data })?;

        let new_state = if opponent_disconnected {
            ChessUiState::WaitingForOpponentToReconnect(match_data.clone())
        } else {
            ChessUiState::OpponentTurn(match_data.clone())
        };
        Ok(Some(new_state))
    } else {
        Ok(None)
    }
}

async fn run_game_loop(
    ws_client: &crate::websocket::WebSocketClient,
    my_player_id: i64,
    initial_state: ChessUiState,
    initial_my_player: Option<Player>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut my_player = initial_my_player;
    let mut ui_state = initial_state;
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut input_line = String::new();
    let mut opponent_disconnected = false;

    ui_state.render(my_player.unwrap_or(Player::White));

    loop {
        let waiting_for_input = matches!(ui_state, ChessUiState::MyTurn(_));

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
                            ) {
                                ui_state = new_state;
                                ui_state.render(my_player.unwrap_or(Player::White));
                            }
                        }
                        ServerMessage::MatchEnded { reason } => {
                            ui_state = handle_match_ended(reason, &ui_state, my_player);
                            ui_state.render(my_player.unwrap_or(Player::White));
                            println!("\nPress any key to return to main menu...");
                            io::stdout().flush()?;
                            crate::ui::wait_for_keypress()?;
                            return Ok(());
                        }
                        ServerMessage::MatchFound { match_data } | ServerMessage::GameStateUpdate { match_data } => {
                            if let Ok(Some(new_state)) = handle_match_found_or_update(
                                match_data,
                                my_player_id,
                                &mut my_player,
                                &ui_state,
                                opponent_disconnected,
                            ) {
                                let should_exit = matches!(
                                    new_state,
                                    ChessUiState::MatchEndedYouWon(_) |
                                    ChessUiState::MatchEndedYouLost(_) |
                                    ChessUiState::MatchEndedDraw(_) |
                                    ChessUiState::MatchEndedOpponentDisconnected(_)
                                );

                                if opponent_disconnected && !matches!(new_state, ChessUiState::WaitingForOpponentToReconnect(_)) {
                                    opponent_disconnected = false;
                                }

                                ui_state = new_state;
                                ui_state.render(my_player.unwrap());

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
                        my_player.unwrap(),
                    ) {
                        ui_state = new_state;
                        ui_state.render(my_player.unwrap());
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
        ChessUiState::WaitingForOpponentToJoin,
        None,
    ).await
}

pub async fn resume_game(session: &SessionState, game_match: Match) -> Result<(), Box<dyn std::error::Error>> {
    let ws_client = session.ws_client.as_ref().ok_or("Not connected to WebSocket")?;
    let my_player_id = session.player_id.ok_or("No player ID in session")?;

    let my_player = if game_match.player1_id == my_player_id {
        Player::White
    } else {
        Player::Black
    };

    let initial_state = if let Ok(game_state) = serde_json::from_value::<ChessGameState>(game_match.game_state.clone()) {
        if game_state.current_turn == my_player && !game_state.is_finished() {
            ChessUiState::MyTurn(game_match.clone())
        } else {
            ChessUiState::OpponentTurn(game_match.clone())
        }
    } else {
        ChessUiState::OpponentTurn(game_match.clone())
    };

    run_game_loop(ws_client, my_player_id, initial_state, Some(my_player)).await
}
