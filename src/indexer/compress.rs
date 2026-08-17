use crate::config::DEFAULT_DATABASE_PATH;
use crate::util::{expand_tilde, get_num_cpus};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::util::{abs_track_path, copy_lyrics_tag, Colorize, TickingBar};

fn export_playlists_for_compressed(
    conn: &rusqlite::Connection,
    music_dir: &str,
    output_dir: &str,
    format: &str,
) {
    let mut stmt = match conn.prepare("SELECT name, path FROM playlists") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to query playlists: {}", e);
            return;
        }
    };
    let playlists: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    if playlists.is_empty() {
        println!("No playlists found.");
        return;
    }
    println!("Found {} playlists to export", playlists.len());

    for (name, playlist_path) in playlists {
        let content = match std::fs::read_to_string(&playlist_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read playlist '{}': {}", name, e);
                continue;
            }
        };

        let playlist_dir = Path::new(&playlist_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));

        let mut updated_lines = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                updated_lines.push(line.to_string());
                continue;
            }
            let song_path = if Path::new(trimmed).is_absolute() {
                PathBuf::from(trimmed)
            } else {
                playlist_dir.join(trimmed)
            };
            match song_path.strip_prefix(music_dir) {
                Ok(rel) => {
                    let mut new_path = PathBuf::from(rel);
                    new_path.set_extension(format);
                    updated_lines.push(new_path.to_string_lossy().to_string());
                }
                Err(_) => {
                    eprintln!(
                        "Warning: '{}' in playlist '{}' is not under music dir, skipping",
                        song_path.display(),
                        name
                    );
                    updated_lines.push(line.to_string());
                }
            }
        }

        let out = PathBuf::from(output_dir).join(format!("{}.m3u8", name));
        if let Some(parent) = out.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create dir for playlist '{}': {}", name, e);
                continue;
            }
        }
        match std::fs::write(&out, updated_lines.join("\n") + "\n") {
            Ok(_) => println!("  ✓ {}", name),
            Err(e) => eprintln!("  ✗ {}: {}", name, e),
        }
    }
}

fn create_output_db(src_db_path: &str, output_dir: &str, pairs: &[(String, String)]) {
    let db_filename = Path::new(src_db_path)
        .file_name()
        .unwrap_or_else(|| Path::new(DEFAULT_DATABASE_PATH).file_name().unwrap());
    let out_db = PathBuf::from(output_dir).join(db_filename);
    if let Err(e) = std::fs::copy(src_db_path, &out_db) {
        eprintln!("Failed to create output database: {}", e);
        return;
    }

    let mut conn = match rusqlite::Connection::open(&out_db) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to open output database: {}", e);
            return;
        }
    };

    let tx = conn.transaction().expect("begin transaction");
    {
        let mut update = tx.prepare("UPDATE tracks SET path = ? WHERE path = ?").expect("prepare update");
        for (src_rel, out_rel) in pairs {
            update.execute(rusqlite::params![out_rel, src_rel]).ok();
        }
    }
    {
        tx.execute_batch("CREATE TEMP TABLE _keep (path TEXT PRIMARY KEY)").expect("create temp table");
        let mut insert = tx.prepare("INSERT OR IGNORE INTO _keep VALUES (?)").expect("prepare insert");
        for (_, out_rel) in pairs {
            insert.execute([out_rel]).ok();
        }
        tx.execute_batch("DELETE FROM tracks WHERE path NOT IN (SELECT path FROM _keep)").expect("delete missing");
    }
    tx.commit().expect("commit");

    println!("Output database written to {}", out_db.display());
}

