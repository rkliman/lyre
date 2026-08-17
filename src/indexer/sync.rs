use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::util::Colorize;

struct TrackRow {
    artist: String,
    album: String,
    albumartist: String,
    title: String,
    duration: i64,
    year: i32,
    genre: String,
    added_at: i64,
    favorite: i64,
}

fn load_tracks(conn: &rusqlite::Connection) -> HashMap<String, TrackRow> {
    let mut stmt = conn
        .prepare(
            "SELECT path, artist, album, albumartist, title, duration, year, genre, added_at, favorite
             FROM tracks",
        )
        .expect("prepare source query");
    stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            TrackRow {
                artist: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                album: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                albumartist: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                title: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                duration: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                year: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                genre: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                added_at: row.get::<_, Option<i64>>(8)?.unwrap_or(0),
                favorite: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            },
        ))
    })
    .expect("query source")
    .filter_map(Result::ok)
    .collect()
}

fn open_or_create_dest_db(dst_db: &str) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(dst_db).expect("Failed to open destination database");
    conn.execute_batch(
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
        );
        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL
        );",
    )
    .expect("Failed to initialize destination database");
    conn
}

fn progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    pb
}

fn sync_playlists(src_music_dir: &str, dst_music_dir: &str, dry_run: bool, no_delete: bool) {
    use std::ffi::OsStr;

    let playlist_files = |dir: &str| -> Vec<PathBuf> {
        walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| matches!(e.path().extension().and_then(OsStr::to_str), Some("m3u") | Some("m3u8")))
            .map(|e| e.into_path())
            .collect()
    };

    let src_playlists = playlist_files(src_music_dir);
    let dst_rel: HashSet<PathBuf> = playlist_files(dst_music_dir)
        .iter()
        .filter_map(|p| p.strip_prefix(dst_music_dir).ok().map(|r| r.to_path_buf()))
        .collect();

    let to_copy: Vec<&PathBuf> = src_playlists.iter()
        .filter(|p| p.strip_prefix(src_music_dir).ok().map(|r| !dst_rel.contains(r)).unwrap_or(false))
        .collect();
    let to_update: Vec<&PathBuf> = src_playlists.iter()
        .filter(|p| p.strip_prefix(src_music_dir).ok().map(|r| dst_rel.contains(r)).unwrap_or(false))
        .collect();

    let src_rel: HashSet<PathBuf> = src_playlists.iter()
        .filter_map(|p| p.strip_prefix(src_music_dir).ok().map(|r| r.to_path_buf()))
        .collect();
    let to_remove: Vec<PathBuf> = if no_delete {
        vec![]
    } else {
        playlist_files(dst_music_dir)
            .into_iter()
            .filter(|p| p.strip_prefix(dst_music_dir).ok().map(|r| !src_rel.contains(r)).unwrap_or(false))
            .collect()
    };

    println!(
        "Playlist sync: {} to copy  {}  {} to update",
        to_copy.len().to_string().green(),
        if no_delete { "(deletions skipped)".to_string() } else { format!("{} to remove", to_remove.len().to_string().red()) },
        to_update.len().to_string().yellow(),
    );

    if dry_run {
        for p in &to_copy { println!("  + {}", p.display()); }
        for p in &to_remove { println!("  - {}", p.display()); }
        return;
    }

    for src_path in to_copy.iter().chain(to_update.iter()) {
        if let Ok(rel) = src_path.strip_prefix(src_music_dir) {
            let dst_path = PathBuf::from(dst_music_dir).join(rel);
            if let Some(parent) = dst_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::copy(src_path, &dst_path) {
                eprintln!("Failed to copy playlist {}: {}", src_path.display(), e);
            }
        }
    }

    for dst_path in &to_remove {
        if let Err(e) = std::fs::remove_file(dst_path) {
            eprintln!("Failed to remove playlist {}: {}", dst_path.display(), e);
        }
    }
}

