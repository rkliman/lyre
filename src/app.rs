use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::db::Db;
use crate::keybindings::{Action, Keybindings};
use crate::player::Player;
use crate::playlist::{scan_playlists, Playlist};
use crate::types::{
    Lyrics, Overlay, Panel, PlayerState, Result, SidebarItem, SortField, SortOrder, Track,
    TrackContext,
};
use ratatui::style::Color;
use std::fs;

// ============================================================================
// Configuration Constants
// ============================================================================

/// Default database path
pub const DEFAULT_DATABASE_PATH: &str = "~/.local/share/lyre/music.db";

/// Default music directory
pub const DEFAULT_MUSIC_DIR: &str = "~/Music";

/// Default color values for UI theme
pub mod default_colors {
    pub const FOREGROUND: &str = "#dcd7cd";
    pub const BACKGROUND: &str = "#12100e";
    pub const ACCENT: &str = "#d6a362";
    pub const ACCENT2: &str = "#b27844";
    pub const DIM: &str = "#645c50";
    pub const HIGHLIGHT: &str = "#f0c378";
    pub const PLAYING: &str = "#82c88c";
    pub const HEADER_BG: &str = "#1e1a16";
    pub const SELECTION_BG: &str = "#2d261c";
    pub const OVERLAY_BG: &str = "#16120e";
    pub const GAUGE_BG: &str = "#28231c";
    pub const ART_BG: &str = "#12100e";
    pub const ART_BORDER: &str = "#28231c";
}

/// Expand tilde (~) to home directory
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct FilesConfig {
    #[serde(default = "default_database_name")]
    database_name: String,
    #[serde(default = "default_music_directory")]
    music_directory: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct UiColorsConfig {
    #[serde(default = "default_foreground")]
    foreground: String,
    #[serde(default = "default_background")]
    background: String,
    #[serde(default = "default_accent")]
    accent: String,
    #[serde(default = "default_accent2")]
    accent2: String,
    #[serde(default = "default_dim")]
    dim: String,
    #[serde(default = "default_highlight")]
    highlight: String,
    #[serde(default = "default_playing")]
    playing: String,
    #[serde(default = "default_header_bg")]
    header_bg: String,
    #[serde(default = "default_selection_bg")]
    selection_bg: String,
    #[serde(default = "default_overlay_bg")]
    overlay_bg: String,
    #[serde(default = "default_gauge_bg")]
    gauge_bg: String,
    #[serde(default = "default_art_bg")]
    art_bg: String,
    #[serde(default = "default_art_border")]
    art_border: String,
}

#[derive(Debug, Deserialize, Clone)]
struct UiConfig {
    #[serde(default)]
    colors: UiColorsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    files: FilesConfig,
    #[serde(default)]
    ui: UiConfig,
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

fn load_config() -> Config {
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

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub foreground: Color,
    pub background: Color,
    pub accent: Color,
    pub accent2: Color,
    pub dim: Color,
    pub highlight: Color,
    pub playing: Color,
    pub header_bg: Color,
    pub selection_bg: Color,
    pub overlay_bg: Color,
    pub gauge_bg: Color,
    pub art_bg: Color,
    pub art_border: Color,
}

fn parse_color(color_str: &str) -> Color {
    let trimmed = color_str.trim();

    // Try hex format: #RRGGBB
    if trimmed.starts_with('#') && trimmed.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&trimmed[1..3], 16),
            u8::from_str_radix(&trimmed[3..5], 16),
            u8::from_str_radix(&trimmed[5..7], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }

    // Try comma-separated RGB: "R, G, B"
    if trimmed.contains(',') {
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].trim().parse::<u8>(),
                parts[1].trim().parse::<u8>(),
                parts[2].trim().parse::<u8>(),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
    }

    // Fallback to white
    Color::Rgb(255, 255, 255)
}

impl ColorScheme {
    pub fn from_config(colors: &UiColorsConfig) -> Self {
        Self {
            foreground: parse_color(&colors.foreground),
            background: parse_color(&colors.background),
            accent: parse_color(&colors.accent),
            accent2: parse_color(&colors.accent2),
            dim: parse_color(&colors.dim),
            highlight: parse_color(&colors.highlight),
            playing: parse_color(&colors.playing),
            header_bg: parse_color(&colors.header_bg),
            selection_bg: parse_color(&colors.selection_bg),
            overlay_bg: parse_color(&colors.overlay_bg),
            gauge_bg: parse_color(&colors.gauge_bg),
            art_bg: parse_color(&colors.art_bg),
            art_border: parse_color(&colors.art_border),
        }
    }

    /// Style for normal text
    pub fn normal_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.foreground)
    }

    /// Style for dimmed/secondary text
    pub fn dim_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.dim)
    }

    /// Style for accent text
    pub fn accent_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent)
    }

    /// Style for bold accent text (headers, titles)
    pub fn accent_bold_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.accent)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    /// Style for highlighted text
    pub fn highlight_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.highlight)
    }

    /// Style for bold highlighted text
    pub fn highlight_bold_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.highlight)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    /// Style for background blocks
    pub fn block_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().bg(self.background)
    }

    /// Style for selected items
    pub fn selected_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.highlight)
            .bg(self.selection_bg)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    /// Style for selected items (with accent2 foreground)
    pub fn selected_accent2_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.accent2)
            .bg(self.selection_bg)
    }

    /// Style for borders
    pub fn border_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent2)
    }

    /// Style for active borders
    pub fn border_active_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent)
    }

    /// Style for inactive borders
    pub fn border_inactive_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.dim)
    }
}