pub(super) fn compress_tracks(
    music_dir: &str,
    db_path: &str,
    output_dir: &str,
    format: &str,
    bitrate: &str,
    jobs: Option<usize>,
    force: bool,
    query: Option<String>,
) {
    let music_dir = expand_tilde(music_dir);
    let output_dir = expand_tilde(output_dir);
    let db_path_expanded = expand_tilde(db_path);

    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("{}", "Error: ffmpeg is not installed or not in PATH".red());
        return;
    }

    if let Some(n) = jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .ok();
    }

    let conn = rusqlite::Connection::open(&db_path_expanded).expect("Failed to open database");

    let (query_sql, pattern) = if let Some(ref q) = query {
        (
            "SELECT path FROM tracks WHERE album LIKE ?1 OR artist LIKE ?1 OR title LIKE ?1",
            Some(format!("%{}%", q)),
        )
    } else {
        ("SELECT path FROM tracks", None)
    };

    let mut stmt = conn.prepare(query_sql).expect("prepare");
    let paths: Vec<String> = match &pattern {
        Some(p) => stmt
            .query_map([p], |row| row.get(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect(),
        None => stmt
            .query_map([], |row| row.get(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect(),
    };
    drop(stmt);

    if paths.is_empty() {
        println!("{}", "No tracks found to compress.".yellow());
        return;
    }

    // Deduplicate paths that differ only in case: if two source files would produce
    // the same case-insensitive output path, keep only the highest-quality source
    // (FLAC > M4A > MP3) to avoid Syncthing conflicts on case-insensitive filesystems.
    fn source_quality_rank(path: &str) -> u8 {
        match Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "flac" => 0,
            "m4a" => 1,
            "mp3" => 2,
            _ => 100,
        }
    }
    let paths = {
        let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for src_path in paths {
            let mut out = PathBuf::from(&src_path);
            out.set_extension(format);
            let key = out.to_string_lossy().to_lowercase();
            let better = map
                .get(&key)
                .map_or(true, |existing| source_quality_rank(&src_path) < source_quality_rank(existing));
            if better {
                map.insert(key, src_path);
            }
        }
        map.into_values().collect::<Vec<_>>()
    };

    let thread_count = jobs.unwrap_or_else(get_num_cpus);
    println!(
        "Compressing {} tracks → {} as {} @ {} ({} threads)…",
        paths.len(),
        output_dir,
        format,
        bitrate,
        thread_count
    );

    let multi = Arc::new(MultiProgress::new());
    let main_pb_raw = Arc::new(multi.add(ProgressBar::new(paths.len() as u64)));
    main_pb_raw.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) - {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    let worker_count = thread_count.min(8);
    let worker_bars: Vec<Arc<ProgressBar>> = (0..worker_count)
        .map(|i| {
            let pb = multi.add(ProgressBar::new_spinner());
            pb.set_style(
                ProgressStyle::with_template(&format!("  Worker {}: {{spinner}} {{msg}}", i + 1))
                    .unwrap(),
            );
            pb.set_message("Idle");
            Arc::new(pb)
        })
        .collect();

    let compressed = Arc::new(Mutex::new(0usize));
    let skipped = Arc::new(Mutex::new(0usize));
    let failed = Arc::new(Mutex::new(0usize));
    let failed_files: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    // (src_relative_path, out_relative_path) for tracks present in the output
    let output_pairs: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    let worker_bars_ticker = worker_bars.clone();
    let bar = TickingBar::from_bar(Arc::clone(&main_pb_raw));
    let workers_running = Arc::clone(&bar.running);
    let worker_ticker = {
        let running = Arc::clone(&workers_running);
        thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                for wb in &worker_bars_ticker {
                    wb.tick();
                }
                thread::sleep(Duration::from_millis(100));
            }
        })
    };

    let main_pb = Arc::clone(&main_pb_raw);
    let worker_bars_clone = worker_bars.clone();
    let output_pairs_clone = Arc::clone(&output_pairs);

    paths.par_iter().for_each(|src_path| {
        let abs_src_str = abs_track_path(&music_dir, src_path);
        let src = Path::new(&abs_src_str);
        if !src.exists() {
            *failed.lock().unwrap() += 1;
            failed_files.lock().unwrap().push(src_path.clone());
            main_pb.inc(1);
            return;
        }

        let rel = src.strip_prefix(&music_dir).unwrap_or(src);
        let mut out_path = PathBuf::from(&output_dir);
        out_path.push(rel);
        out_path.set_extension(format);

        if let Some(parent) = out_path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                *failed.lock().unwrap() += 1;
                failed_files.lock().unwrap().push(src_path.clone());
                main_pb.inc(1);
                return;
            }
        }

        if !force && out_path.exists() {
            *skipped.lock().unwrap() += 1;
            let out_rel = out_path.strip_prefix(&output_dir).unwrap_or(&out_path).to_string_lossy().to_string();
            output_pairs_clone.lock().unwrap().push((src_path.clone(), out_rel));
            main_pb.inc(1);
            return;
        }

        let fname = src.file_name().unwrap_or_default().to_string_lossy().to_string();
        let widx = rayon::current_thread_index().unwrap_or(0) % worker_count;
        let wb = &worker_bars_clone[widx];
        wb.set_message(format!("🎵 {}", fname));

        let mut cmd = std::process::Command::new("ffmpeg");
        cmd.arg("-i").arg(&abs_src_str);
        match format {
            "mp3" => {
                cmd.arg("-c:a").arg("libmp3lame");
            }
            "aac" | "m4a" => {
                cmd.arg("-c:a").arg("aac");
            }
            "opus" => {
                cmd.arg("-c:a").arg("libopus");
            }
            _ => {
                cmd.arg("-c:a").arg("libmp3lame");
            }
        }
        cmd.arg("-b:a")
            .arg(bitrate)
            .arg("-map")
            .arg("0")
            .arg("-c:v")
            .arg("copy")
            .arg("-y")
            .arg(&out_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        match cmd.status() {
            Ok(s) if s.success() => {
                copy_lyrics_tag(&out_path);
                *compressed.lock().unwrap() += 1;
                wb.set_message(format!("✓ {}", fname));
                let out_rel = out_path.strip_prefix(&output_dir).unwrap_or(&out_path).to_string_lossy().to_string();
                output_pairs_clone.lock().unwrap().push((src_path.clone(), out_rel));
            }
            _ => {
                *failed.lock().unwrap() += 1;
                failed_files.lock().unwrap().push(src_path.clone());
                wb.set_message(format!("✗ {}", fname));
            }
        }
        main_pb.inc(1);

        let c = *compressed.lock().unwrap();
        let sk = *skipped.lock().unwrap();
        let f = *failed.lock().unwrap();
        main_pb.set_message(format!("✓ {} ⊘ {} ✗ {}", c, sk, f));
    });

    drop(bar);
    worker_ticker.join().ok();
    main_pb_raw.finish_with_message("Compression complete");
    for wb in &worker_bars {
        wb.finish_and_clear();
    }

    let c = *compressed.lock().unwrap();
    let sk = *skipped.lock().unwrap();
    let f = *failed.lock().unwrap();
    let fl = failed_files.lock().unwrap();

    println!("\nSummary:");
    println!("  Compressed: {}", c.to_string().green());
    println!("  Skipped:    {}", sk.to_string().yellow());
    println!("  Failed:     {}", f.to_string().red());
    if !fl.is_empty() {
        println!("\nFailed files:");
        for p in fl.iter() {
            println!("  {}", p.red());
        }
    }

    let pairs = output_pairs.lock().unwrap();
    create_output_db(&db_path_expanded, &output_dir, &pairs);
    drop(pairs);

    println!("\nExporting playlists…");
    export_playlists_for_compressed(&conn, &music_dir, &output_dir, format);
}
