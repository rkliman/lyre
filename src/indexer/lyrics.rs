use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::lrclib::resolve_lyrics;
use super::util::{abs_track_path, is_synced_lyrics, normalize_text, open_db, write_lyrics_tag, Colorize, TickingBar};

struct LyricsCandidate {
    path: String,
    /// Album artist if present, else track artist — matches
    /// `Track::display_artist()` so the CLI and the app query lrclib with the
    /// same key and print the same name.
    artist: String,
    album: String,
    title: String,
    duration: i64,
    has_lyrics: bool,
    #[allow(dead_code)]
    is_synced: bool,
}

pub(super) fn add_lyrics(
    db_path: &str,
    music_dir: &str,
    query: Option<String>,
    overwrite: bool,
    dry_run: bool,
) {
    let conn = open_db(db_path);

    let (query_sql, pattern) = if let Some(ref q) = query {
        (
            "SELECT path, artist, albumartist, album, title, duration FROM tracks \
             WHERE album LIKE ?1 OR artist LIKE ?1 OR title LIKE ?1",
            Some(format!("%{}%", q)),
        )
    } else {
        (
            "SELECT path, artist, albumartist, album, title, duration FROM tracks",
            None,
        )
    };

    let mut stmt = conn.prepare(query_sql).expect("prepare");
    let mut rows = if let Some(ref p) = pattern {
        stmt.query([p]).expect("query")
    } else {
        stmt.query([]).expect("query")
    };

    let mut raw = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        raw.push((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
            row.get::<_, String>(4).unwrap_or_default(),
            row.get::<_, i64>(5).unwrap_or_default(),
        ));
    }
    drop(rows);
    drop(stmt);

    if raw.is_empty() {
        println!("{}", "No tracks found.".yellow());
        return;
    }

    println!("Checking existing lyrics tags for {} tracks…", raw.len());
    let mut candidates = Vec::new();
    for (path, artist, albumartist, album, title, duration) in raw {
        let abs = abs_track_path(music_dir, &path);
        let p = Path::new(&abs);
        if !p.exists() {
            continue;
        }
        let (has_lyrics, is_synced) = match crate::lyrics::extract_lyrics(&abs)
            .filter(|s| !s.trim().is_empty())
        {
            Some(lyrics) => {
                let synced = is_synced_lyrics(&lyrics);
                (true, synced)
            }
            None => (false, false),
        };
        if !has_lyrics || (!is_synced && overwrite) {
            let artist = if albumartist.trim().is_empty() {
                artist
            } else {
                albumartist
            };
            candidates.push(LyricsCandidate {
                path: abs,
                artist,
                album,
                title,
                duration,
                has_lyrics,
                is_synced,
            });
        }
    }

    if candidates.is_empty() {
        println!("{}", "No tracks need lyrics updates.".green());
        return;
    }

    println!("\n{} tracks will be updated:", candidates.len());
    for c in &candidates {
        let reason = if !c.has_lyrics {
            "missing lyrics"
        } else {
            "unsynced — will overwrite"
        };
        println!("  {} - {} ({})", c.artist.cyan(), c.title, reason.yellow());
    }

    if dry_run {
        println!(
            "\n{}",
            "[dry-run] No files modified. Re-run without --dry-run to apply.".yellow()
        );
        return;
    }

    let bar = TickingBar::new(candidates.len() as u64);
    let updated = Arc::new(Mutex::new(0usize));
    let not_found = Arc::new(Mutex::new(0usize));
    let failed = Arc::new(Mutex::new(0usize));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let client = reqwest::Client::builder()
        .user_agent("lyre/0.1 (https://github.com/rkliman/lyre)")
        .build()
        .expect("Failed to build HTTP client");

    // Cap concurrency: lrclib.net rate-limits aggressive parallel callers,
    // which silently turns into 429s and looks like "not found" to the user.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Failed to build lyrics thread pool");

    let pb = Arc::clone(&bar.pb);
    pool.install(|| candidates.par_iter().for_each(|c| {
        let path = Path::new(&c.path);
        let dur = if c.duration > 0 { Some(c.duration as u32) } else { None };

        let result = rt.block_on(resolve_lyrics(
            &client,
            &normalize_text(&c.artist),
            &normalize_text(&c.title),
            if c.album.is_empty() { None } else { Some(c.album.as_str()) },
            dur,
        ));

        match result {
            Ok(Some(resolution)) => {
                let kind = resolution.kind();
                if write_lyrics_tag(path, resolution.tag_text(), &c.artist, &c.title, &pb, kind) {
                    *updated.lock().unwrap() += 1;
                } else {
                    *failed.lock().unwrap() += 1;
                }
            }
            Ok(None) => {
                pb.set_message(format!("⊘ not found: {}", c.title));
                *not_found.lock().unwrap() += 1;
            }
            Err(e) => {
                eprintln!("Error fetching lyrics for '{}': {}", c.title, e);
                *failed.lock().unwrap() += 1;
            }
        }
        pb.inc(1);
    }));

    bar.pb.finish_with_message("Lyrics update complete");
    drop(bar);

    println!("\nSummary:");
    println!("  Updated:          {}", updated.lock().unwrap().to_string().green());
    println!("  No lyrics found:  {}", not_found.lock().unwrap().to_string().yellow());
    println!("  Failed:           {}", failed.lock().unwrap().to_string().red());
}
