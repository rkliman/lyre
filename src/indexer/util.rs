use crate::util::expand_tilde;
use deunicode::deunicode;
use indicatif::{ProgressBar, ProgressStyle};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::ItemKey;
use lofty::tag::TagExt;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ── ANSI colour helpers ───────────────────────────────────────────────────────

macro_rules! color_fn {
    ($name:ident, $code:literal) => {
        fn $name(&self) -> String {
            format!("\x1b[{}m{}\x1b[0m", $code, self)
        }
    };
}

pub(super) trait Colorize {
    fn red(&self) -> String;
    fn green(&self) -> String;
    fn yellow(&self) -> String;
    fn cyan(&self) -> String;
    fn bold(&self) -> String;
    fn underline(&self) -> String;
}

impl Colorize for str {
    color_fn!(red, "31");
    color_fn!(green, "32");
    color_fn!(yellow, "33");
    color_fn!(cyan, "36");
    color_fn!(bold, "1");
    color_fn!(underline, "4");
}

// ── Progress bar ──────────────────────────────────────────────────────────────

pub(super) fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("##-")
}

pub(super) struct TickingBar {
    pub pb: Arc<ProgressBar>,
    pub running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl TickingBar {
    pub(super) fn new(len: u64) -> Self {
        let pb = Arc::new(ProgressBar::new(len));
        pb.set_style(bar_style());
        Self::from_bar(pb)
    }

    pub(super) fn from_bar(pb: Arc<ProgressBar>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let pb_clone = Arc::clone(&pb);
        let handle = thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                pb_clone.tick();
                thread::sleep(Duration::from_millis(100));
            }
        });
        Self {
            pb,
            running,
            handle: Some(handle),
        }
    }
}

impl Drop for TickingBar {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

// ── Misc helpers ──────────────────────────────────────────────────────────────

pub(super) fn get_dir_size(path: &str) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

pub(super) fn normalize_text(s: &str) -> String {
    deunicode(s)
}

pub(super) fn write_csv_row<W: std::io::Write>(writer: &mut W, fields: &[&str]) -> std::io::Result<()> {
    let escaped: Vec<String> = fields
        .iter()
        .map(|f| {
            if f.contains(',') || f.contains('"') || f.contains('\n') {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.to_string()
            }
        })
        .collect();
    writeln!(writer, "{}", escaped.join(","))
}

// ── DB helpers ────────────────────────────────────────────────────────────────

pub(super) fn open_db(db_path: &str) -> rusqlite::Connection {
    let path = expand_tilde(db_path);
    rusqlite::Connection::open(&path).expect("Failed to open database")
}

pub(super) fn query_triples(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Vec<(String, String, String)> {
    let mut stmt = conn.prepare(sql).expect("Failed to prepare statement");
    stmt.query_map(params, |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
        ))
    })
    .expect("Failed to execute query")
    .filter_map(Result::ok)
    .collect()
}

pub(super) fn abs_track_path(base_dir: &str, path: &str) -> String {
    Path::new(base_dir).join(path).to_string_lossy().into_owned()
}

pub(super) fn collect_existing_paths_column(conn: &rusqlite::Connection, table: &str, base_dir: &str) -> Vec<String> {
    let sql = format!("SELECT path FROM {}", table);
    let mut stmt = conn.prepare(&sql).expect("Failed to prepare select");
    let mut rows = stmt.query([]).expect("Failed to query rows");
    let mut missing = Vec::new();
    while let Some(row) = rows.next().expect("Failed to fetch row") {
        let path: String = row.get(0).expect("Failed to get path");
        let abs = abs_track_path(base_dir, &path);
        if !Path::new(&abs).exists() {
            missing.push(path);
        }
    }
    missing
}

// ── File-naming helpers ───────────────────────────────────────────────────────

pub(super) fn sanitize_filename_component(s: &str, replacements: Option<&HashMap<String, String>>) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        let mut replaced = false;
        if let Some(map) = replacements {
            for (from, to) in map {
                if from.chars().count() == 1 && c == from.chars().next().unwrap() {
                    result.push_str(to);
                    replaced = true;
                    break;
                }
            }
        }
        if !replaced {
            result.push(c);
        }
    }
    result
}