pub struct App {
    pub colors: ColorScheme,
    pub keybindings: Keybindings,
    pub all_tracks: Vec<Track>,
    pub filtered_tracks: Vec<Track>,
    pub track_context: TrackContext,
    // Pre-computed display lines per track.
    // Each entry is (collapsed_line, expanded_lines).
    // collapsed_line: single row with … truncation for non-selected display.
    // expanded_lines: full word-wrapped rows shown when the track is selected.
    pub wrapped_tracks: Vec<(String, Vec<String>)>,
    pub wrapped_width: usize,
    pub sidebar_artists: Vec<String>,
    pub sidebar_albums: Vec<String>,
    pub sidebar_genres: Vec<String>,
    pub sidebar_items: Vec<SidebarItem>,
    pub sidebar_index: usize,
    pub sidebar_expanded: HashMap<String, bool>,
    pub sidebar_search_mode: bool,
    pub sidebar_search_query: String,
    pub sidebar_search_section: Option<String>,
    pub filtered_sidebar_artists: Vec<String>,
    pub filtered_sidebar_albums: Vec<String>,
    pub filtered_sidebar_genres: Vec<String>,
    pub playlists: Vec<Playlist>,
    pub music_dir: String,
    pub track_list_index: usize,
    pub track_list_offset: usize,
    pub selected_tracks: HashSet<usize>,
    pub selection_anchor: Option<usize>,
    pub queue_index: usize,
    pub active_panel: Panel,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub search_mode: bool,
    pub search_query: String,
    pub player: Player,
    pub status_message: Option<String>,
    pub album_art: Option<crate::art::BlockArt>,
    pub album_art_image: Option<DynamicImage>,
    pub album_art_path: Option<String>,
    // Cache raw album art bytes to avoid re-extracting from file
    pub album_art_bytes: Option<Vec<u8>>,
    // Cached file:// URI for MPRIS (avoids re-extracting on every tick)
    pub mpris_art_url: Option<String>,
    pub image_picker: Option<Picker>,
    // Cached block art for art window (regenerated only when dimensions change)
    pub cached_art_window_block: Option<crate::art::BlockArt>,
    pub cached_art_window_dims: (u16, u16),
    // Cached protocol for terminal graphics rendering (regenerated only when image/dimensions change)
    pub cached_art_window_protocol: Option<StatefulProtocol>,
    pub overlay: Overlay,
    pub show_help: bool,
    pub help_scroll: usize,
    pub lyrics_visible: bool,
    pub lyrics_content: Option<Lyrics>,
    pub lyrics_scroll: usize,
    pub lyrics_track_path: Option<String>,
    pub art_window_visible: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = load_config();
        let colors = ColorScheme::from_config(&config.ui.colors);
        let music_dir = expand_tilde(&config.files.music_directory);
        let db_path = expand_tilde(&config.files.database_name);

        let db = Db::open(&db_path)?;
        let all_tracks = db.all_tracks().unwrap_or_default();
        let filtered_tracks = all_tracks.clone();
        let artists = db.distinct_artists().unwrap_or_default();
        let albums = db.distinct_albums().unwrap_or_default();
        let genres = db.distinct_genres().unwrap_or_default();
        let playlists = scan_playlists(&music_dir);

        let mut sidebar_expanded: HashMap<String, bool> = HashMap::new();
        sidebar_expanded.insert("Artists".to_string(), false);
        sidebar_expanded.insert("Albums".to_string(), false);
        sidebar_expanded.insert("Genres".to_string(), false);
        sidebar_expanded.insert("Playlists".to_string(), false);

        let player = Player::new().expect("Failed to initialize audio.");
        let image_picker = crate::art::create_picker();
        let keybindings = Keybindings::new();

