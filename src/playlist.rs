/// .m3u / .m3u8 playlist support.
///
/// Paths in m3u files can be absolute or relative to the playlist file.
/// We always write them as absolute paths for simplicity.
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Playlist {
    /// Display name (stem of the file)
    pub name: String,
    /// Absolute path to the .m3u file
    pub path: String,
    /// Absolute paths of entries, in order
    pub entries: Vec<String>,
}

impl Playlist {
    /// Load a playlist from disk, resolving relative paths against the playlist's directory.
    pub fn load(path: &str) -> Result<Self> {
        let abs_path = PathBuf::from(path);
        let dir = abs_path.parent().unwrap_or_else(|| Path::new(""));
        let name = abs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unnamed")
            .to_string();

        let content = std::fs::read_to_string(&abs_path)?;
        let entries: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                let p = Path::new(l);
                if p.is_absolute() {
                    l.to_string()
                } else {
                    dir.join(p).to_string_lossy().to_string()
                }
            })
            .collect();

        Ok(Self {
            name,
            path: path.to_string(),
            entries,
        })
    }

    /// Write the playlist back to disk. Entries are written as absolute paths.
    pub fn save(&self) -> Result<()> {
        let lines: Vec<String> = self.entries.iter().map(|e| e.clone()).collect();
        std::fs::write(&self.path, lines.join("\n") + "\n")?;
        Ok(())
    }

    /// Create a new empty playlist at the given path and save it immediately.
    pub fn create(path: &str, name: &str) -> Result<Self> {
        let pl = Self {
            name: name.to_string(),
            path: path.to_string(),
            entries: Vec::new(),
        };
        pl.save()?;
        Ok(pl)
    }

    pub fn add_entry(&mut self, track_path: &str) {
        if !self.entries.iter().any(|e| e == track_path) {
            self.entries.push(track_path.to_string());
        }
    }

    pub fn remove_entry(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
        }
    }

    pub fn move_entry_up(&mut self, index: usize) {
        if index > 0 && index < self.entries.len() {
            self.entries.swap(index - 1, index);
        }
    }

    pub fn move_entry_down(&mut self, index: usize) {
        if index + 1 < self.entries.len() {
            self.entries.swap(index, index + 1);
        }
    }
}

/// Scan a directory recursively for .m3u / .m3u8 files and return loaded playlists.
pub fn scan_playlists(music_dir: &str) -> Vec<Playlist> {
    let mut playlists = Vec::new();
    let walker = walkdir::WalkDir::new(music_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "m3u" || ext == "m3u8" {
                if let Ok(pl) = Playlist::load(&path.to_string_lossy()) {
                    playlists.push(pl);
                }
            }
        }
    }

    playlists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    playlists
}
