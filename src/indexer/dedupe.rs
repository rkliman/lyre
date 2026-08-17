use std::path::Path;

use super::util::{abs_track_path, open_db, Colorize};

pub(super) fn find_duplicates(db_path: &str, music_dir: &str, fix: bool) {
    let conn = open_db(db_path);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kept_duplicates (
            id INTEGER PRIMARY KEY,
            artist TEXT NOT NULL,
            title TEXT NOT NULL,
            UNIQUE(artist, title)
        )",
        [],
    )
    .expect("Failed to create kept_duplicates table");

    let mut stmt = conn
        .prepare(
            "SELECT MIN(artist), MIN(title), COUNT(*) as count FROM tracks \
             WHERE artist != '' AND title != '' \
             GROUP BY LOWER(artist), LOWER(title) HAVING count > 1",
        )
        .expect("Failed to prepare statement");

    let mut rows = stmt.query([]).expect("Failed to execute query");
    let mut found = false;

    while let Some(row) = rows.next().expect("row") {
        found = true;
        let artist: String = row.get(0).unwrap_or_default();
        let title: String = row.get(1).unwrap_or_default();
        let count: i32 = row.get(2).unwrap_or(0);

        let artist_lower = artist.to_lowercase();
        let title_lower = title.to_lowercase();

        let is_kept: bool = conn
            .query_row(
                "SELECT 1 FROM kept_duplicates WHERE artist = ?1 AND title = ?2",
                [&artist_lower, &title_lower],
                |_| Ok(true),
            )
            .unwrap_or(false);

        let keep_tag = if is_kept { "[Keep All] ".green() } else { "".to_string() };
        println!(
            "{}{}",
            keep_tag,
            format!("{} - {} (x{})", artist, title, count).cyan()
        );

        let mut path_stmt = conn
            .prepare("SELECT id, path FROM tracks WHERE LOWER(artist) = ?1 AND LOWER(title) = ?2")
            .expect("Failed to prepare");
        let mut path_rows = path_stmt.query([&artist_lower, &title_lower]).expect("query");
        let mut paths: Vec<(i64, String)> = Vec::new();
        while let Some(pr) = path_rows.next().expect("row") {
            let id: i64 = pr.get(0).unwrap_or(0);
            let path: String = pr.get(1).unwrap_or_default();
            println!("  {}", path);
            paths.push((id, path));
        }

        if fix && paths.len() > 1 && !is_kept {
            let mut options = vec!["Skip".to_string(), "Keep both".to_string()];
            options.extend(paths.iter().map(|(_, p)| p.clone()));
            match inquire::Select::new(
                &format!("Keep which copy of '{} - {}'?", artist, title),
                options,
            )
            .prompt()
            {
                Ok(sel) if sel != "Skip" && sel != "Keep both" => {
                    for (id, path) in &paths {
                        if path != &sel {
                            conn.execute("DELETE FROM tracks WHERE id = ?1", [id]).ok();
                            let abs = abs_track_path(music_dir, path);
                            match std::fs::remove_file(&abs) {
                                Ok(_) => println!("  Deleted: {}", path),
                                Err(e) => eprintln!("  Failed to delete '{}': {}", path, e),
                            }
                        }
                    }
                }
                Ok(sel) if sel == "Keep both" => {
                    conn.execute(
                        "INSERT OR IGNORE INTO kept_duplicates (artist, title) VALUES (?1, ?2)",
                        [&artist_lower, &title_lower],
                    )
                    .ok();
                    println!("  Keeping all copies (won't show again)");
                }
                _ => println!("  Skipped"),
            }
        }
    }

    if !found {
        println!("{}", "No duplicate tracks found.".green());
    }

    println!("\nTracks with lower-quality duplicates (FLAC > M4A > MP3):");
    let mut stmt = conn
        .prepare(
            "SELECT MIN(artist), MIN(title), GROUP_CONCAT(path) FROM tracks \
             WHERE artist != '' AND title != '' \
             GROUP BY LOWER(artist), LOWER(title) HAVING COUNT(*) > 1",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut found_quality = false;

    while let Some(row) = rows.next().expect("row") {
        let artist: String = row.get(0).unwrap_or_default();
        let title: String = row.get(1).unwrap_or_default();
        let paths_str: String = row.get(2).unwrap_or_default();
        let files: Vec<&str> = paths_str.split(',').collect();

        fn quality_rank(ext: &str) -> u8 {
            match ext.to_lowercase().as_str() {
                "flac" => 1,
                "m4a" => 2,
                "mp3" => 3,
                _ => 100,
            }
        }

        let mut qualities: Vec<(u8, &str)> = files
            .iter()
            .filter_map(|p| {
                Path::new(p)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| (quality_rank(ext), *p))
            })
            .collect();
        qualities.sort_by_key(|q| q.0);

        if qualities.len() > 1 && qualities[0].0 < qualities[1].0 {
            found_quality = true;
            println!("{}", format!("{} - {}", artist, title).cyan());
            for (rank, path) in &qualities {
                let label = match rank {
                    1 => "FLAC",
                    2 => "M4A",
                    3 => "MP3",
                    _ => "OTHER",
                };
                println!("  [{}] {}", label, path);
            }
        }
    }

    if !found_quality {
        println!("{}", "No lower-quality duplicates found.".green());
    }
}
