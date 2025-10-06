-- Initial schema migration
-- Players table (simplified)
CREATE TABLE IF NOT EXISTS players (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    public_key_hint TEXT NOT NULL,
    public_key TEXT NOT NULL,
    name TEXT NOT NULL,
    score INTEGER NOT NULL DEFAULT 0
);

-- Matches table for multi-game support
CREATE TABLE IF NOT EXISTS matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    player1_id INTEGER NOT NULL,
    player2_id INTEGER,
    in_progress INTEGER NOT NULL DEFAULT 1,
    outcome TEXT,
    game_type TEXT NOT NULL DEFAULT 'tris',
    current_player INTEGER,
    game_state TEXT,
    FOREIGN KEY (player1_id) REFERENCES players (id),
    FOREIGN KEY (player2_id) REFERENCES players (id)
);