pub(super) fn generate_path_from_pattern(
    pattern: &str,
    artist: &str,
    albumartist: &str,
    album: &str,
    title: &str,
    ext: &str,
    replacements: Option<&HashMap<String, String>>,
) -> String {
    let artist_s = sanitize_filename_component(artist, replacements);
    let albumartist_s = if albumartist.trim().is_empty()
        || albumartist.trim().eq_ignore_ascii_case("Various Artists")
    {
        sanitize_filename_component(artist, replacements)
    } else {
        sanitize_filename_component(albumartist, replacements)
    };
    let album_s = sanitize_filename_component(album, replacements);
    let title_s = sanitize_filename_component(title, replacements);
    let ext_s = sanitize_filename_component(ext, replacements);

    pattern
        .replace("{artist}", &artist_s)
        .replace("{albumartist}", &albumartist_s)
        .replace("{album}", &album_s)
        .replace("{title}", &title_s)
        .replace("{ext}", &ext_s)
}

pub(super) fn extract_song_name_from_filename(filename: &str) -> Option<String> {
    let stem = Path::new(filename).file_stem()?.to_str()?;
    for sep in [" - ", " － "] {
        let parts: Vec<&str> = stem.split(sep).collect();
        if parts.len() > 1 {
            return Some(parts[1].to_string());
        }
    }
    None
}

pub(super) fn update_playlist_line(
    playlist_path: &str,
    target_line: &str,
    new_line: &str,
) -> std::io::Result<()> {
    let content = std::fs::read_to_string(playlist_path)?;
    let playlist_dir = Path::new(playlist_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));

    let target_path = Path::new(target_line);
    let target_rel = target_path.strip_prefix(playlist_dir).unwrap_or(target_path);
    let new_path = Path::new(new_line);
    let new_rel = new_path.strip_prefix(playlist_dir).unwrap_or(new_path);

    let mut replaced = false;
    let mut new_lines = Vec::new();
    for line in content.lines() {
        let line_path = Path::new(line.trim());
        let line_rel = line_path.strip_prefix(playlist_dir).unwrap_or(line_path);
        if !replaced && line_rel == target_rel {
            new_lines.push(new_rel.to_string_lossy().to_string());
            replaced = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    println!(
        "Updating playlist: {} -> {}",
        target_rel.display(),
        new_rel.display()
    );
    if !replaced {
        println!(
            "{}",
            format!(
                "Warning: target '{}' not found in '{}'",
                target_rel.display(),
                playlist_path
            )
            .yellow()
        );
        return Ok(());
    }
    std::fs::write(playlist_path, new_lines.join("\n"))?;
    Ok(())
}

// ── Lofty helpers ─────────────────────────────────────────────────────────────

pub(super) fn tag_str(tag: Option<&lofty::tag::Tag>, key: &ItemKey) -> String {
    tag.and_then(|t| t.get_string(key))
        .unwrap_or("")
        .to_string()
}

pub(super) fn get_duration_with_lofty(path: &Path) -> i64 {
    match lofty::read_from_path(path) {
        Ok(f) => f.properties().duration().as_secs() as i64,
        Err(_) => 0,
    }
}

pub(super) fn copy_lyrics_tag(path: &Path) {
    let Ok(mut tagged_file) = lofty::read_from_path(path) else {
        return;
    };
    let Some(tag) = tagged_file.primary_tag_mut() else {
        return;
    };
    let Some((key, lyrics)) = tag
        .items()
        .find(|i| matches!(i.key(), ItemKey::Unknown(k) if k.to_lowercase().starts_with("lyrics")))
        .and_then(|i| Some((i.key().clone(), i.value().text()?.to_string())))
    else {
        return;
    };
    tag.remove_key(&key);
    tag.insert_text(ItemKey::Lyrics, lyrics);
    let _ = tag.save_to_path(path, WriteOptions::default());
}

pub(super) fn write_lyrics_tag(
    path: &Path,
    lyrics: String,
    artist: &str,
    title: &str,
    pb: &ProgressBar,
    kind: &str,
) -> bool {
    match lofty::read_from_path(path) {
        Ok(mut tagged_file) => {
            if let Some(tag) = tagged_file.primary_tag_mut() {
                tag.insert_text(ItemKey::Lyrics, lyrics);
                match tag.save_to_path(path, WriteOptions::default()) {
                    Ok(_) => {
                        pb.set_message(format!(
                            "✓ tagged {} lyrics for {} - {}",
                            kind, artist, title
                        ));
                        true
                    }
                    Err(e) => {
                        eprintln!("Failed to write lyrics to {}: {}", path.display(), e);
                        false
                    }
                }
            } else {
                false
            }
        }
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            false
        }
    }
}

pub(super) fn is_synced_lyrics(s: &str) -> bool {
    s.lines().any(|l| {
        let l = l.trim();
        l.starts_with('[') && l[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
    })
}
