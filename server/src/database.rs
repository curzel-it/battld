use sqlx::{SqlitePool, FromRow};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, FromRow)]
pub struct PlayerRecord {
    pub id: i64,
    pub public_key_hint: String,
    pub public_key: String,
    pub name: String,
    pub score: i64,
}

#[derive(Debug, FromRow)]
pub struct MatchRecord {
    pub id: i64,
    pub player1_id: i64,
    pub player2_id: i64,
    pub in_progress: i64,
    pub outcome: Option<String>,
    pub game_type: String,
    pub current_player: i64,
    pub game_state: String,
}

impl Database {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn from_pool(pool: SqlitePool) -> Self {
        Database { pool }
    }

    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        if let Some(file_path) = database_url.strip_prefix("sqlite://") {
            if !std::path::Path::new(file_path).exists() {
                std::fs::File::create(file_path)
                    .map_err(sqlx::Error::Io)?;
            }
        }

        let pool = SqlitePool::connect(database_url).await?;
        Ok(Database { pool })
    }

    pub async fn initialize(&self) -> Result<(), sqlx::Error> {
        // Run migrations from the migrations directory
        sqlx::migrate!("../migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_player(
        &self,
        public_key_hint: &str,
        public_key: &str,
        name: &str,
    ) -> Option<i64> {
        println!("DB: Inserting player into database: name='{name}', hint='{public_key_hint}'");

        let result = sqlx::query(
            "INSERT INTO players (public_key_hint, public_key, name) VALUES (?, ?, ?)"
        )
        .bind(public_key_hint)
        .bind(public_key)
        .bind(name)
        .execute(&self.pool)
        .await;

        match result {
            Ok(result) => {
                let player_id = result.last_insert_rowid();
                println!("DB: Player inserted successfully with ID: {player_id}");
                Some(player_id)
            },
            Err(e) => {
                println!("DB: Error during player insert {e:#?}");
                None
            }
        }
    }

    pub async fn get_player_by_id(&self, id: i64) -> Option<PlayerRecord> {
        println!("DB: Querying player by ID: {id}");

        let player = sqlx::query_as::<_, PlayerRecord>("SELECT * FROM players WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await.ok().flatten();

        match player {
            Some(p) => {
                println!("DB: Found player: id={}, name='{}'", p.id, p.name);
                Some(p)
            }
            None => {
                println!("DB: No player found with ID: {id}");
                None
            }
        }
    }

    // Match operations
    pub async fn create_match(
        &self,
        player1_id: i64,
        player2_id: i64,
        current_player: i64,
        game_state: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO matches (player1_id, player2_id, in_progress, game_type, current_player, game_state)
             VALUES (?, ?, 1, 'tris', ?, ?)"
        )
        .bind(player1_id)
        .bind(player2_id)
        .bind(current_player)
        .bind(game_state)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn create_waiting_match(&self, player1_id: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO matches (player1_id, player2_id, in_progress, game_type)
             VALUES (?, NULL, 1, 'tris')"
        )
        .bind(player1_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_waiting_match(&self, player_id: i64) -> Option<MatchRecord> {
        sqlx::query_as::<_, MatchRecord>(
            "SELECT * FROM matches WHERE player2_id IS NULL AND player1_id != ? AND in_progress = 1 LIMIT 1"
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn join_waiting_match(
        &self,
        match_id: i64,
        player2_id: i64,
        current_player: i64,
        game_state: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE matches SET player2_id = ?, current_player = ?, game_state = ? WHERE id = ?"
        )
        .bind(player2_id)
        .bind(current_player)
        .bind(game_state)
        .bind(match_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_match_for_player(&self, player_id: i64) -> Option<MatchRecord> {
        sqlx::query_as::<_, MatchRecord>(
            "SELECT * FROM matches WHERE (player1_id = ? OR player2_id = ?) AND in_progress = 1"
        )
        .bind(player_id)
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn update_match(
        &self,
        match_id: i64,
        current_player: i64,
        game_state: &str,
        in_progress: bool,
        outcome: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE matches SET current_player = ?, game_state = ?, in_progress = ?, outcome = ? WHERE id = ?"
        )
        .bind(current_player)
        .bind(game_state)
        .bind(if in_progress { 1 } else { 0 })
        .bind(outcome)
        .bind(match_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_waiting_match_for_player(&self, player_id: i64) -> Option<MatchRecord> {
        sqlx::query_as::<_, MatchRecord>(
            "SELECT * FROM matches WHERE player1_id = ? AND player2_id IS NULL AND in_progress = 1"
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn delete_match(&self, match_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM matches WHERE id = ?")
            .bind(match_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_match_by_id(&self, match_id: i64) -> Option<MatchRecord> {
        sqlx::query_as::<_, MatchRecord>("SELECT * FROM matches WHERE id = ?")
            .bind(match_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
    }

    pub async fn update_player_scores_from_match(&self, match_record: &MatchRecord) -> Result<(), sqlx::Error> {
        if let Some(outcome_str) = &match_record.outcome {
            let player1_score_delta;
            let player2_score_delta;

            match outcome_str.as_str() {
                "p1_win" => {
                    player1_score_delta = 3;
                    player2_score_delta = -1;
                }
                "p2_win" => {
                    player1_score_delta = -1;
                    player2_score_delta = 3;
                }
                "draw" => {
                    player1_score_delta = 1;
                    player2_score_delta = 1;
                }
                _ => return Ok(()), // Unknown outcome, skip
            }

            // Update player1 score
            sqlx::query("UPDATE players SET score = score + ? WHERE id = ?")
                .bind(player1_score_delta)
                .bind(match_record.player1_id)
                .execute(&self.pool)
                .await?;

            // Update player2 score
            sqlx::query("UPDATE players SET score = score + ? WHERE id = ?")
                .bind(player2_score_delta)
                .bind(match_record.player2_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_db() -> Database {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let db = Database::from_pool(pool);
        db.initialize().await.unwrap();
        db
    }

    async fn create_test_player(db: &Database, name: &str) -> i64 {
        db.create_player(&format!("{name}_hint"), &format!("{name}_key"), name)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_update_player_scores_p1_win() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match with p1 winning
        let match_id = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match_id, 1, "{}", false, Some("p1_win")).await.unwrap();

        let match_record = db.get_match_by_id(match_id).await.unwrap();
        db.update_player_scores_from_match(&match_record).await.unwrap();

        // Check scores: p1 should have +3, p2 should have -1
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, 3, "Player 1 (winner) should have +3 points");
        assert_eq!(p2_record.score, -1, "Player 2 (loser) should have -1 points");
    }

    #[tokio::test]
    async fn test_update_player_scores_p2_win() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match with p2 winning
        let match_id = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match_id, 2, "{}", false, Some("p2_win")).await.unwrap();

        let match_record = db.get_match_by_id(match_id).await.unwrap();
        db.update_player_scores_from_match(&match_record).await.unwrap();

        // Check scores: p1 should have -1, p2 should have +3
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, -1, "Player 1 (loser) should have -1 points");
        assert_eq!(p2_record.score, 3, "Player 2 (winner) should have +3 points");
    }

    #[tokio::test]
    async fn test_update_player_scores_draw() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match with draw
        let match_id = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match_id, 1, "{}", false, Some("draw")).await.unwrap();

        let match_record = db.get_match_by_id(match_id).await.unwrap();
        db.update_player_scores_from_match(&match_record).await.unwrap();

        // Check scores: both should have +1
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, 1, "Player 1 should have +1 point for draw");
        assert_eq!(p2_record.score, 1, "Player 2 should have +1 point for draw");
    }

    #[tokio::test]
    async fn test_update_player_scores_multiple_matches() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Match 1: p1 wins
        let match1 = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match1, 1, "{}", false, Some("p1_win")).await.unwrap();
        let match1_record = db.get_match_by_id(match1).await.unwrap();
        db.update_player_scores_from_match(&match1_record).await.unwrap();

        // Match 2: p2 wins
        let match2 = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match2, 2, "{}", false, Some("p2_win")).await.unwrap();
        let match2_record = db.get_match_by_id(match2).await.unwrap();
        db.update_player_scores_from_match(&match2_record).await.unwrap();

        // Match 3: draw
        let match3 = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match3, 1, "{}", false, Some("draw")).await.unwrap();
        let match3_record = db.get_match_by_id(match3).await.unwrap();
        db.update_player_scores_from_match(&match3_record).await.unwrap();

        // Check total scores: p1 = 3-1+1 = 3, p2 = -1+3+1 = 3
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, 3, "Player 1 total: +3 (win) -1 (loss) +1 (draw) = 3");
        assert_eq!(p2_record.score, 3, "Player 2 total: -1 (loss) +3 (win) +1 (draw) = 3");
    }

    #[tokio::test]
    async fn test_update_player_scores_no_outcome() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match without outcome (still in progress)
        let match_id = db.create_match(p1, p2, 1, "{}").await.unwrap();

        let match_record = db.get_match_by_id(match_id).await.unwrap();
        db.update_player_scores_from_match(&match_record).await.unwrap();

        // Scores should remain 0
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, 0, "Player 1 score should be 0 (no outcome)");
        assert_eq!(p2_record.score, 0, "Player 2 score should be 0 (no outcome)");
    }

    #[tokio::test]
    async fn test_update_player_scores_unknown_outcome() {
        let db = create_test_db().await;
        let p1 = create_test_player(&db, "player1").await;
        let p2 = create_test_player(&db, "player2").await;

        // Create a match with an unknown/invalid outcome
        let match_id = db.create_match(p1, p2, 1, "{}").await.unwrap();
        db.update_match(match_id, 1, "{}", false, Some("unknown")).await.unwrap();

        let match_record = db.get_match_by_id(match_id).await.unwrap();
        db.update_player_scores_from_match(&match_record).await.unwrap();

        // Scores should remain 0 (unknown outcomes are skipped)
        let p1_record = db.get_player_by_id(p1).await.unwrap();
        let p2_record = db.get_player_by_id(p2).await.unwrap();

        assert_eq!(p1_record.score, 0, "Player 1 score should be 0 (unknown outcome)");
        assert_eq!(p2_record.score, 0, "Player 2 score should be 0 (unknown outcome)");
    }
}
