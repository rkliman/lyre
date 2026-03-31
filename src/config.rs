use crate::colors::default_colors;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use crate::util::expand_tilde;

// ============================================================================
// Configuration Constants
// ============================================================================

/// Default database path
pub const DEFAULT_DATABASE_PATH: &str = "~/.local/share/lyre/music.db";

/// Default music directory
pub const DEFAULT_MUSIC_DIR: &str = "~/Music";

#[derive(Debug, Deserialize, Clone)]
pub struct FilesConfig {
    #[serde(default = "default_database_name")]
    pub database_name: String,
    #[serde(default = "default_music_directory")]
    pub music_directory: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct UiColorsConfig {
    #[serde(default = "default_foreground")]
    pub foreground: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_accent2")]
    pub accent2: String,
    #[serde(default = "default_dim")]
    pub dim: String,
    #[serde(default = "default_highlight")]
    pub highlight: String,
    #[serde(default = "default_playing")]
    pub playing: String,
    #[serde(default = "default_header_bg")]
    pub header_bg: String,
    #[serde(default = "default_selection_bg")]
    pub selection_bg: String,
    #[serde(default = "default_overlay_bg")]
    pub overlay_bg: String,
    #[serde(default = "default_gauge_bg")]
    pub gauge_bg: String,
    #[serde(default = "default_art_bg")]
    pub art_bg: String,
    #[serde(default = "default_art_border")]
    pub art_border: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UiConfig {
    #[serde(default)]
    pub colors: UiColorsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub files: FilesConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

// Default functions for FilesConfig
fn default_database_name() -> String {
    DEFAULT_DATABASE_PATH.to_string()
}

fn default_music_directory() -> String {
    DEFAULT_MUSIC_DIR.to_string()
}

// Default functions for UiColorsConfig
fn default_foreground() -> String {
    default_colors::FOREGROUND.to_string()
}

fn default_background() -> String {
    default_colors::BACKGROUND.to_string()
}

fn default_accent() -> String {
    default_colors::ACCENT.to_string()
}

fn default_accent2() -> String {
    default_colors::ACCENT2.to_string()
}

fn default_dim() -> String {
    default_colors::DIM.to_string()
}

fn default_highlight() -> String {
    default_colors::HIGHLIGHT.to_string()
}

fn default_playing() -> String {
    default_colors::PLAYING.to_string()
}

fn default_header_bg() -> String {
    default_colors::HEADER_BG.to_string()
}

fn default_selection_bg() -> String {
    default_colors::SELECTION_BG.to_string()
}

fn default_overlay_bg() -> String {
    default_colors::OVERLAY_BG.to_string()
}

fn default_gauge_bg() -> String {
    default_colors::GAUGE_BG.to_string()
}

fn default_art_bg() -> String {
    default_colors::ART_BG.to_string()
}

fn default_art_border() -> String {
    default_colors::ART_BORDER.to_string()
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            database_name: default_database_name(),
            music_directory: default_music_directory(),
        }
    }
}

impl Default for UiColorsConfig {
    fn default() -> Self {
        Self {
            foreground: default_foreground(),
            background: default_background(),
            accent: default_accent(),
            accent2: default_accent2(),
            dim: default_dim(),
            highlight: default_highlight(),
            playing: default_playing(),
            header_bg: default_header_bg(),
            selection_bg: default_selection_bg(),
            overlay_bg: default_overlay_bg(),
            gauge_bg: default_gauge_bg(),
            art_bg: default_art_bg(),
            art_border: default_art_border(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colors: UiColorsConfig::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            files: FilesConfig::default(),
            ui: UiConfig::default(),
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

    if let Ok(cfg) = config::Config::builder()
        .add_source(config::File::with_name(&config_path_str).required(false))
        .build()
    {
        if let Ok(settings) = cfg.try_deserialize::<Config>() {
            return settings;
        }
    }

    Config::default()
}