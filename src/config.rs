use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::util::expand_tilde;

// ============================================================================

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
// Configuration Constants
// ============================================================================

/// Default database path
pub const DEFAULT_DATABASE_PATH: &str = "~/Music/music_library.db";

/// Default music directory
pub const DEFAULT_MUSIC_DIR: &str = "~/Music";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FilesConfig {
    #[serde(default = "default_database_name")]
    pub database_name: String,
    #[serde(default = "default_music_directory")]
    pub music_directory: String,
    /// Optional file-naming pattern used by `lyre index` to rename files on import.
    /// Example: `"{album}/{albumartist} - {title}.{ext}"`
    #[serde(default)]
    pub file_pattern: Option<String>,
    /// Glob patterns (relative to music_directory) to skip during indexing.
    #[serde(default)]
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct UiColorsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art_border: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default)]
    pub colors: UiColorsConfig,
    /// Optional track-list columns to show, by config key (see `TrackColumn::config_key`).
    #[serde(default = "default_columns")]
    pub columns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub files: FilesConfig,
    #[serde(default)]
    pub ui: UiConfig,
    /// Character replacements applied to file/folder names when renaming during indexing.
    /// Maps forbidden filesystem chars to safe Unicode lookalikes.
    #[serde(default = "default_replace")]
    pub replace: Option<HashMap<String, String>>,
}

// Default functions for FilesConfig
fn default_database_name() -> String {
    DEFAULT_DATABASE_PATH.to_string()
}

fn default_music_directory() -> String {
    DEFAULT_MUSIC_DIR.to_string()
}

fn default_columns() -> Vec<String> {
    ["artist", "album", "duration", "play_count"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn default_replace() -> Option<HashMap<String, String>> {
    Some(
        [
            (":", "∶"),
            ("/", "⁄"),
            ("*", "∗"),
            ("?", "？"),
            ("\"", "″"),
            ("\\", "⧵"),
            (".", "․"),
            ("|", "ǀ"),
            ("<", "‹"),
            (">", "›"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
    )
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            database_name: default_database_name(),
            music_directory: default_music_directory(),
            file_pattern: None,
            ignore: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: None,
            colors: UiColorsConfig::default(),
            columns: default_columns(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            files: FilesConfig::default(),
            ui: UiConfig::default(),
            replace: default_replace(),
        }
    }
}

pub fn load_config() -> Config {
    let config_path_str = expand_tilde("~/.config/lyre/config.toml");
    let config_path = Path::new(&config_path_str);

    // Create default config if it doesn't exist
    if !config_path.exists() {
        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Write default config file
        let default_config = include_str!("../config.toml.example");
        let _ = fs::write(config_path, default_config);
    }

    if let Ok(contents) = fs::read_to_string(&config_path_str) {
        if let Ok(settings) = toml::from_str::<Config>(&contents) {
            return settings;
        }
    }

    Config::default()
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path_str = expand_tilde("~/.config/lyre/config.toml");
    if let Some(parent) = Path::new(&config_path_str).parent() {
        let _ = fs::create_dir_all(parent);
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(config_path_str, contents)?;
    Ok(())
}
