use battld_common::{HEADER_AUTH, LeaderboardResponse};
use colored::*;
use std::io::{self, Write};

use crate::state::*;
use crate::ui::*;

pub async fn show_leaderboard(session: &mut SessionState) -> Result<(), Box<dyn std::error::Error>> {
    if !session.is_authenticated {
        return Err("Not authenticated".into());
    }

    let config = &session.config;
    let server_url = config.server_url.as_ref().ok_or("No server URL configured")?;
    let token = session.auth_token.as_ref().ok_or("No auth token")?;

    let page_size = match crossterm::terminal::size() {
        Ok((_, h)) => {
            ((h as i64).saturating_sub(10)).max(5)
        }
        Err(_) => 10,
    };

    let mut offset = 0i64;

    loop {
        clear_screen()?;
        println!("\n{}", "Loading leaderboard...".cyan());

        let client = reqwest::Client::new();
        let url = format!("{server_url}/leaderboard?limit={page_size}&offset={offset}");

        let response = client
            .get(&url)
            .header(HEADER_AUTH, format!("Bearer {token}"))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Server error: {}", response.status()).into());
        }

        let leaderboard: LeaderboardResponse = response.json().await?;

        clear_screen()?;
        println!();
        println!("{}", "═══════════════════════════════════════════════════════════════════".bright_cyan());
        println!("{}", "                           LEADERBOARD                             ".bright_cyan().bold());
        println!("{}", "═══════════════════════════════════════════════════════════════════".bright_cyan());
        println!();

        let current_page = (offset / page_size) + 1;
        let total_pages = ((leaderboard.total_count + page_size - 1) / page_size).max(1);

        println!("{}", format!("Page {} of {} (Total players: {})", current_page, total_pages, leaderboard.total_count).bright_yellow());
        println!("{}", "───────────────────────────────────────────────────────────────────".dimmed());
        println!("{:>4} {:30} {:>10}",
            "Rank".dimmed(), "Player".dimmed(), "Score".dimmed());
        println!("{}", "───────────────────────────────────────────────────────────────────".dimmed());

        for entry in &leaderboard.entries {
            let rank_str = format!("#{}", entry.rank);
            println!("{:>4} {:30} {:>10}",
                rank_str,
                entry.player_name,
                entry.score);
        }

        println!();
        println!("{}", "═══════════════════════════════════════════════════════════════════".bright_cyan());

        let mut controls = vec![];
        if offset > 0 {
            controls.push("p: previous");
        }
        if offset + page_size < leaderboard.total_count {
            controls.push("n: next");
        }
        controls.push("q: quit");

        println!("{}", controls.join(" | ").dimmed());
        print!("> ");
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "n" if offset + page_size < leaderboard.total_count => {
                offset += page_size;
            }
            "p" if offset > 0 => {
                offset = (offset - page_size).max(0);
            }
            "q" => break,
            _ => {}
        }
    }

    Ok(())
}
