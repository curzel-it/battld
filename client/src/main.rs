pub mod api;
pub mod auth;
pub mod config;
pub mod leaderboard;
pub mod games;
pub mod state;
pub mod stats;
pub mod ui;
pub mod utils;
pub mod websocket;

use std::io;

use battld_common::games::{game_type::GameType, matches::Match};
use colored::*;
use crossterm::{event::{self, Event}, terminal};
use rustyline::DefaultEditor;

use auth::try_auto_login;
use leaderboard::*;
use state::*;
use stats::*;
use ui::*;
use utils::VERSION;

use crate::games::{rock_paper_scissors, tic_tac_toe, briscola, chess};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());

    if let Err(e) = start_app(&config_path).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn start_app(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize session
    let mut session = SessionState::new_with_config(config_path)?;

    // Try automatic login
    match try_auto_login(&mut session).await {
        Ok(true) => {
            println!("{}", "✓ Logged in successfully".green());
        }
        Ok(false) | Err(_) => {
            println!("{}", "Please login or create an account:".dimmed());

            // If auto-login fails, try interactive login/registration
            if let Err(e) = auth::handle_login_command(&mut session).await {
                eprintln!("Login failed: {e}");
                return Err("Authentication required".into());
            }
        }
    }

    // Check for resumable match after login
    if let Err(e) = check_and_handle_resumable_match(&mut session).await {
        println!("{}", format!("Resume check error: {e}").yellow());
    }

    // Enter main menu loop
    loop {
        match read_menu_choice(&mut session).await? {
            MenuChoice::StartTicTacToe => {
                // Start TicTacToe game flow
                if let Err(e) = start_game_flow(&mut session, GameType::TicTacToe).await {
                    println!("{}", format!("Game error: {e}").red());
                    println!("\nPress any key to return to menu...");
                    wait_for_keypress()?;
                }
            }
            MenuChoice::StartRockPaperScissors => {
                // Start Rock-Paper-Scissors game flow
                if let Err(e) = start_game_flow(&mut session, GameType::RockPaperScissors).await {
                    println!("{}", format!("Game error: {e}").red());
                    println!("\nPress any key to return to menu...");
                    wait_for_keypress()?;
                }
            }
            MenuChoice::StartBriscola => {
                if let Err(e) = start_game_flow(&mut session, GameType::Briscola).await {
                    println!("{}", format!("Game error: {e}").red());
                    println!("\nPress any key to return to menu...");
                    wait_for_keypress()?;
                }
            }
            // MenuChoice::StartChess => {
            //     if let Err(e) = start_game_flow(&mut session, GameType::Chess).await {
            //         println!("{}", format!("Game error: {e}").red());
            //         println!("\nPress any key to return to menu...");
            //         wait_for_keypress()?;
            //     }
            // }
            MenuChoice::Stats => {
                if let Err(e) = show_stats(&mut session).await {
                    println!("{}", format!("Error loading stats: {e}").red());
                }
                println!("\nPress any key to return to menu...");
                wait_for_keypress()?;
            }
            MenuChoice::Leaderboard => {
                if let Err(e) = show_leaderboard(&mut session).await {
                    println!("{}", format!("Error loading leaderboard: {e}").red());
                }
                println!("\nPress any key to return to menu...");
                wait_for_keypress()?;
            }
            MenuChoice::Exit => {
                println!("\n{}", "Goodbye!".cyan());
                break;
            }
        }
    }

    Ok(())
}

enum MenuChoice {
    StartTicTacToe,
    StartRockPaperScissors,
    StartBriscola,
    // StartChess,
    Stats,
    Leaderboard,
    Exit,
}

fn display_menu(title: &str, items: &[(String, String)]) {
    clear_screen().ok();

    // ASCII art logo
    println!();
    println!();
    println!("{}", "░█▀▄░█▀█░▀█▀░▀█▀░█░░░█▀▄".bright_cyan());
    println!("{}", "░█▀▄░█▀█░░█░░░█░░█░░░█░█".bright_cyan());
    println!("{}", "░▀▀░░▀░▀░░▀░░░▀░░▀▀▀░▀▀░".bright_cyan());
    println!();

    // Title
    println!("{}", title.dimmed());
    println!();

    // Menu items with numbers
    for (num, text) in items {
        println!("  {}. {}", num.bright_yellow(), text);
    }

    println!();
}

