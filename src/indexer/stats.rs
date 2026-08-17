use std::path::Path;

use super::util::{abs_track_path, get_dir_size, get_duration_with_lofty, open_db, TickingBar};

pub(super) fn get_stats(music_dir: &str, db_path: &str) {
    let conn = open_db(db_path);

    let total_tracks: i64 = conn
        .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
        .unwrap_or(0);
    let total_artists: i64 = conn
        .query_row("SELECT COUNT(DISTINCT artist) FROM tracks", [], |r| r.get(0))
        .unwrap_or(0);
    let total_albums: i64 = conn
        .query_row("SELECT COUNT(DISTINCT album) FROM tracks", [], |r| r.get(0))
        .unwrap_or(0);

    let mut stmt = conn
        .prepare("SELECT id, path FROM tracks WHERE duration = 0")
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut rows_vec = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        rows_vec.push((
            row.get::<_, i64>(0).unwrap_or(0),
            row.get::<_, String>(1).unwrap_or_default(),
        ));
    }
    drop(rows);
    drop(stmt);

    let bar = TickingBar::new(rows_vec.len() as u64);
    for (id, path) in rows_vec {
        let abs = abs_track_path(music_dir, &path);
        let dur = get_duration_with_lofty(Path::new(&abs));
        if dur > 0 {
            conn.execute(
                "UPDATE tracks SET duration = ?1 WHERE id = ?2",
                rusqlite::params![dur, id],
            )
            .ok();
        }
        bar.pb.inc(1);
        bar.pb.set_message(path);
    }
    bar.pb.finish_with_message("Duration update complete");
    drop(bar);

    let total_duration: f64 = conn
        .query_row("SELECT SUM(duration) FROM tracks", [], |r| r.get(0))
        .unwrap_or(0.0);

    fn fmt_dur(secs: f64) -> String {
        let months = secs / 2_592_000.0;
        let weeks = secs / 604_800.0;
        let days = secs / 86_400.0;
        let hours = secs / 3_600.0;
        let mins = secs / 60.0;
        if months > 1.0 {
            format!("{:.2} months", months)
        } else if weeks > 1.0 {
            format!("{:.2} weeks", weeks)
        } else if days > 1.0 {
            format!("{:.2} days", days)
        } else if hours > 1.0 {
            format!("{:.2} hours", hours)
        } else if mins > 1.0 {
            format!("{:.2} minutes", mins)
        } else {
            format!("{:.2} seconds", secs)
        }
    }

    let folder_size = crate::util::format_bytes(get_dir_size(music_dir).unwrap_or(0));
    println!("Total tracks:  {}", total_tracks);
    println!("Total artists: {}", total_artists);
    println!("Total albums:  {}", total_albums);
    println!("Total size:    {}", folder_size);
    println!("Total time:    {}", fmt_dur(total_duration));

    println!("\nTracks by Year:");
    let mut stmt = conn
        .prepare(
            "SELECT year, COUNT(*) FROM tracks \
             WHERE year IS NOT NULL AND year > 0 GROUP BY year ORDER BY year",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut year_counts = Vec::new();
    let mut max_count = 0i64;
    while let Some(row) = rows.next().expect("row") {
        let year: i64 = row.get(0).unwrap_or(0);
        let count: i64 = row.get(1).unwrap_or(0);
        if count > max_count {
            max_count = count;
        }
        year_counts.push((year, count));
    }
    for (year, count) in year_counts {
        let bar_len = if max_count > 0 {
            (count * 40 / max_count) as usize
        } else {
            0
        };
        println!("{:4}: {:4} {}", year, count, "█".repeat(bar_len));
    }
}
