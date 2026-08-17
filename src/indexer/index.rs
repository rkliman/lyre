use crate::util::expand_tilde;
use globset::{Glob, GlobSetBuilder};
use lofty::file::TaggedFileExt;
use lofty::prelude::ItemKey;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::util::{
    collect_existing_paths_column, extract_song_name_from_filename,
    generate_path_from_pattern, tag_str, update_playlist_line, TickingBar,
};

pub(super) fn index_library(
    music_dir: &str,
    db_path: &str,
    file_pattern: Option<&str>,
    ignore: Option<&Vec<String>>,
    replace: Option<&std::collections::HashMap<String, String>>,
    dry_run: bool,
) {
    let music_dir = expand_tilde(music_dir);
    let db_path = expand_tilde(db_path);

    let mut glob_builder = GlobSetBuilder::new();
    if let Some(patterns) = ignore {
        for p in patterns {
            if let Ok(glob) = Glob::new(p) {
                glob_builder.add(glob);
            }
        }
    }
    let glob_set = glob_builder.build().unwrap();

    let entries: Vec<_> = walkdir::WalkDir::new(&music_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let rel = e.path().strip_prefix(&music_dir).unwrap_or(e.path());
            !glob_set.is_match(rel)
        })
        .collect();

    let mut conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
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
    )
    .expect("Failed to create table");
    let _ = conn.execute(
        "ALTER TABLE tracks ADD COLUMN added_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE tracks ADD COLUMN favorite INTEGER DEFAULT 0",
        [],
    );

    let tx = conn.transaction().expect("Failed to start transaction");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    println!("Indexing music files in: {}", music_dir);
    let bar = TickingBar::new(entries.len() as u64);
    let pb = Arc::clone(&bar.pb);

    let tracks: Vec<_> = entries
        .par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            let (artist, album, albumartist, title, year, genre) =
                match lofty::read_from_path(path) {
                    Ok(tagged_file) => {
                        let tag = tagged_file.primary_tag();
                        let artist = tag_str(tag, &ItemKey::TrackArtist);
                        let albumartist = tag_str(tag, &ItemKey::AlbumArtist);
                        let album = tag_str(tag, &ItemKey::AlbumTitle);
                        let title = tag_str(tag, &ItemKey::TrackTitle);
                        let year = tag
                            .and_then(|t| t.get_string(&ItemKey::Year))
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(0);
                        let genre = tag_str(tag, &ItemKey::Genre);
                        (artist, album, albumartist, title, year, genre)
                    }
                    Err(_) => {
                        pb.inc(1);
                        return None;
                    }
                };

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "mp3" | "flac" | "wav" | "m4a") {
                    let mut path_str = path.to_string_lossy().to_string();

                    if let Some(pattern) = file_pattern {
                        let new_rel = generate_path_from_pattern(
                            pattern,
                            &artist,
                            &albumartist,
                            &album,
                            &title,
                            ext,
                            replace,
                        );
                        let new_abs = Path::new(&music_dir).join(&new_rel);
                        if new_abs != path {
                            if dry_run {
                                println!(
                                    "[dry-run] Would move:\n  from: {}\n  to:   {}",
                                    path.display(),
                                    new_abs.display()
                                );
                            } else {
                                if let Some(parent) = new_abs.parent() {
                                    std::fs::create_dir_all(parent).ok();
                                }
                                std::fs::rename(path, &new_abs).ok();
                            }
                            path_str = new_abs.to_string_lossy().to_string();
                        }
                    }

                    let rel_path = Path::new(&path_str)
                        .strip_prefix(&music_dir)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or(path_str);

                    pb.inc(1);
                    return Some((rel_path, artist, albumartist, album, title, year, genre));
                }
            }
            pb.inc(1);
            None
        })
        .collect();

    bar.pb.finish_with_message("Metadata reading complete");
    drop(bar);

    println!("Inserting {} tracks into database…", tracks.len());
    let insert_bar = TickingBar::new(tracks.len() as u64);

    for (path_str, artist, albumartist, album, title, year, genre) in tracks {
        let result = tx.execute(
            "INSERT OR IGNORE INTO tracks (path, artist, albumartist, album, title, duration, year, genre, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![path_str, artist, albumartist, album, title, 0i64, year, genre, now],
        );
        if let Ok(1) = result {
            insert_bar.pb.set_message(format!("Added: {}", path_str));
        }
        insert_bar.pb.inc(1);
    }
    insert_bar.pb.finish_with_message("Database insertion complete");
    drop(insert_bar);

    println!("Checking for missing files…");
    let to_remove = collect_existing_paths_column(&tx, "tracks", &music_dir);
    for path in &to_remove {
        println!("Removing missing file from database: {}", path);
        tx.execute("DELETE FROM tracks WHERE path = ?1", [path]).ok();
    }
    if !to_remove.is_empty() {
        println!("Removed {} missing files", to_remove.len());
    }

    tx.commit().expect("Failed to commit transaction");
}

