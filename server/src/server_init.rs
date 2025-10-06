use sqlx::SqlitePool;
use rand::Rng;

const FAKE_USERS: &[(&str, &str)] = &[
    ("Alice", "alice_pk_hint"),
    ("Bob", "bob_pk_hint"),
    ("Charlie", "charlie_pk_hint"),
    ("Diana", "diana_pk_hint"),
    ("Eve", "eve_pk_hint"),
    ("Frank", "frank_pk_hint"),
    ("Grace", "grace_pk_hint"),
    ("Henry", "henry_pk_hint"),
    ("Iris", "iris_pk_hint"),
    ("Jack", "jack_pk_hint"),
    ("Alice2", "alice_pk_hint2"),
    ("Bob2", "bob_pk_hint2"),
    ("Charlie2", "charlie_pk_hint2"),
    ("Diana2", "diana_pk_hint2"),
    ("Eve2", "eve_pk_hint2"),
    ("Frank2", "frank_pk_hint2"),
    ("Grace2", "grace_pk_hint2"),
    ("Henry2", "henry_pk_hint2"),
    ("Iris2", "iris_pk_hint2"),
    ("Jack2", "jack_pk_hint2"),
    ("Alice3", "alice_pk_hint3"),
    ("Bob3", "bob_pk_hint3"),
    ("Charlie3", "charlie_pk_hint3"),
    ("Diana3", "diana_pk_hint3"),
    ("Eve3", "eve_pk_hint3"),
    ("Frank3", "frank_pk_hint3"),
    ("Grace3", "grace_pk_hint3"),
    ("Henry3", "henry_pk_hint3"),
    ("Iris3", "iris_pk_hint3"),
    ("Jack3", "jack_pk_hint3"),
];

fn generate_random_completed_game() -> (String, String, i64) {
    let mut rng = rand::thread_rng();

    // Generate a random completed game
    let mut cells = [0i32; 9];
    let mut current_player = 1;

    // Play random moves until game ends
    loop {
        // Get available positions
        let available: Vec<usize> = cells.iter()
            .enumerate()
            .filter(|(_, &cell)| cell == 0)
            .map(|(i, _)| i)
            .collect();

        if available.is_empty() {
            break;
        }

        // Make a random move
        let pos = available[rng.gen_range(0..available.len())];
        cells[pos] = current_player;

        // Check for winner
        if check_winner(&cells).is_some() {
            break;
        }

        // Switch player
        current_player = if current_player == 1 { 2 } else { 1 };
    }

    let winner = check_winner(&cells);
    let outcome = match winner {
        Some(1) => "p1_win",
        Some(2) => "p2_win",
        _ => "draw",
    };

    let game_state = format!("{{\"cells\":[{},{},{},{},{},{},{},{},{}]}}",
        cells[0], cells[1], cells[2], cells[3], cells[4],
        cells[5], cells[6], cells[7], cells[8]);

    (game_state, outcome.to_string(), current_player as i64)
}

fn check_winner(cells: &[i32; 9]) -> Option<i32> {
    let wins = [
        [0, 1, 2], [3, 4, 5], [6, 7, 8], // rows
        [0, 3, 6], [1, 4, 7], [2, 5, 8], // columns
        [0, 4, 8], [2, 4, 6],            // diagonals
    ];

    for win in &wins {
        let [a, b, c] = *win;
        if cells[a] != 0 && cells[a] == cells[b] && cells[b] == cells[c] {
            return Some(cells[a]);
        }
    }
    None
}

pub async fn seed_users(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if there are any users
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM players")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        println!("Database already has {} users. Skipping seed.", count.0);
        return Ok(());
    }

    println!("No users found. Creating {} fake users...", FAKE_USERS.len());

    let mut player_ids = Vec::new();

    for (name, hint) in FAKE_USERS {
        let public_key = format!("{hint}_public_key_data");

        let result = sqlx::query(
            "INSERT INTO players (public_key_hint, public_key, name) VALUES (?, ?, ?)"
        )
        .bind(hint)
        .bind(&public_key)
        .bind(name)
        .execute(pool)
        .await?;

        let player_id = result.last_insert_rowid();
        player_ids.push(player_id);

        println!("Created user: {name} (ID: {player_id})");
    }

    println!("Successfully created {} fake users!", FAKE_USERS.len());

    // Create random matches between players
    let num_matches = 310;
    println!("\nCreating {num_matches} random matches...");

    let mut rng = rand::thread_rng();

    for i in 0..num_matches {
        // Pick two random different players
        let player1_idx = rng.gen_range(0..player_ids.len());
        let mut player2_idx = rng.gen_range(0..player_ids.len());
        while player2_idx == player1_idx {
            player2_idx = rng.gen_range(0..player_ids.len());
        }

        let player1_id = player_ids[player1_idx];
        let player2_id = player_ids[player2_idx];

        let (game_state, outcome, current_player) = generate_random_completed_game();

        sqlx::query(
            "INSERT INTO matches (player1_id, player2_id, in_progress, outcome, game_type, current_player, game_state)
             VALUES (?, ?, 0, ?, 'tris', ?, ?)"
        )
        .bind(player1_id)
        .bind(player2_id)
        .bind(&outcome)
        .bind(current_player)
        .bind(&game_state)
        .execute(pool)
        .await?;

        println!("Created match {}: Player {} vs Player {} - {}", i + 1, player1_id, player2_id, outcome);
    }

    println!("\nSuccessfully created {num_matches} matches!");

    Ok(())
}