async fn read_menu_choice(_session: &mut SessionState) -> io::Result<MenuChoice> {
    let menu_items = vec![
        ("1".to_string(), "Start Tic-Tac-Toe Game".to_string()),
        ("2".to_string(), "Start Rock-Paper-Scissors Game".to_string()),
        ("3".to_string(), "Start Briscola Game".to_string()),
        // ("4".to_string(), "Start Chess Game".to_string()),
        ("4".to_string(), "Your Stats".to_string()),
        ("5".to_string(), "Leaderboard".to_string()),
        ("6".to_string(), "Exit".to_string()),
    ];

    let title = format!("v{VERSION}");
    display_menu(&title, &menu_items);

    let mut rl = DefaultEditor::new().map_err(io::Error::other)?;

    loop {
        let readline = rl.readline("Select option: ");
        match readline {
            Ok(line) => {
                let choice = line.trim();
                match choice {
                    "1" => return Ok(MenuChoice::StartTicTacToe),
                    "2" => return Ok(MenuChoice::StartRockPaperScissors),
                    "3" => return Ok(MenuChoice::StartBriscola),
                    // "4" => return Ok(MenuChoice::StartChess),
                    "4" => return Ok(MenuChoice::Stats),
                    "5" => return Ok(MenuChoice::Leaderboard),
                    "6" => return Ok(MenuChoice::Exit),
                    _ => {
                        println!("{}", format!("Invalid choice. Please enter 1-{}.", menu_items.len() + 1).red());
                        continue;
                    }
                }
            }
            Err(_) => {
                return Ok(MenuChoice::Exit);
            }
        }
    }
}

async fn check_and_handle_resumable_match(session: &mut SessionState) -> Result<(), Box<dyn std::error::Error>> {
    use battld_common::*;

    let ws_client = session.ws_client.as_ref().ok_or("Not connected to WebSocket")?;

    // Wait a bit for server to send ResumableMatch message after auth
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let messages = ws_client.get_messages().await;

    for msg in messages {
        if let ServerMessage::ResumableMatch { match_data } = msg {
            clear_screen()?;
            println!("\n{}", "You have an active match!".yellow().bold());
            println!("{}", format!("Match ID: {}", match_data.id).dimmed());
            println!("{}", format!("Opponent: Player {}", if match_data.player1_id == session.player_id.unwrap() { match_data.player2_id } else { match_data.player1_id }).dimmed());
            println!();

            // Automatically resume
            ws_client.send(ClientMessage::ResumeMatch)?;

            println!("{}", "Resuming match...".cyan());
            let game_match = wait_for_game_state(ws_client).await?;

            // Route to correct game based on game_type
            match game_match.game_type {
                GameType::TicTacToe => {
                    tic_tac_toe::resume_game(session, game_match).await?;
                }
                GameType::RockPaperScissors => {
                    rock_paper_scissors::resume_game(session, game_match).await?;
                }
                GameType::Briscola => {
                    briscola::resume_game(session, game_match).await?;
                }
                GameType::Chess => {
                    chess::resume_game(session, game_match).await?;
                }
            }

            return Ok(());
        }
    }

    Ok(())
}

async fn wait_for_game_state(ws_client: &crate::websocket::WebSocketClient) -> Result<Match, Box<dyn std::error::Error>> {
    use battld_common::*;

    loop {
        let messages = ws_client.get_messages().await;

        for msg in messages {
            if let ServerMessage::GameStateUpdate { match_data } = msg {
                return Ok(match_data);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

async fn start_game_flow(session: &mut SessionState, game_type: GameType) -> Result<(), Box<dyn std::error::Error>> {
    clear_screen()?;

    println!("\n{}", format!("Starting {game_type} matchmaking...").cyan());
    println!("{}", "Waiting for opponent...".dimmed());

    // Route to appropriate game module
    match game_type {
        GameType::TicTacToe => games::tic_tac_toe::start_game(session, game_type).await?,
        GameType::RockPaperScissors => games::rock_paper_scissors::start_game(session, game_type).await?,
        GameType::Briscola => games::briscola::start_game(session, game_type).await?,
        GameType::Chess => games::chess::start_game(session, game_type).await?,
    }

    Ok(())
}

fn wait_for_keypress() -> io::Result<()> {
    terminal::enable_raw_mode()?;

    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(_) = event::read()? {
                break Ok(());
            }
        }
    };

    terminal::disable_raw_mode()?;
    result
}