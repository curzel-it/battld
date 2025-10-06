use battld_common::*;
use crate::database::{Database, PlayerRecord, MatchRecord};

pub async fn fetch_player(database: &Database, player_id: i64) -> Option<Player> {
    println!("Fetching player {player_id} from database");

    let record = match database.get_player_by_id(player_id).await {
        Some(record) => record,
        _ => return None
    };

    Some(record.to_player())
}

pub async fn public_key_from_player_id(database: &Database, player_id: i64) -> Option<String> {
    match database.get_player_by_id(player_id).await {
        Some(record) => Some(record.public_key),
        _ => None
    }
}

pub async fn create_player(database: &Database, name: &str, public_key_hint: &str, public_key: &str) -> Option<i64> {
    println!("REPO: Creating player: name='{name}', public_key_hint='{public_key_hint}'");
    match database.create_player(public_key_hint, public_key, name).await {
        Some(id) => {
            println!("REPO: Player created successfully with ID: {id}");
            Some(id)
        },
        _ => {
            println!("REPO: Failed to create player");
            None
        }
    }
}

impl PlayerRecord {
    fn to_player(&self) -> Player {
        Player {
            id: self.id,
            public_key_hint: self.public_key_hint.clone(),
            public_key: self.public_key.clone(),
            name: self.name.clone(),
            score: self.score,
        }
    }
}

impl MatchRecord {
    pub fn to_match(&self) -> Option<Match> {
        let game_state = GameState::from_json(&self.game_state).ok()?;
        let outcome = self.outcome.as_ref().and_then(|s| MatchOutcome::from_string(s));

        Some(Match {
            id: self.id,
            player1_id: self.player1_id,
            player2_id: self.player2_id,
            in_progress: self.in_progress != 0,
            outcome,
            game_type: self.game_type.clone(),
            current_player: self.current_player as i32,
            game_state,
        })
    }
}
