-- Add score column to players table
ALTER TABLE players ADD COLUMN score INTEGER NOT NULL DEFAULT 0;