        let mut app = Self {
            colors,
            keybindings,
            all_tracks,
            filtered_tracks,
            track_context: TrackContext::Library,
            wrapped_tracks: Vec::new(),
            wrapped_width: 0,
            sidebar_artists: artists.clone(),
            sidebar_albums: albums.clone(),
            sidebar_genres: genres.clone(),
            sidebar_items: Vec::new(),
            sidebar_index: 0,
            sidebar_expanded,
            sidebar_search_mode: false,
            sidebar_search_query: String::new(),
            sidebar_search_section: None,
            filtered_sidebar_artists: artists,
            filtered_sidebar_albums: albums,
            filtered_sidebar_genres: genres,
            playlists,
            music_dir,
            track_list_index: 0,
            track_list_offset: 0,
            selected_tracks: HashSet::new(),
            selection_anchor: None,
            queue_index: 0,
            active_panel: Panel::Sidebar,
            sort_field: SortField::Artist,
            sort_order: SortOrder::Asc,
            search_mode: false,
            search_query: String::new(),
            player,
            status_message: None,
            album_art: None,
            album_art_image: None,
            album_art_path: None,
            album_art_bytes: None,
            mpris_art_url: None,
            image_picker,
            cached_art_window_block: None,
            cached_art_window_dims: (0, 0),
            cached_art_window_protocol: None,
            overlay: Overlay::None,
            show_help: false,
            help_scroll: 0,
            lyrics_visible: false,
            lyrics_content: None,
            lyrics_scroll: 0,
            lyrics_track_path: None,
            art_window_visible: false,
        };
        app.rebuild_sidebar();
        app.apply_sort();
        Ok(app)
    }

    pub fn rebuild_sidebar(&mut self) {
        let mut items = vec![SidebarItem::AllTracks];

        let artists_open = *self.sidebar_expanded.get("Artists").unwrap_or(&true);
        items.push(SidebarItem::Artists);
        if artists_open {
            for a in &self.filtered_sidebar_artists {
                items.push(SidebarItem::Artist(a.clone()));
            }
        }

        let albums_open = *self.sidebar_expanded.get("Albums").unwrap_or(&true);
        items.push(SidebarItem::Albums);
        if albums_open {
            for a in &self.filtered_sidebar_albums {
                items.push(SidebarItem::Album(a.clone()));
            }
        }

        let genres_open = *self.sidebar_expanded.get("Genres").unwrap_or(&true);
        items.push(SidebarItem::Genres);
        if genres_open {
            for g in &self.filtered_sidebar_genres {
                items.push(SidebarItem::Genre(g.clone()));
            }
        }

        let playlists_open = *self.sidebar_expanded.get("Playlists").unwrap_or(&true);
        items.push(SidebarItem::Playlists);
        if playlists_open {
            for pl in &self.playlists {
                items.push(SidebarItem::Playlist(pl.name.clone()));
            }
        }

        if self.sidebar_index >= items.len() {
            self.sidebar_index = items.len().saturating_sub(1);
        }
        self.sidebar_items = items;
    }

    fn toggle_sidebar_section(&mut self, item: &SidebarItem) {
        let key = match item {
            SidebarItem::Artists => "Artists",
            SidebarItem::Albums => "Albums",
            SidebarItem::Genres => "Genres",
            SidebarItem::Playlists => "Playlists",
            _ => return,
        };
        let current = *self.sidebar_expanded.get(key).unwrap_or(&true);
        self.sidebar_expanded.insert(key.to_string(), !current);
        self.rebuild_sidebar();
    }

    fn filter_sidebar_section(&mut self, section: &str) {
        if self.sidebar_search_query.is_empty() {
            // Reset to full lists
            self.filtered_sidebar_artists = self.sidebar_artists.clone();
            self.filtered_sidebar_albums = self.sidebar_albums.clone();
            self.filtered_sidebar_genres = self.sidebar_genres.clone();
            return;
        }

        let query = &self.sidebar_search_query;
        match section {
            "Artists" => {
                self.filtered_sidebar_artists = self.fuzzy_match_items(&self.sidebar_artists, query);
            }
            "Albums" => {
                self.filtered_sidebar_albums = self.fuzzy_match_items(&self.sidebar_albums, query);
            }
            "Genres" => {
                self.filtered_sidebar_genres = self.fuzzy_match_items(&self.sidebar_genres, query);
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        if self.player.is_finished() {
            let _ = self.player.next();
            self.refresh_album_art();
        }

        // Auto-reload lyrics when track changes
        if self.lyrics_visible {
            let current_track_path = self.player.current_track.as_ref().map(|t| t.path.clone());
            if current_track_path != self.lyrics_track_path {
                self.load_lyrics();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Handle sidebar search mode before keybindings lookup
        if self.sidebar_search_mode {
            return self.handle_sidebar_search_key(key.code);
        }

        // Handle Escape on sidebar with active filter
        if key.code == KeyCode::Esc
            && self.active_panel == Panel::Sidebar
            && (!self.sidebar_search_query.is_empty() || self.sidebar_search_section.is_some())
        {
            self.sidebar_search_mode = false;
            self.sidebar_search_query.clear();
            if let Some(section) = &self.sidebar_search_section.clone() {
                self.filter_sidebar_section(section);
                self.rebuild_sidebar();
            }
            self.sidebar_search_section = None;
            return false;
        }

        let action = self.keybindings.lookup(
            key.code,
            key.modifiers,
            self.active_panel,
            self.search_mode,
            self.show_help,
            self.overlay != Overlay::None,
        );

        if let Some(action) = action {
            self.execute_action(action)
        } else {
            false
        }
    }

    fn execute_action(&mut self, action: Action) -> bool {
        match action {
            // Global actions
            Action::Quit => return true,
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.help_scroll = 0;
                }
            }
            Action::CycleForward => self.cycle_panel_forward(),
            Action::CycleBackward => self.cycle_panel_backward(),
            Action::JumpToSidebar => self.active_panel = Panel::Sidebar,
            Action::JumpToQueue => self.active_panel = Panel::Queue,
            Action::JumpToTracks => self.active_panel = Panel::TrackList,
            Action::ToggleArtWindow => self.toggle_art_window(),
            Action::ToggleLyrics => self.toggle_lyrics_panel(),

            // Playback
            Action::PlayPause => {
                if self.player.state == PlayerState::Stopped {
                    self.play_selected();
                } else {
                    self.player.toggle_pause();
                }
            }
            Action::Next => {
                let _ = self.player.next();
                self.refresh_album_art();
            }
            Action::Previous => {
                let _ = self.player.prev();
                self.refresh_album_art();
            }
            Action::Stop => self.player.stop(),
            Action::VolumeUp => self.player.volume_up(),
            Action::VolumeDown => self.player.volume_down(),
            Action::SeekForward => {
                let _ = self.player.seek_forward(std::time::Duration::from_secs(5));
            }
            Action::SeekBackward => {
                let _ = self.player.seek_backward(std::time::Duration::from_secs(5));
            }
            Action::ToggleShuffle => self.toggle_shuffle(),
            Action::ToggleLoop => self.toggle_loop(),

            // Navigation
            Action::MoveUp => self.handle_move(key_to_code(Action::MoveUp)),
            Action::MoveDown => self.handle_move(key_to_code(Action::MoveDown)),
            Action::PageUp => self.handle_move(key_to_code(Action::PageUp)),
            Action::PageDown => self.handle_move(key_to_code(Action::PageDown)),
            Action::GoToTop => self.handle_move(key_to_code(Action::GoToTop)),
            Action::GoToBottom => self.handle_move(key_to_code(Action::GoToBottom)),
            Action::MoveLeft => self.handle_move(key_to_code(Action::MoveLeft)),
            Action::MoveRight => self.handle_move(key_to_code(Action::MoveRight)),
            Action::ExtendSelectionUp => self.extend_selection_up(),
            Action::ExtendSelectionDown => self.extend_selection_down(),
            Action::Enter => {
                match self.active_panel {
                    Panel::Sidebar => self.active_panel = Panel::TrackList,
                    Panel::TrackList => self.play_selected(),
                    Panel::Queue => {
                        if self.queue_index < self.player.queue.len() {
                            self.player.queue_index = self.queue_index;
                            let track = self.player.queue[self.queue_index].clone();
                            match self.player.play_track(track) {
                                Ok(_) => {
                                    self.refresh_album_art();
                                }
                                Err(e) => {
                                    self.set_status(format!("Error playing track: {}", e));
                                }
                            }
                        }
                    }
                    Panel::Lyrics => {}
                }
            }

            // Library
            Action::CycleSort => self.cycle_sort(),
            Action::ToggleSortOrder => self.toggle_sort_order(),
            Action::EnterSearch => {
                // If on sidebar, activate sidebar search for the current section
                if self.active_panel == Panel::Sidebar {
                    if self.sidebar_index < self.sidebar_items.len() {
                        let item = self.sidebar_items[self.sidebar_index].clone();
                        let section = match &item {
                            SidebarItem::Artists => Some("Artists"),
                            SidebarItem::Albums => Some("Albums"),
                            SidebarItem::Genres => Some("Genres"),
                            SidebarItem::Artist(_) => Some("Artists"),
                            SidebarItem::Album(_) => Some("Albums"),
                            SidebarItem::Genre(_) => Some("Genres"),
                            _ => None,
                        };
                        if let Some(s) = section {
                            self.sidebar_search_mode = true;
                            self.sidebar_search_section = Some(s.to_string());
                        }
                    }
                } else {
                    // Otherwise, activate global track search
                    self.enter_search();
                }
            }

            // Queue
            Action::AddToQueue => self.add_selected_to_queue(),
            Action::AddAllToQueue => self.add_all_to_queue(),
            Action::RemoveFromQueue => {
                self.player.remove_from_queue(self.queue_index);
                if self.queue_index >= self.player.queue.len() && self.queue_index > 0 {
                    self.queue_index -= 1;
                }
            }
            Action::ClearQueue => {
                self.player.stop();
                self.player.clear_queue();
                self.queue_index = 0;
                self.clear_album_art();
            }

            // Playlists
            Action::NewPlaylist => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if matches!(item, SidebarItem::Playlists | SidebarItem::Playlist(_)) {
                    self.overlay = Overlay::NewPlaylist(String::new());
                }
            }
            Action::AddToPlaylist => self.open_add_to_playlist_overlay(),
            Action::RemoveFromPlaylist => {
                if matches!(&self.track_context, TrackContext::Playlist(_)) {
                    self.playlist_remove_track(self.track_list_index);
                }
            }
            Action::MoveTrackUp => {
                if matches!(&self.track_context, TrackContext::Playlist(_)) {
                    self.playlist_move_track_up(self.track_list_index);
                }
            }
            Action::MoveTrackDown => {
                if matches!(&self.track_context, TrackContext::Playlist(_)) {
                    self.playlist_move_track_down(self.track_list_index);
                }
            }

            // Search
            Action::SearchExit => {
                self.search_mode = false;
                if self.search_query.is_empty() {
                    self.filtered_tracks = self.all_tracks.clone();
                    self.apply_sort();
                }
            }
            Action::SearchConfirm => {
                self.search_mode = false;
                self.active_panel = Panel::TrackList;
                self.track_list_index = 0;
            }
            Action::SearchBackspace => {
                self.search_query.pop();
                self.apply_fuzzy_search();
            }
            Action::SearchChar(c) => {
                self.search_query.push(c);
                self.apply_fuzzy_search();
            }

            // Help
            Action::HelpScroll(delta) => {
                if delta < 0 {
                    self.help_scroll = self.help_scroll.saturating_sub(delta.abs() as usize);
                } else {
                    self.help_scroll = self.help_scroll.saturating_add(delta as usize);
                }
            }
            Action::HelpClose => {
                self.show_help = false;
                self.help_scroll = 0;
            }

            // Lyrics
            Action::LyricsReload => self.load_lyrics(),

            // Overlay
            Action::OverlayConfirm => self.handle_overlay_confirm(),
            Action::OverlayCancel => {
                self.overlay = Overlay::None;
            }
            Action::OverlayChar(c) => self.handle_overlay_char(c),
            Action::OverlayBackspace => self.handle_overlay_backspace(),
            Action::OverlayNavigate(delta) => self.handle_overlay_navigate(delta),
        }
        false
    }

    fn handle_move(&mut self, key: KeyCode) {
        match self.active_panel {
            Panel::Sidebar => self.handle_sidebar_key(key),
            Panel::TrackList => self.handle_tracklist_key(key),
            Panel::Queue => self.handle_queue_key(key),
            Panel::Lyrics => self.handle_lyrics_key(key),
        }
    }

    fn handle_overlay_confirm(&mut self) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                if !name.trim().is_empty() {
                    self.create_playlist(name.trim());
                }
                self.overlay = Overlay::None;
            }
            Overlay::AddToPlaylist { track_paths, selected } => {
                if *selected < self.playlists.len() {
                    self.add_tracks_to_playlist(*selected, track_paths);
                }
                self.overlay = Overlay::None;
            }
            Overlay::None => {}
        }
    }

    fn handle_overlay_char(&mut self, c: char) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                let mut name = name.clone();
                name.push(c);
                self.overlay = Overlay::NewPlaylist(name);
            }
            Overlay::AddToPlaylist { .. } => {}
            Overlay::None => {}
        }
    }

    fn handle_overlay_backspace(&mut self) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                let mut name = name.clone();
                name.pop();
                self.overlay = Overlay::NewPlaylist(name);
            }
            Overlay::AddToPlaylist { .. } => {}
            Overlay::None => {}
        }
    }

    fn handle_overlay_navigate(&mut self, delta: i32) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(_) => {}
            Overlay::AddToPlaylist { track_paths, selected } => {
                let mut new_selected = *selected as i32 + delta;
                new_selected = new_selected.max(0).min(self.playlists.len().saturating_sub(1) as i32);
                self.overlay = Overlay::AddToPlaylist {
                    track_paths: track_paths.clone(),
                    selected: new_selected as usize,
                };
            }
            Overlay::None => {}
        }
    }

    /// Handle common navigation keys for lists. Returns true if handled.
    /// Operates on a mutable index and max length.
    fn handle_list_navigation(key: KeyCode, index: &mut usize, max_len: usize) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if *index > 0 {
                    *index -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *index + 1 < max_len {
                    *index += 1;
                }
                true
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                *index = index.saturating_sub(10);
                true
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                *index = (*index + 10).min(max_len.saturating_sub(1));
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                *index = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                *index = max_len.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyCode) {
        if Self::handle_list_navigation(key, &mut self.sidebar_index, self.sidebar_items.len()) {
            self.on_sidebar_select();
            return;
        }

        match key {
            KeyCode::Enter => {
                self.active_panel = Panel::TrackList;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if item.is_header() {
                    let k = match &item {
                        SidebarItem::Artists => "Artists",
                        SidebarItem::Albums => "Albums",
                        SidebarItem::Genres => "Genres",
                        SidebarItem::Playlists => "Playlists",
                        _ => "",
                    };
                    if !k.is_empty() {
                        let open = *self.sidebar_expanded.get(k).unwrap_or(&true);
                        if !open {
                            self.sidebar_expanded.insert(k.to_string(), true);
                            self.rebuild_sidebar();
                        } else {
                            self.active_panel = Panel::TrackList;
                        }
                    }
                } else {
                    self.active_panel = Panel::TrackList;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if item.is_header() {
                    self.toggle_sidebar_section(&item);
                } else {
                    // If on a child item, move cursor to the header of its section
                    let header_index = match &item {
                        SidebarItem::Artist(_) => self
                            .sidebar_items
                            .iter()
                            .position(|i| matches!(i, SidebarItem::Artists)),
                        SidebarItem::Album(_) => self
                            .sidebar_items
                            .iter()
                            .position(|i| matches!(i, SidebarItem::Albums)),
                        SidebarItem::Genre(_) => self
                            .sidebar_items
                            .iter()
                            .position(|i| matches!(i, SidebarItem::Genres)),
                        SidebarItem::Playlist(_) => self
                            .sidebar_items
                            .iter()
                            .position(|i| matches!(i, SidebarItem::Playlists)),
                        _ => None,
                    };
                    if let Some(idx) = header_index {
                        self.sidebar_index = idx;
                        self.on_sidebar_select();
                    }
                }
            }
            KeyCode::Char('N') => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if matches!(item, SidebarItem::Playlists | SidebarItem::Playlist(_)) {
                    self.overlay = Overlay::NewPlaylist(String::new());
                }
            }
            _ => {}
        }
    }

    fn handle_tracklist_key(&mut self, key: KeyCode) {
        if Self::handle_list_navigation(key, &mut self.track_list_index, self.filtered_tracks.len()) {
            self.clear_selection();
            return;
        }

        match key {
            KeyCode::Left | KeyCode::Char('h') => {
                self.active_panel = Panel::Sidebar;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.active_panel = Panel::Queue;
            }
            _ => {}
        }
    }

    fn handle_queue_key(&mut self, key: KeyCode) {
        if Self::handle_list_navigation(key, &mut self.queue_index, self.player.queue.len()) {
            return;
        }

        match key {
            KeyCode::Enter => {
                if self.queue_index < self.player.queue.len() {
                    self.player.queue_index = self.queue_index;
                    let track = self.player.queue[self.queue_index].clone();
                    match self.player.play_track(track) {
                        Ok(_) => {
                            self.refresh_album_art();
                        }
                        Err(e) => {
                            self.set_status(format!("Error playing track: {}", e));
                        }
                    }
                }
            }
            KeyCode::Delete | KeyCode::Char('x') => {
                self.player.remove_from_queue(self.queue_index);
                if self.queue_index >= self.player.queue.len() && self.queue_index > 0 {
                    self.queue_index -= 1;
                }
            }
            KeyCode::Char('c') => {
                self.player.stop();
                self.player.clear_queue();
                self.queue_index = 0;
                self.clear_album_art();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.active_panel = Panel::TrackList;
            }
            _ => {}
        }
    }

    fn handle_lyrics_key(&mut self, key: KeyCode) {
        // Lyrics use scroll with no upper bound (clamped in render)
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.lyrics_scroll = self.lyrics_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.lyrics_scroll = self.lyrics_scroll.saturating_add(1);
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                self.lyrics_scroll = self.lyrics_scroll.saturating_sub(10);
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                self.lyrics_scroll = self.lyrics_scroll.saturating_add(10);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.lyrics_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.lyrics_scroll = usize::MAX; // Will be clamped in render
            }
            KeyCode::Char('r') => {
                self.load_lyrics();
            } // Reload lyrics
            KeyCode::Left | KeyCode::Char('h') => {
                self.active_panel = Panel::Queue;
            }
            _ => {}
        }
    }

    fn handle_sidebar_search_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                // Clear search and hide searchbar
                self.sidebar_search_mode = false;
                self.sidebar_search_query.clear();
                if let Some(section) = &self.sidebar_search_section.clone() {
                    self.filter_sidebar_section(section);
                    self.rebuild_sidebar();
                }
                self.sidebar_search_section = None;
            }
            KeyCode::Enter => {
                // Keep search active but allow navigation
                self.sidebar_search_mode = false;
            }
            KeyCode::Backspace => {
                self.sidebar_search_query.pop();
                if let Some(section) = &self.sidebar_search_section.clone() {
                    self.filter_sidebar_section(section);
                    self.rebuild_sidebar();
                }
            }
            KeyCode::Char(c) => {
                self.sidebar_search_query.push(c);
                if let Some(section) = &self.sidebar_search_section.clone() {
                    self.filter_sidebar_section(section);
                    self.rebuild_sidebar();
                }
            }
            _ => {}
        }
        false
    }

    fn extend_selection_up(&mut self) {
        if self.filtered_tracks.is_empty() {
            return;
        }

        // Set anchor if this is the first extend operation
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.track_list_index);
            self.selected_tracks.insert(self.track_list_index);
        }

        // Move up
        if self.track_list_index > 0 {
            self.track_list_index -= 1;
        }

        // Update selection range
        self.update_selection_range();
    }

    fn extend_selection_down(&mut self) {
        if self.filtered_tracks.is_empty() {
            return;
        }

        // Set anchor if this is the first extend operation
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.track_list_index);
            self.selected_tracks.insert(self.track_list_index);
        }

        // Move down
        if self.track_list_index + 1 < self.filtered_tracks.len() {
            self.track_list_index += 1;
        }

        // Update selection range
        self.update_selection_range();
    }

    fn update_selection_range(&mut self) {
        if let Some(anchor) = self.selection_anchor {
            self.selected_tracks.clear();
            let start = anchor.min(self.track_list_index);
            let end = anchor.max(self.track_list_index);
            for i in start..=end {
                self.selected_tracks.insert(i);
            }
        }
    }

    fn clear_selection(&mut self) {
        self.selected_tracks.clear();
        self.selection_anchor = None;
    }


    fn on_sidebar_select(&mut self) {
        let item = self.sidebar_items[self.sidebar_index].clone();

        // Skip expensive operations when navigating to header items
        // Headers don't have associated tracks to display
        if item.is_header() {
            return;
        }

        self.search_query.clear();
        self.clear_selection();
        self.track_context = TrackContext::Library;

        self.filtered_tracks = match &item {
            SidebarItem::AllTracks => self.all_tracks.clone(),
            SidebarItem::Artist(a) => self
                .all_tracks
                .iter()
                .filter(|t| {
                    t.artist.eq_ignore_ascii_case(a) || t.albumartist.eq_ignore_ascii_case(a)
                })
                .cloned()
                .collect(),
            SidebarItem::Album(a) => self
                .all_tracks
                .iter()
                .filter(|t| t.album.eq_ignore_ascii_case(a))
                .cloned()
                .collect(),
            SidebarItem::Genre(g) => self
                .all_tracks
                .iter()
                .filter(|t| t.genre.split(',').any(|s| s.trim().eq_ignore_ascii_case(g)))
                .cloned()
                .collect(),
            SidebarItem::Playlist(name) => {
                self.track_context = TrackContext::Playlist(name.clone());
                self.load_playlist_tracks(name)
            }
            _ => self.all_tracks.clone(),
        };

        self.track_list_index = 0;
        self.track_list_offset = 0;
        if self.track_context == TrackContext::Library {
            self.apply_sort();
        }
        self.invalidate_wrap_cache();
    }

    fn load_playlist_tracks(&self, name: &str) -> Vec<Track> {
        let pl = match self.playlists.iter().find(|p| p.name == name) {
            Some(p) => p,
            _ => return Vec::new(),
        };
        pl.entries
            .iter()
            .filter_map(|ep| self.all_tracks.iter().find(|t| t.path == *ep).cloned())
            .collect()
    }

    fn create_playlist(&mut self, name: &str) {
        let file_name = format!("{}.m3u", name.replace('/', "_"));
        let path_str = Path::new(&self.music_dir)
            .join(&file_name)
            .to_string_lossy()
            .to_string();
        match Playlist::create(&path_str, name) {
            Ok(pl) => {
                self.playlists.push(pl);
                self.playlists
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.rebuild_sidebar();
                self.set_status(format!("Created playlist: {}", name));
            }
            Err(e) => self.set_status(format!("Failed to create playlist: {}", e)),
        }
    }

    fn open_add_to_playlist_overlay(&mut self) {
        if self.playlists.is_empty() {
            self.set_status(
                "No playlists — press N on the Playlists header to create one".to_string(),
            );
            return;
        }

        let track_paths = if self.selected_tracks.is_empty() {
            // Add single track
            if let Some(track) = self.filtered_tracks.get(self.track_list_index) {
                vec![track.path.clone()]
            } else {
                return;
            }
        } else {
            // Add multiple selected tracks
            let indices = self.get_selected_indices_ascending();
            indices.iter()
                .filter_map(|&idx| self.filtered_tracks.get(idx))
                .map(|track| track.path.clone())
                .collect()
        };

        if !track_paths.is_empty() {
            self.overlay = Overlay::AddToPlaylist {
                track_paths,
                selected: 0,
            };
        }
    }

    fn load_lyrics(&mut self) {
        self.lyrics_scroll = 0;
        self.lyrics_content = match &self.player.current_track {
            Some(track) => {
                self.lyrics_track_path = Some(track.path.clone());
                if let Some(lyrics_text) = crate::lyrics::extract_lyrics(&track.path) {
                    Some(crate::lyrics::parse_lyrics(&lyrics_text))
                } else {
                    Some(Lyrics::Plain("No lyrics available for this track.".to_string()))
                }
            }
            None => {
                self.lyrics_track_path = None;
                Some(Lyrics::Plain("No track is currently playing.".to_string()))
            }
        };
    }

    fn add_tracks_to_playlist(&mut self, playlist_idx: usize, track_paths: &[String]) {
        if playlist_idx >= self.playlists.len() {
            return;
        }
        let pl = &mut self.playlists[playlist_idx];
        for track_path in track_paths {
            pl.add_entry(track_path);
        }
        let pl_name = pl.name.clone();
        if let Err(e) = pl.save() {
            self.set_status(format!("Failed to save: {}", e));
            return;
        }
        let count = track_paths.len();
        if count == 1 {
            self.set_status(format!("Added to {}", pl_name));
        } else {
            self.set_status(format!("Added {} tracks to {}", count, pl_name));
        }
        if self.track_context == TrackContext::Playlist(pl_name.clone()) {
            self.filtered_tracks = self.load_playlist_tracks(&pl_name);
        }
        // Clear selection after adding
        self.clear_selection();
    }

    fn playlist_remove_track(&mut self, index: usize) {
        if self.selected_tracks.is_empty() {
            // Remove single track
            if self.with_current_playlist(|pl| {
                pl.remove_entry(index);
            }) {
                self.set_status("Removed from playlist".to_string());
            }
        } else {
            // Remove multiple selected tracks (in descending order to avoid index shifting)
            let indices = self.get_selected_indices_descending();
            let count = indices.len();

            if self.with_current_playlist(|pl| {
                for &idx in &indices {
                    pl.remove_entry(idx);
                }
            }) {
                self.set_status(format!("Removed {} tracks from playlist", count));
            }
            self.clear_selection();
        }

        if self.track_list_index >= self.filtered_tracks.len() && self.track_list_index > 0 {
            self.track_list_index -= 1;
        }
        self.invalidate_wrap_cache();
    }

    fn playlist_move_track_up(&mut self, index: usize) {
        if self.selected_tracks.is_empty() {
            // Move single track up
            self.with_current_playlist(|pl| {
                pl.move_entry_up(index);
            });
            if index > 0 {
                self.track_list_index -= 1;
            }
        } else {
            // Move multiple selected tracks up (process from top to bottom)
            let indices = self.get_selected_indices_ascending();
            let can_move = indices.first().map_or(false, |&first| first > 0);

            if can_move {
                self.with_current_playlist(|pl| {
                    for &idx in &indices {
                        pl.move_entry_up(idx);
                    }
                });

                // Update selection indices and current index
                self.selected_tracks = indices.iter().map(|&i| i - 1).collect();
                if self.track_list_index > 0 {
                    self.track_list_index -= 1;
                }
            }
        }
        self.invalidate_wrap_cache();
    }

    fn playlist_move_track_down(&mut self, index: usize) {
        if self.selected_tracks.is_empty() {
            // Move single track down
            let old_len = self.filtered_tracks.len();
            self.with_current_playlist(|pl| {
                pl.move_entry_down(index);
            });
            if index + 1 < old_len {
                self.track_list_index += 1;
            }
        } else {
            // Move multiple selected tracks down (process from bottom to top)
            let indices = self.get_selected_indices_descending();
            let max_index = self.filtered_tracks.len().saturating_sub(1);
            let can_move = indices.first().map_or(false, |&last| last < max_index);

            if can_move {
                self.with_current_playlist(|pl| {
                    for &idx in &indices {
                        pl.move_entry_down(idx);
                    }
                });

                // Update selection indices and current index
                self.selected_tracks = indices.iter().map(|&i| i + 1).collect();
                if self.track_list_index + 1 < self.filtered_tracks.len() {
                    self.track_list_index += 1;
                }
            }
        }
        self.invalidate_wrap_cache();
    }

    fn apply_fuzzy_search(&mut self) {
        self.clear_selection();
        if self.search_query.is_empty() {
            self.filtered_tracks = self.all_tracks.clone();
            return;
        }
        let matcher = SkimMatcherV2::default();
        let q = &self.search_query;
        let mut scored: Vec<(i64, Track)> = self
            .all_tracks
            .iter()
            .filter_map(|t| {
                let h = format!("{} {} {} {}", t.title, t.artist, t.albumartist, t.album);
                matcher.fuzzy_match(&h, q).map(|s| (s, t.clone()))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.filtered_tracks = scored.into_iter().map(|(_, t)| t).collect();
        self.track_list_index = 0;
        self.invalidate_wrap_cache();
    }

    pub fn apply_sort(&mut self) {
        if matches!(self.track_context, TrackContext::Playlist(_)) {
            return;
        }
        let asc = self.sort_order == SortOrder::Asc;
        match self.sort_field {
            SortField::Title => self.filtered_tracks.sort_by(|a, b| {
                let c = a.title.to_lowercase().cmp(&b.title.to_lowercase());
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
            SortField::Artist => self.filtered_tracks.sort_by(|a, b| {
                let c = a
                    .display_artist()
                    .to_lowercase()
                    .cmp(&b.display_artist().to_lowercase());
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
            SortField::Album => self.filtered_tracks.sort_by(|a, b| {
                let c = a.album.to_lowercase().cmp(&b.album.to_lowercase());
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
            SortField::Year => self.filtered_tracks.sort_by(|a, b| {
                let c = a.year.cmp(&b.year);
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
            SortField::Genre => self.filtered_tracks.sort_by(|a, b| {
                let c = a.genre.to_lowercase().cmp(&b.genre.to_lowercase());
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
            SortField::Duration => self.filtered_tracks.sort_by(|a, b| {
                let c = a.duration.cmp(&b.duration);
                if asc {
                    c
                } else {
                    c.reverse()
                }
            }),
        }
        self.invalidate_wrap_cache();
    }

    fn cycle_sort(&mut self) {
        self.sort_field = self.sort_field.next();
        self.apply_sort();
        self.set_status(format!(
            "Sort: {} {}",
            self.sort_field.label(),
            if self.sort_order == SortOrder::Asc {
                "↑"
            } else {
                "↓"
            }
        ));
    }

    fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Asc => SortOrder::Desc,
            SortOrder::Desc => SortOrder::Asc,
        };
        self.apply_sort();
        self.set_status(format!(
            "Sort: {} {}",
            self.sort_field.label(),
            if self.sort_order == SortOrder::Asc {
                "↑"
            } else {
                "↓"
            }
        ));
    }

    fn toggle_shuffle(&mut self) {
        self.player.toggle_shuffle();
        self.set_status(format!(
            "Shuffle: {}",
            if self.player.shuffle { "on" } else { "off" }
        ));
    }

    fn toggle_loop(&mut self) {
        self.player.toggle_loop();
        self.set_status(format!("Loop: {}", self.player.loop_mode.label()));
    }

    fn cycle_panel_forward(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Sidebar => Panel::TrackList,
            Panel::TrackList => Panel::Queue,
            Panel::Queue => {
                if self.lyrics_visible {
                    self.load_lyrics();
                    Panel::Lyrics
                } else {
                    Panel::Sidebar
                }
            }
            Panel::Lyrics => Panel::Sidebar,
        };
    }

    fn cycle_panel_backward(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Sidebar => {
                if self.lyrics_visible {
                    self.load_lyrics();
                    Panel::Lyrics
                } else {
                    Panel::Queue
                }
            }
            Panel::TrackList => Panel::Sidebar,
            Panel::Queue => Panel::TrackList,
            Panel::Lyrics => Panel::Queue,
        };
    }

    fn toggle_lyrics_panel(&mut self) {
        if self.lyrics_visible {
            // Hide lyrics panel
            self.lyrics_visible = false;
            // If currently on lyrics panel, switch to queue
            if self.active_panel == Panel::Lyrics {
                self.active_panel = Panel::Queue;
            }
        } else {
            // Show lyrics panel
            self.lyrics_visible = true;
            self.load_lyrics();
            self.active_panel = Panel::Lyrics;
        }
    }

    fn toggle_art_window(&mut self) {
        self.art_window_visible = !self.art_window_visible;
    }

    fn enter_search(&mut self) {
        self.search_mode = true;
        self.track_context = TrackContext::Library;
        // Switch to track list panel so keypresses go to the search bar
        self.active_panel = Panel::TrackList;
    }

    fn play_selected(&mut self) {
        if self.filtered_tracks.is_empty() {
            return;
        }
        let idx = self.track_list_index.min(self.filtered_tracks.len() - 1);

        // If queue is empty, start a new queue from filtered tracks.
        // Otherwise, just play the selected track from the current queue.
        if self.player.queue.is_empty() {
            self.player.set_queue(self.filtered_tracks.clone(), idx);
            // Update UI's queue_index to match
            self.queue_index = idx;
        } else {
            // Add the selected track to the queue if not already there
            let track = self.filtered_tracks[idx].clone();
            if !self.player.queue.iter().any(|t| t.path == track.path) {
                self.player.queue.push(track.clone());
            }
            // Set queue index to this newly added (or existing) track
            if let Some(pos) = self
                .player
                .queue
                .iter()
                .position(|t| t.path == self.filtered_tracks[idx].path)
            {
                self.player.queue_index = pos;
                // Update UI's queue_index to match
                self.queue_index = pos;
            }
        }

        let track = self.player.queue[self.player.queue_index].clone();
        match self.player.play_track(track) {
            Ok(_) => {
                self.set_status(format!(
                    "Playing: {} — {}",
                    self.player
                        .current_track
                        .as_ref()
                        .map(|t| t.display_title())
                        .unwrap_or(""),
                    self.player
                        .current_track
                        .as_ref()
                        .map(|t| t.display_artist())
                        .unwrap_or("")
                ));
                self.refresh_album_art();
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e));
            }
        }
    }

    fn add_selected_to_queue(&mut self) {
        if self.selected_tracks.is_empty() {
            // Add single track
            if let Some(track) = self.filtered_tracks.get(self.track_list_index) {
                let title = track.display_title().to_string();
                self.player.add_to_queue(track.clone());
                // Update UI's queue_index to point to the newly added track
                self.queue_index = self.player.queue.len().saturating_sub(1);
                self.set_status(format!("Added to queue: {}", title));
            }
        } else {
            // Add multiple selected tracks
            let count = self.selected_tracks.len();
            let indices = self.get_selected_indices_ascending();

            for &idx in &indices {
                if let Some(track) = self.filtered_tracks.get(idx) {
                    self.player.add_to_queue(track.clone());
                }
            }

            // Update UI's queue_index to point to the first newly added track
            self.queue_index = self.player.queue.len().saturating_sub(count);
            self.set_status(format!("Added {} tracks to queue", count));

            // Clear selection after adding
            self.clear_selection();
        }
    }

    fn add_all_to_queue(&mut self) {
        let n = self.filtered_tracks.len();
        for t in &self.filtered_tracks {
            self.player.queue.push(t.clone());
        }
        // Update UI's queue_index to point to the first newly added track
        if n > 0 {
            self.queue_index = self.player.queue.len().saturating_sub(n);
        }
        self.set_status(format!("Added {} tracks to queue", n));
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    /// Call this after any change to filtered_tracks so the wrap cache is rebuilt.
    pub fn invalidate_wrap_cache(&mut self) {
        self.wrapped_width = 0; // force rebuild on next render
    }

    /// Rebuild the wrap cache for the given total panel width.
    /// Called from ui.rs when the width changes or cache is invalid.
    pub fn rebuild_wrapped_tracks(&mut self, panel_w: usize) {
        let title_w1 = (panel_w * 28 / 100).saturating_sub(2);
        let artist_w = panel_w * 22 / 100;
        let album_w = panel_w * 22 / 100;
        let genre_w = panel_w * 16 / 100;

        self.wrapped_tracks = self
            .filtered_tracks
            .iter()
            .map(|track| {
                let title = track.display_title();
                let artist = track.display_artist();
                let album = track.display_album();
                let genre = track.genre.split(',').next().unwrap_or("").trim();
                let dur = track.duration_str();

                // ── Collapsed: truncate each field with … if it overflows ─────────
                let t0 = truncate_field(title, title_w1);
                let a0 = truncate_field(artist, artist_w);
                let b0 = truncate_field(album, album_w);
                let g0 = truncate_field(genre, genre_w);
                let collapsed = format!("  {}  {}  {}  {}  {:>4}", t0, a0, b0, g0, dur);

                // ── Expanded: word-wrap each field across as many rows as needed ──
                let tc = wrap_field(title, title_w1);
                let ac = wrap_field(artist, artist_w);
                let bc = wrap_field(album, album_w);
                let gc = wrap_field(genre, genre_w);

                let n = [tc.len(), ac.len(), bc.len(), gc.len()]
                    .into_iter()
                    .max()
                    .unwrap_or(1)
                    .max(1);

                let empty_t = " ".repeat(title_w1);
                let empty_a = " ".repeat(artist_w);
                let empty_b = " ".repeat(album_w);
                let empty_g = " ".repeat(genre_w);

                let expanded = (0..n)
                    .map(|row| {
                        let t = tc
                            .get(row)
                            .map(|s| pad_to(s, title_w1))
                            .unwrap_or_else(|| empty_t.clone());
                        let a = ac
                            .get(row)
                            .map(|s| pad_to(s, artist_w))
                            .unwrap_or_else(|| empty_a.clone());
                        let b = bc
                            .get(row)
                            .map(|s| pad_to(s, album_w))
                            .unwrap_or_else(|| empty_b.clone());
                        let g = gc
                            .get(row)
                            .map(|s| pad_to(s, genre_w))
                            .unwrap_or_else(|| empty_g.clone());
                        if row == 0 {
                            format!("  {}  {}  {}  {}  {:>4}", t, a, b, g, dur)
                        } else {
                            format!("  {}  {}  {}  {}", t, a, b, g)
                        }
                    })
                    .collect();

                (collapsed, expanded)
            })
            .collect();

        self.wrapped_width = panel_w;
    }

    /// Clear all album art state and caches
    fn clear_album_art(&mut self) {
        self.album_art = None;
        self.album_art_image = None;
        self.album_art_path = None;
        self.album_art_bytes = None;
        self.mpris_art_url = None;
        self.cached_art_window_block = None;
        self.cached_art_window_dims = (0, 0);
        self.cached_art_window_protocol = None;
    }

    /// Get selected track indices sorted in ascending order
    fn get_selected_indices_ascending(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected_tracks.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    /// Get selected track indices sorted in descending order
    fn get_selected_indices_descending(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected_tracks.iter().copied().collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices
    }

    /// Perform fuzzy matching on a list of items and return matches sorted by score
    fn fuzzy_match_items(&self, items: &[String], query: &str) -> Vec<String> {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, String)> = items
            .iter()
            .filter_map(|item| {
                matcher.fuzzy_match(item, query).map(|s| (s, item.clone()))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, item)| item).collect()
    }

    /// Get the current playlist name if in playlist context, otherwise returns None
    fn current_playlist_name(&self) -> Option<String> {
        match &self.track_context {
            TrackContext::Playlist(name) => Some(name.clone()),
            _ => None,
        }
    }

    /// Execute an operation on the current playlist and sync state
    /// Returns true if the operation was performed
    fn with_current_playlist<F>(&mut self, mut operation: F) -> bool
    where
        F: FnMut(&mut crate::playlist::Playlist),
    {
        let pl_name = match self.current_playlist_name() {
            Some(name) => name,
            None => return false,
        };

        if let Some(pl) = self.playlists.iter_mut().find(|p| p.name == pl_name) {
            operation(pl);
            let _ = pl.save();
            self.filtered_tracks = self.load_playlist_tracks(&pl_name);
            true
        } else {
            false
        }
    }

    pub fn refresh_album_art(&mut self) {
        let path = match self.player.current_track.as_ref().map(|t| t.path.clone()) {
            Some(p) => p,
            None => {
                self.clear_album_art();
                return;
            }
        };
        if self.album_art_path.as_deref() == Some(&path) {
            return;
        }
        self.album_art_path = Some(path.clone());
        self.mpris_art_url = None; // will be lazily re-extracted by main.rs
        // Clear cached art window rendering since track changed
        self.cached_art_window_block = None;
        self.cached_art_window_dims = (0, 0);
        self.cached_art_window_protocol = None;

        // Load album art: extract once and cache the bytes
        if let Some(bytes) = crate::art::extract_cover_bytes(&path) {
            self.album_art_image = crate::art::load_cover_image(&bytes);
            // Small version for player bar
            self.album_art = crate::art::render_block_art(&bytes, 8, 3);
            // Cache the bytes to avoid re-extraction
            self.album_art_bytes = Some(bytes);
        } else {
            self.clear_album_art();
        }
    }

    /// Get the index of the currently active lyric line based on playback position.
    /// Returns None if lyrics are not timed or no lyrics are loaded.
    pub fn current_lyric_index(&self) -> Option<usize> {
        use crate::types::Lyrics;

        if let Some(Lyrics::Timed(lines)) = &self.lyrics_content {
            let elapsed = self.player.elapsed_secs() as f64;

            // Find the last line whose timestamp has passed
            let mut current_index = None;
            for (i, line) in lines.iter().enumerate() {
                if let Some(timestamp) = line.timestamp {
                    if timestamp <= elapsed + 0.5 {
                        current_index = Some(i);
                    } else {
                        break;
                    }
                }
            }
            current_index
        } else {
            None
        }
    }
}

/// Split `s` into lines where each line's display width ≤ `max` columns.
/// Breaks on whitespace where possible; falls back to character-breaking
/// only when a single word is wider than `max`.
fn wrap_field(s: &str, max: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;
    use unicode_width::UnicodeWidthStr;
    if max == 0 || s.is_empty() {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for word in s.split_whitespace() {
        let word_w = word.width();

        // If adding this word (plus a space if current is non-empty) fits, append it
        let sep_w = if current.is_empty() { 0 } else { 1 };
        if current_w + sep_w + word_w <= max {
            if !current.is_empty() {
                current.push(' ');
                current_w += 1;
            }
            current.push_str(word);
            current_w += word_w;
        } else if word_w > max {
            // Word is wider than the column — flush current line then
            // character-break the word itself across as many lines as needed
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            let mut char_buf = String::new();
            let mut char_w = 0usize;
            for ch in word.chars() {
                let cw = ch.width().unwrap_or(1);
                if char_w + cw > max {
                    lines.push(char_buf.clone());
                    char_buf.clear();
                    char_w = 0;
                }
                char_buf.push(ch);
                char_w += cw;
            }
            // whatever's left becomes the new current line
            current = char_buf;
            current_w = char_w;
        } else {
            // Word fits on a new line — flush current and start fresh
            if !current.is_empty() {
                lines.push(current.clone());
            }
            current = word.to_string();
            current_w = word_w;
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Pad `s` to exactly `width` display columns by appending spaces.
/// Rust's `{:<width$}` uses char count, not display width — this fixes that.
fn pad_to(s: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let display_w = s.width();
    if display_w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_w))
    }
}

/// Truncate `s` to `max` display columns, appending `…` if it was cut.
/// Pads with spaces to exactly `max` columns. Unicode-aware.
fn truncate_field(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    use unicode_width::UnicodeWidthStr;
    if max == 0 {
        return String::new();
    }
    if s.width() <= max {
        return pad_to(s, max);
    }

    // Build up to max-1 display cols, then add …
    let mut result = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(1);
        if w + cw > max - 1 {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result.push('…');
    w += 1;
    // Pad if the ellipsis itself left a gap (e.g. last char was 2-wide)
    if w < max {
        result.push_str(&" ".repeat(max - w));
    }
    result
}

/// Helper function to convert an action back to a representative KeyCode
/// This is used for the panel-specific handlers that still use KeyCode matching
fn key_to_code(action: Action) -> KeyCode {
    match action {
        Action::MoveUp => KeyCode::Up,
        Action::MoveDown => KeyCode::Down,
        Action::MoveLeft => KeyCode::Left,
        Action::MoveRight => KeyCode::Right,
        Action::PageUp => KeyCode::PageUp,
        Action::PageDown => KeyCode::PageDown,
        Action::GoToTop => KeyCode::Home,
        Action::GoToBottom => KeyCode::End,
        _ => KeyCode::Null,
    }
}