pub(super) fn sync_libraries(src_db: &str, dst_db: &str, dry_run: bool, no_delete: bool) {
    let src_db = crate::util::expand_tilde(src_db);
    let dst_db = crate::util::expand_tilde(dst_db);

    let src_music_dir = Path::new(&src_db)
        .parent()
        .expect("src db has no parent directory")
        .to_string_lossy()
        .to_string();
    let dst_music_dir = Path::new(&dst_db)
        .parent()
        .expect("dst db has no parent directory")
        .to_string_lossy()
        .to_string();

    if !Path::new(&src_db).exists() {
        eprintln!("{}", format!("Source database not found: {}", src_db).red());
        return;
    }

    let src_conn = rusqlite::Connection::open(&src_db).expect("Failed to open source database");
    let src_tracks = load_tracks(&src_conn);
    let src_paths: HashSet<&str> = src_tracks.keys().map(String::as_str).collect();

    let mut dst_conn = open_or_create_dest_db(&dst_db);
    let dst_tracks = load_tracks(&dst_conn);
    let dst_paths: HashSet<&str> = dst_tracks.keys().map(String::as_str).collect();

    let to_add: Vec<&str> = src_paths.difference(&dst_paths).copied().collect();
    let to_remove: Vec<&str> = if no_delete {
        vec![]
    } else {
        dst_paths.difference(&src_paths).copied().collect()
    };
    let to_update: Vec<&str> = src_paths.intersection(&dst_paths).copied().collect();

    println!(
        "Sync plan: {} to add  {}  {} unchanged",
        to_add.len().to_string().green(),
        if no_delete {
            format!("(deletions skipped)")
        } else {
            format!("{} to remove", to_remove.len().to_string().red())
        },
        to_update.len().to_string().yellow(),
    );

    if dry_run {
        if !to_add.is_empty() {
            println!("\nWould add:");
            for p in &to_add {
                println!("  + {}", p);
            }
        }
        if !to_remove.is_empty() {
            println!("\nWould remove:");
            for p in &to_remove {
                println!("  - {}", p);
            }
        }
        return;
    }

    let mut copied = 0usize;
    let mut removed = 0usize;
    let mut failed = 0usize;

    // Copy new files
    if !to_add.is_empty() {
        println!("\nCopying {} new tracks…", to_add.len());
        let pb = progress_bar(to_add.len() as u64);
        for rel in &to_add {
            let src_file = PathBuf::from(&src_music_dir).join(rel);
            let dst_file = PathBuf::from(&dst_music_dir).join(rel);
            if let Some(parent) = dst_file.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("Failed to create dir for {}: {}", rel, e);
                    failed += 1;
                    pb.inc(1);
                    continue;
                }
            }
            match std::fs::copy(&src_file, &dst_file) {
                Ok(_) => copied += 1,
                Err(e) => {
                    eprintln!("Failed to copy {}: {}", rel, e);
                    failed += 1;
                }
            }
            pb.inc(1);
        }
        pb.finish_and_clear();
    }

    // Delete removed files
    if !to_remove.is_empty() {
        println!("Removing {} tracks…", to_remove.len());
        let pb = progress_bar(to_remove.len() as u64);
        for rel in &to_remove {
            let dst_file = PathBuf::from(&dst_music_dir).join(rel);
            if let Err(e) = std::fs::remove_file(&dst_file) {
                eprintln!("Failed to remove {}: {}", rel, e);
            } else {
                removed += 1;
                // Remove empty parent directories up to dst_music_dir
                let mut dir = dst_file.parent();
                while let Some(d) = dir {
                    if d == Path::new(&dst_music_dir) {
                        break;
                    }
                    if std::fs::read_dir(d).map(|mut e| e.next().is_none()).unwrap_or(false) {
                        let _ = std::fs::remove_dir(d);
                    }
                    dir = d.parent();
                }
            }
            pb.inc(1);
        }
        pb.finish_and_clear();
    }

    // Update destination DB in a single transaction
    println!("Updating destination database…");
    let tx = dst_conn.transaction().expect("begin transaction");
    {
        let mut insert = tx
            .prepare(
                "INSERT OR IGNORE INTO tracks
                 (path, artist, album, albumartist, title, duration, year, genre, added_at, favorite)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .expect("prepare insert");
        for rel in &to_add {
            // Only insert tracks that were successfully copied
            let dst_file = PathBuf::from(&dst_music_dir).join(rel);
            if !dst_file.exists() {
                continue;
            }
            if let Some(row) = src_tracks.get(*rel) {
                insert
                    .execute(rusqlite::params![
                        rel,
                        row.artist,
                        row.album,
                        row.albumartist,
                        row.title,
                        row.duration,
                        row.year,
                        row.genre,
                        row.added_at,
                        row.favorite,
                    ])
                    .ok();
            }
        }
    }
    {
        let mut delete = tx
            .prepare("DELETE FROM tracks WHERE path = ?")
            .expect("prepare delete");
        for rel in &to_remove {
            delete.execute([rel]).ok();
        }
    }
    {
        let mut update = tx
            .prepare(
                "UPDATE tracks SET artist=?, album=?, albumartist=?, title=?,
                 duration=?, year=?, genre=?
                 WHERE path=?",
            )
            .expect("prepare update");
        for rel in &to_update {
            if let Some(row) = src_tracks.get(*rel) {
                update
                    .execute(rusqlite::params![
                        row.artist,
                        row.album,
                        row.albumartist,
                        row.title,
                        row.duration,
                        row.year,
                        row.genre,
                        rel,
                    ])
                    .ok();
            }
        }
    }
    tx.commit().expect("commit");

    println!("\nSummary:");
    println!("  Copied:  {}", copied.to_string().green());
    println!("  Removed: {}", removed.to_string().red());
    if failed > 0 {
        println!("  Failed:  {}", failed.to_string().red());
    }

    println!();
    sync_playlists(&src_music_dir, &dst_music_dir, dry_run, no_delete);
}