pub(super) fn index_playlists(music_dir: &str, db_path: &str) {
    let db_path = expand_tilde(db_path);
    let mut conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE
        )",
        [],
    )
    .expect("Failed to create playlists table");

    let tx = conn.transaction().expect("Failed to start transaction");

    for path in collect_existing_paths_column(&tx, "playlists", "") {
        println!("Removing missing playlist: {}", path);
        tx.execute("DELETE FROM playlists WHERE path = ?1", [&path]).ok();
    }

    println!("Indexing playlists in: {}", music_dir);

    let all_tracks: Vec<(String, String)> = {
        let tc = rusqlite::Connection::open(&db_path).expect("Failed to open database");
        let mut stmt = tc
            .prepare("SELECT title, path FROM tracks")
            .expect("Failed to prepare");
        let mut rows = stmt.query([]).expect("Failed to query");
        let mut v = Vec::new();
        while let Some(row) = rows.next().expect("row") {
            v.push((
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
            ));
        }
        v
    };

    for entry in walkdir::WalkDir::new(music_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(ext) = path.extension() else { continue };
        if ext != "m3u" && ext != "m3u8" {
            continue;
        }

        let path_str = path.to_string_lossy();
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        tx.execute(
            "INSERT OR IGNORE INTO playlists (name, path) VALUES (?1, ?2)",
            [&name as &dyn rusqlite::ToSql, &path_str],
        )
        .ok();

        if let Ok(content) = std::fs::read_to_string(path) {
            let playlist_dir = path.parent().unwrap_or_else(|| Path::new(""));
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let song_path = if Path::new(trimmed).is_absolute() {
                    PathBuf::from(trimmed)
                } else {
                    playlist_dir.join(trimmed)
                };
                if !song_path.exists() {
                    println!("Missing in playlist '{}': {}", name, song_path.display());

                    let fname = song_path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("");
                    let song_name = extract_song_name_from_filename(fname)
                        .unwrap_or_else(|| fname.to_string());
                    println!("  Suggested name: {}", song_name);

                    if !fname.is_empty() {
                        let mut suggestions: Vec<(f64, String)> = all_tracks
                            .iter()
                            .map(|(t, p)| (strsim::jaro(t, &song_name), p.clone()))
                            .collect();
                        suggestions.sort_by(|a, b| {
                            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        let top: Vec<_> = suggestions.into_iter().take(5).collect();

                        if let Some((top_score, top_path)) = top.first() {
                            if *top_score >= 0.9 {
                                println!(
                                    "  Auto-replacing with '{}' (similarity {:.3})",
                                    top_path, top_score
                                );
                                update_playlist_line(
                                    &path_str,
                                    &song_path.display().to_string(),
                                    top_path,
                                )
                                .ok();
                            } else {
                                let mut options: Vec<String> = top
                                    .iter()
                                    .map(|(s, p)| format!("({:.3}) {}", s, p))
                                    .collect();
                                options.push("Remove".to_string());
                                options.push("Skip".to_string());

                                match inquire::Select::new(
                                    &format!("Replacement for '{}':", fname),
                                    options.clone(),
                                )
                                .prompt()
                                {
                                    Ok(sel) if sel != "Skip" && sel != "Remove" => {
                                        let selected_path = sel
                                            .split_once(')')
                                            .map(|x| x.1.trim())
                                            .unwrap_or(&sel);
                                        update_playlist_line(
                                            &path_str,
                                            &song_path.display().to_string(),
                                            selected_path,
                                        )
                                        .ok();
                                    }
                                    Ok(sel) if sel == "Remove" => {
                                        update_playlist_line(
                                            &path_str,
                                            &song_path.display().to_string(),
                                            "",
                                        )
                                        .ok();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    tx.commit().expect("Failed to commit transaction");
}
