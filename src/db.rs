type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;

use crate::util::expand_tilde;
use crate::types::Track;

pub struct Db {
    conn: Connection,
    music_dir: String,
}

impl Db {
    pub fn open(path: &str, music_dir: &str) -> Result<Self> {
        let expanded = expand_tilde(path);

        if let Some(parent) = Path::new(&expanded).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&expanded)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                artist TEXT,
                album TEXT,
                albumartist TEXT,
                title TEXT,
                duration INTEGER,
                year INTEGER,
                genre TEXT,
                added_at INTEGER DEFAULT (strftime('%s', 'now')),
                favorite INTEGER DEFAULT 0
            )",
            [],
        )?;
        // Migrate existing databases that lack the added_at column
        let _ = conn.execute(
            "ALTER TABLE tracks ADD COLUMN added_at INTEGER DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tracks ADD COLUMN favorite INTEGER DEFAULT 0",
            [],
        );
        Ok(Self { conn, music_dir: music_dir.to_string() })
    }

    fn to_relative<'a>(&self, path: &'a str) -> &'a str {
        Path::new(path)
            .strip_prefix(&self.music_dir)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or(path)
    }

    pub fn all_tracks(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, artist, album, albumartist, title, duration, year, genre, added_at, favorite
             FROM tracks
             ORDER BY artist, album, title",
        )?;

        let music_dir = self.music_dir.clone();
        let tracks = stmt
            .query_map([], |row| {
                let rel: String = row.get(0)?;
                let abs_path = Path::new(&music_dir).join(&rel).to_string_lossy().into_owned();
                Ok(Track {
                    path: abs_path,
                    artist: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    album: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    albumartist: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    title: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    duration: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    year: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                    genre: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                    added_at: row.get::<_, Option<i64>>(8)?.unwrap_or(0),
                    favorite: row.get::<_, Option<i64>>(9)?.unwrap_or(0) != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tracks)
    }

    pub fn set_favorite(&self, path: &str, favorite: bool) -> Result<()> {
        let rel = self.to_relative(path);
        self.conn.execute(
            "UPDATE tracks SET favorite = ? WHERE path = ?",
            rusqlite::params![if favorite { 1 } else { 0 }, rel],
        )?;
        Ok(())
    }

    pub fn update_durations(&self, updates: &[(String, i64)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "UPDATE tracks SET duration = ? WHERE path = ?",
            )?;
            for (path, duration) in updates {
                let rel = self.to_relative(path);
                let _ = stmt.execute(rusqlite::params![duration, rel]);
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_track_metadata(
        &self,
        path: &str,
        title: &str,
        artist: &str,
        album: &str,
        albumartist: &str,
        year: i32,
        genre: &str,
    ) -> Result<()> {
        let rel = self.to_relative(path);
        self.conn.execute(
            "UPDATE tracks SET title = ?, artist = ?, album = ?, albumartist = ?, year = ?, genre = ?
             WHERE path = ?",
            rusqlite::params![title, artist, album, albumartist, year, genre, rel],
        )?;
        Ok(())
    }

    pub fn distinct_artists(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT COALESCE(NULLIF(albumartist,''), artist) as a
             FROM tracks WHERE a != '' ORDER BY a",
        )?;
        let artists: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(artists)
    }

    pub fn distinct_albums(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT album FROM tracks WHERE album != '' ORDER BY album")?;
        let albums: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(albums)
    }

    pub fn distinct_genres(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT genre FROM tracks WHERE genre != ''")?;
        let mut genre_set: HashSet<String> = HashSet::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let genre_str: String = row.get(0)?;
            for g in genre_str.split(',') {
                let g = g.trim().to_string();
                if !g.is_empty() {
                    genre_set.insert(g);
                }
            }
        }
        let mut genres: Vec<String> = genre_set.into_iter().collect();
        genres.sort();
        Ok(genres)
    }
}
