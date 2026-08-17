use std::sync::Arc;
use rusqlite::Connection;
use crate::types::Track;
use crate::util::expand_tilde;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct StateDb {
    conn: Connection,
}

impl StateDb {
    pub fn open() -> Result<Self> {
        let path = expand_tilde("~/.local/share/lyre/state.db");

        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS queue (
                position INTEGER PRIMARY KEY,
                path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS player_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn save_queue(&self, tracks: &[Arc<Track>], queue_index: usize) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM queue", [])?;
        {
            let mut stmt =
                tx.prepare_cached("INSERT INTO queue (position, path) VALUES (?, ?)")?;
            for (i, track) in tracks.iter().enumerate() {
                stmt.execute(rusqlite::params![i as i64, &track.path])?;
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO player_state (key, value) VALUES ('queue_index', ?)",
            [&queue_index.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_queue_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM queue ORDER BY position")?;
        let paths = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(paths)
    }

    pub fn save_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO player_state (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    pub fn load_state(&self, key: &str) -> Option<String> {
        self.conn
            .prepare("SELECT value FROM player_state WHERE key = ?")
            .ok()?
            .query_row([key], |row| row.get(0))
            .ok()
    }
}
