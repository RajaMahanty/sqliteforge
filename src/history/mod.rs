use rusqlite::Connection;
use std::path::PathBuf;

/// Persistent query history backed by SQLite
pub struct History {
    conn: Connection,
}

impl History {
    /// Open or create the history database
    pub fn open() -> Result<Self, String> {
        let path = Self::history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC);",
        )
        .map_err(|e| e.to_string())?;

        Ok(Self { conn })
    }

    /// Returns the path to the history database
    pub fn history_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("sqliteforge")
            .join("history.db")
    }

    /// Add a query to history
    pub fn add(&self, query: &str) -> Result<(), String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        self.conn
            .execute("INSERT INTO history (query) VALUES (?1)", [trimmed])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Search history with a pattern (fuzzy)
    pub fn search(&self, pattern: &str) -> Vec<String> {
        let like_pattern = format!("%{}%", pattern);
        self.conn
            .prepare(
                "SELECT DISTINCT query FROM history WHERE query LIKE ?1 ORDER BY id DESC LIMIT 50",
            )
            .ok()
            .map(|mut stmt| {
                stmt.query_map([&like_pattern], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get recent history entries
    pub fn recent(&self, limit: usize) -> Vec<String> {
        self.conn
            .prepare("SELECT query FROM history ORDER BY id DESC LIMIT ?1")
            .ok()
            .map(|mut stmt| {
                stmt.query_map([limit as i64], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get all history for reedline integration
    pub fn all_entries(&self) -> Vec<String> {
        self.conn
            .prepare("SELECT query FROM history ORDER BY id ASC")
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }
}
