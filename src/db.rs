use anyhow::Result;
use rusqlite::Connection;
use shellexpand;
use std::collections::HashSet;

use crate::types::Track;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let expanded = shellexpand::tilde(path).to_string();
        let conn = Connection::open(&expanded)?;
        Ok(Self { conn })
    }

    pub fn all_tracks(&self) -> Result<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, artist, album, albumartist, title, duration, year, genre
             FROM tracks
             ORDER BY artist, album, title",
        )?;

        let tracks = stmt
            .query_map([], |row| {
                Ok(Track {
                    path: row.get(0)?,
                    artist: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    album: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    albumartist: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    title: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    duration: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    year: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                    genre: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tracks)
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
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT album FROM tracks WHERE album != '' ORDER BY album",
        )?;
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
