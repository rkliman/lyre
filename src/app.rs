use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, mpsc};

use crate::db::Db;
use crate::keybindings::{Action, Keybindings, key_to_code};
use crate::player::Player;
use crate::playlist::{scan_playlists, Playlist};
use crate::state::StateDb;
use crate::types::{
    GlobalSearchResult, LoopMode, LyricsFetchStatus, LyricsState, Overlay, Panel, PlayerState,
    Result, SetupField, SidebarItem, SidebarSection, SidebarSectionState, SidebarSections,
    SortField, SortOrder, Track, TrackContext,
};

fn default_sidebar_expanded() -> HashMap<String, bool> {
    [
        ("Artists".to_string(), false),
        ("Albums".to_string(), false),
        ("Genres".to_string(), false),
        ("Playlists".to_string(), false),
    ]
    .into()
}

pub enum LyricsFetchResult {
    Found { path: String, text: String },
    NotFound,
    Error(String),
}
use crate::util::{expand_tilde, pad_to, truncate_field, wrap_field, FAVORITE_ICON};
use crate::colors::ColorScheme;
use crate::config::{load_config, save_config, Config};

pub struct App {
    pub colors: ColorScheme,
    pub keybindings: Keybindings,
    pub all_tracks: Vec<Arc<Track>>,
    pub filtered_tracks: Vec<Arc<Track>>,
    pub search_base_tracks: Vec<Arc<Track>>,
    pub matcher: SkimMatcherV2,
    pub track_context: TrackContext,
    pub track_heading: String,
    // Pre-computed display lines per track.
    // Each entry is (collapsed_line, expanded_lines).
    // collapsed_line: single row with … truncation for non-selected display.
    // expanded_lines: full word-wrapped rows shown when the track is selected.
    pub wrapped_tracks: Vec<(String, Vec<String>)>,
    pub wrapped_width: usize,
    pub sidebar: SidebarSections,
    pub sidebar_items: Vec<SidebarItem>,
    pub sidebar_index: usize,
    pub sidebar_offset: usize,
    pub sidebar_expanded: HashMap<String, bool>,
    pub sidebar_search_mode: bool,
    pub sidebar_search_query: String,
    pub sidebar_search_section: Option<String>,
    pub playlists: Vec<Playlist>,
    pub music_dir: String,
    pub db_path: String,
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
    pub art: crate::art::AlbumArtState,
    pub overlay: Overlay,
    pub global_search_query: String,
    pub global_search_selected: usize,
    pub global_search_results: Vec<GlobalSearchResult>,
    pub show_help: bool,
    pub help_scroll: usize,
    pub show_info: bool,
    /// True when the popup shows the detailed (all fields) view.
    pub info_detailed: bool,
    /// True when the track info popup is in edit mode (fields editable).
    pub info_editing: bool,
    /// Which field is currently being edited.
    pub info_edit_field: crate::types::EditField,
    /// Cached editable field values for the currently-focused track.
    /// Populated lazily when the popup is opened / focus changes.
    pub info_field_values: HashMap<crate::types::EditField, String>,
    /// Path the cache in `info_field_values` was populated for.
    pub info_field_track_path: Option<String>,
    /// Read-only technical info (format, bitrate, etc.) for the focused track.
    pub info_readonly: Vec<(String, String)>,
    /// Vertical scroll offset for the field list in the popup.
    pub info_scroll: usize,
    /// Maximum valid scroll offset, updated by the renderer each frame.
    pub info_max_scroll: usize,
    pub lyrics: LyricsState,
    pub art_window_visible: bool,
    duration_rx: Option<mpsc::Receiver<(String, i64)>>,
    lyrics_fetch_rx: Option<mpsc::Receiver<LyricsFetchResult>>,
    /// Fetched lyrics not yet persisted to file tags. Keyed by track path.
    /// We defer writes until the track is no longer playing, since rewriting
    /// tags on the actively-streamed file corrupts decoding.
    pending_lyrics_writes: HashMap<String, String>,
    state_db: Option<StateDb>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = load_config();
        Self::new_with_config(config)
    }

    fn new_with_config(config: Config) -> Result<Self> {
        let colors = ColorScheme::from_config(&config.ui.colors);
        let music_dir = expand_tilde(&config.files.music_directory);
        let db_path = expand_tilde(&config.files.database_name);
        let db_path_exists = Path::new(&db_path).exists();

        let setup_overlay = Overlay::SetupDatabase {
            database_name: config.files.database_name.clone(),
            music_directory: config.files.music_directory.clone(),
            active_field: SetupField::DatabaseName,
        };

        if !db_path_exists {
            return Self::new_empty(
                colors,
                music_dir,
                db_path,
                Some("No database found. Enter a music directory and database file path to continue.".to_string()),
                setup_overlay,
            );
        }

        match Db::open(&db_path, &music_dir) {
            Ok(db) => {
                let all_tracks: Vec<Arc<Track>> = db
                    .all_tracks()
                    .unwrap_or_default()
                    .into_iter()
                    .map(Arc::new)
                    .collect();
                let filtered_tracks = all_tracks.clone();
                let search_base_tracks = all_tracks.clone();
                let artists = db.distinct_artists().unwrap_or_default();
                let albums = db.distinct_albums().unwrap_or_default();
                let genres = db.distinct_genres().unwrap_or_default();
                let playlists = scan_playlists(&music_dir);
                let playlist_names: Vec<String> = playlists.iter().map(|p| p.name.clone()).collect();
                let sidebar_expanded = default_sidebar_expanded();

                let mut player = Player::new().expect("Failed to initialize audio.");
                let image_picker = crate::art::create_picker();
                let keybindings = Keybindings::new();

                let state_db = StateDb::open().ok();
                let mut restored_queue_index: usize = 0;
                if let Some(ref sdb) = state_db {
                    if let Ok(paths) = sdb.load_queue_paths() {
                        let track_map: HashMap<&str, &Arc<Track>> = all_tracks.iter()
                            .map(|t| (t.path.as_str(), t))
                            .collect();
                        let queue: Vec<Arc<Track>> = paths.iter()
                            .filter_map(|p| track_map.get(p.as_str()).map(|t| Arc::clone(t)))
                            .collect();
                        if !queue.is_empty() {
                            restored_queue_index = sdb.load_state("queue_index")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(0)
                                .min(queue.len() - 1);
                            player.queue_index = restored_queue_index;
                            player.queue = queue;
                        }
                    }
                    if let Some(v) = sdb.load_state("volume").and_then(|s| s.parse::<f32>().ok()) {
                        player.volume = v.clamp(0.0, 1.0);
                    }
                    if let Some(s) = sdb.load_state("shuffle") {
                        player.shuffle = s == "true";
                    }
                    if let Some(lm) = sdb.load_state("loop_mode") {
                        player.loop_mode = match lm.as_str() {
                            "all" => LoopMode::All,
                            "one" => LoopMode::One,
                            _ => LoopMode::Off,
                        };
                    }
                }

                let missing: Vec<String> = all_tracks
                    .iter()
                    .filter(|t| t.duration <= 0)
                    .map(|t| t.path.clone())
                    .collect();

                let duration_rx = if missing.is_empty() {
                    None
                } else {
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        use rayon::prelude::*;
                        missing.par_iter().for_each(|path| {
                            if let Some(dur) = crate::art::extract_duration(path) {
                                let _ = tx.send((path.clone(), dur));
                            }
                        });
                    });
                    Some(rx)
                };

                let mut app = Self {
                    colors,
                    keybindings,
                    all_tracks,
                    filtered_tracks,
                    search_base_tracks,
                    matcher: SkimMatcherV2::default(),
                    track_context: TrackContext::Library,
                    track_heading: "All Tracks".to_string(),
                    wrapped_tracks: Vec::new(),
                    wrapped_width: 0,
                    sidebar: SidebarSections {
                        artists: SidebarSectionState { items: artists, filter_indices: None },
                        albums: SidebarSectionState { items: albums, filter_indices: None },
                        genres: SidebarSectionState { items: genres, filter_indices: None },
                        playlists: SidebarSectionState {
                            items: playlist_names,
                            filter_indices: None,
                        },
                    },
                    sidebar_items: Vec::new(),
                    sidebar_index: 0,
                    sidebar_offset: 0,
                    sidebar_expanded,
                    sidebar_search_mode: false,
                    sidebar_search_query: String::new(),
                    sidebar_search_section: None,
                    playlists,
                    music_dir,
                    db_path,
                    track_list_index: 0,
                    track_list_offset: 0,
                    selected_tracks: HashSet::new(),
                    selection_anchor: None,
                    queue_index: restored_queue_index,
                    active_panel: Panel::Sidebar,
                    sort_field: SortField::Artist,
                    sort_order: SortOrder::Asc,
                    search_mode: false,
                    search_query: String::new(),
                    player,
                    status_message: None,
                    art: crate::art::AlbumArtState {
                        picker: image_picker,
                        ..Default::default()
                    },
                    overlay: Overlay::None,
                    global_search_query: String::new(),
                    global_search_selected: 0,
                    global_search_results: Vec::new(),
                    show_help: false,
                    help_scroll: 0,
                    show_info: false,
                    info_detailed: false,
                    info_editing: false,
                    info_edit_field: crate::types::EditField::Title,
                    info_field_values: HashMap::new(),
                    info_field_track_path: None,
                    info_readonly: Vec::new(),
                    info_scroll: 0,
                    info_max_scroll: 0,
                    lyrics: LyricsState::default(),
                    art_window_visible: false,
                    duration_rx,
                    lyrics_fetch_rx: None,
                    pending_lyrics_writes: HashMap::new(),
                    state_db,
                };
                app.rebuild_sidebar();
                app.apply_sort();
                Ok(app)
            }
            Err(err) => Self::new_empty(
                colors,
                music_dir,
                db_path,
                Some(format!("Failed to open database: {}", err)),
                setup_overlay,
            ),
        }
    }

    fn new_empty(
        colors: ColorScheme,
        music_dir: String,
        db_path: String,
        status_message: Option<String>,
        overlay: Overlay,
    ) -> Result<Self> {
        let playlists = scan_playlists(&music_dir);
        let playlist_names: Vec<String> = playlists.iter().map(|p| p.name.clone()).collect();
        let sidebar_expanded = default_sidebar_expanded();

        let player = Player::new().expect("Failed to initialize audio.");
        let image_picker = crate::art::create_picker();
        let keybindings = Keybindings::new();

        let mut app = Self {
            colors,
            keybindings,
            all_tracks: Vec::new(),
            filtered_tracks: Vec::new(),
            search_base_tracks: Vec::new(),
            matcher: SkimMatcherV2::default(),
            track_context: TrackContext::Library,
            track_heading: "All Tracks".to_string(),
            wrapped_tracks: Vec::new(),
            wrapped_width: 0,
            sidebar: SidebarSections {
                playlists: SidebarSectionState {
                    items: playlist_names,
                    filter_indices: None,
                },
                ..Default::default()
            },
            sidebar_items: Vec::new(),
            sidebar_index: 0,
            sidebar_offset: 0,
            sidebar_expanded,
            sidebar_search_mode: false,
            sidebar_search_query: String::new(),
            sidebar_search_section: None,
            playlists,
            music_dir,
            db_path,
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
            status_message,
            art: crate::art::AlbumArtState {
                picker: image_picker,
                ..Default::default()
            },
            overlay,
            global_search_query: String::new(),
            global_search_selected: 0,
            global_search_results: Vec::new(),
            show_help: false,
            help_scroll: 0,
            show_info: false,
            info_detailed: false,
            info_editing: false,
            info_edit_field: crate::types::EditField::Title,
            info_field_values: HashMap::new(),
            info_field_track_path: None,
            info_readonly: Vec::new(),
            info_scroll: 0,
            info_max_scroll: 0,
            lyrics: LyricsState::default(),
            art_window_visible: false,
            duration_rx: None,
            lyrics_fetch_rx: None,
            pending_lyrics_writes: HashMap::new(),
            state_db: StateDb::open().ok(),
        };

        app.rebuild_sidebar();
        app.apply_sort();
        Ok(app)
    }

    pub fn rebuild_sidebar(&mut self) {
        let mut items = vec![
            SidebarItem::AllTracks,
            SidebarItem::Favorites,
            SidebarItem::RecentlyAdded,
        ];

        let artists_open = *self.sidebar_expanded.get("Artists").unwrap_or(&true);
        items.push(SidebarItem::Artists);
        if artists_open {
            for a in self.sidebar.artists.visible() {
                items.push(SidebarItem::Artist(a.clone()));
            }
        }

        let albums_open = *self.sidebar_expanded.get("Albums").unwrap_or(&true);
        items.push(SidebarItem::Albums);
        if albums_open {
            for a in self.sidebar.albums.visible() {
                items.push(SidebarItem::Album(a.clone()));
            }
        }

        let genres_open = *self.sidebar_expanded.get("Genres").unwrap_or(&true);
        items.push(SidebarItem::Genres);
        if genres_open {
            for g in self.sidebar.genres.visible() {
                items.push(SidebarItem::Genre(g.clone()));
            }
        }

        let playlists_open = *self.sidebar_expanded.get("Playlists").unwrap_or(&true);
        items.push(SidebarItem::Playlists);
        if playlists_open {
            for pl in self.sidebar.playlists.visible() {
                items.push(SidebarItem::Playlist(pl.clone()));
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
            for sec in SidebarSection::ALL {
                self.sidebar.get_mut(sec).filter_indices = None;
            }
            return;
        }

        let Some(section) = SidebarSection::from_key(section) else {
            return;
        };
        let query = self.sidebar_search_query.clone();
        let matcher = &self.matcher;
        let state = self.sidebar.get_mut(section);
        let mut scored: Vec<(i64, usize)> = state
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| matcher.fuzzy_match(item, &query).map(|s| (s, i)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        state.filter_indices = Some(scored.into_iter().map(|(_, i)| i).collect());
    }

    pub fn tick(&mut self) {
        if self.player.poll_track_transition() {
            self.refresh_album_art();
            self.persist_queue();
        }
        self.player.maybe_prebuffer_next();
        if self.player.is_finished() {
            self.player.stop();
        }

        // Flush any pending lyric tag writes for tracks not currently playing.
        // Writing tags to the actively-streamed file corrupts decoding, so we
        // defer until the player has moved on.
        if !self.pending_lyrics_writes.is_empty() {
            let current = self.player.current_track.as_ref().map(|t| t.path.clone());
            let flushable: Vec<String> = self
                .pending_lyrics_writes
                .keys()
                .filter(|p| current.as_deref() != Some(p.as_str()))
                .cloned()
                .collect();
            for path in flushable {
                if let Some(text) = self.pending_lyrics_writes.remove(&path) {
                    crate::lyrics::write_lyrics_to_tag(&path, &text);
                }
            }
        }

        // Auto-reload lyrics when track changes
        if self.lyrics.visible {
            let current_track_path = self.player.current_track.as_ref().map(|t| t.path.clone());
            if current_track_path != self.lyrics.track_path {
                self.load_lyrics();
            }
        }

        // Drain background lyrics fetch results
        if let Some(rx) = &self.lyrics_fetch_rx {
            match rx.try_recv() {
                Ok(LyricsFetchResult::Found { path, text }) => {
                    self.lyrics.content = Some(crate::lyrics::parse_lyrics(&text));
                    self.lyrics.fetch_status = LyricsFetchStatus::Idle;
                    self.lyrics.scroll = 0;
                    self.lyrics.manual_scroll = false;
                    self.pending_lyrics_writes.insert(path, text);
                    self.lyrics_fetch_rx = None;
                    self.set_status("Lyrics fetched from lrclib".to_string());
                }
                Ok(LyricsFetchResult::NotFound) => {
                    self.lyrics.fetch_status = LyricsFetchStatus::NotFound;
                    self.lyrics_fetch_rx = None;
                }
                Ok(LyricsFetchResult::Error(e)) => {
                    self.lyrics.fetch_status = LyricsFetchStatus::Error(e);
                    self.lyrics_fetch_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    if matches!(self.lyrics.fetch_status, LyricsFetchStatus::Fetching) {
                        self.lyrics.fetch_status =
                            LyricsFetchStatus::Error("channel closed".to_string());
                    }
                    self.lyrics_fetch_rx = None;
                }
            }
        }

        // Drain background duration updates
        if self.duration_rx.is_some() {
            let mut updates: Vec<(String, i64)> = Vec::new();
            let mut done = false;
            let rx = self.duration_rx.as_ref().unwrap();
            loop {
                match rx.try_recv() {
                    Ok(update) => updates.push(update),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if !updates.is_empty() {
                for (ref path, dur) in &updates {
                    for t in &mut self.all_tracks {
                        if t.path == *path {
                            Arc::make_mut(t).duration = *dur;
                            break;
                        }
                    }
                    for t in &mut self.filtered_tracks {
                        if t.path == *path {
                            Arc::make_mut(t).duration = *dur;
                            break;
                        }
                    }
                    for t in &mut self.player.queue {
                        if t.path == *path {
                            Arc::make_mut(t).duration = *dur;
                            break;
                        }
                    }
                }
                if let Ok(db) = Db::open(&self.db_path, &self.music_dir) {
                    let _ = db.update_durations(&updates);
                }
            }
            if done {
                self.duration_rx = None;
                self.invalidate_wrap_cache();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if matches!(self.overlay, Overlay::GlobalSearch) {
            return self.handle_global_search_key(key.code);
        }

        // Handle sidebar search mode before keybindings lookup
        if self.sidebar_search_mode {
            return self.handle_sidebar_search_key(key.code);
        }

        // Handle track-info popup keys before normal keybindings lookup.
        if self.show_info {
            // 'd' toggles between simple and detailed views (works in view & edit modes)
            if key.code == KeyCode::Char('d') {
                self.toggle_info_detailed();
                return false;
            }
            if self.info_editing {
                self.handle_info_edit_key(key.code);
                return false;
            }
            if key.code == KeyCode::Char('e') {
                self.enter_info_edit_mode();
                return false;
            }
            // In view mode: up/down (and vim keys) scroll the info panel
            // instead of moving through the track list.
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.info_scroll_by(-1);
                    return false;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.info_scroll_by(1);
                    return false;
                }
                KeyCode::PageUp => {
                    self.info_scroll_by(-8);
                    return false;
                }
                KeyCode::PageDown => {
                    self.info_scroll_by(8);
                    return false;
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.info_scroll = 0;
                    return false;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.info_scroll = self.info_max_scroll;
                    return false;
                }
                _ => {}
            }
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
                    if self.active_panel == Panel::Queue {
                        self.play_queue_selected();
                    } else {
                        self.play_selected();
                    }
                } else {
                    self.player.toggle_pause();
                }
            }
            Action::Next => {
                let _ = self.player.next();
                self.refresh_album_art();
                self.persist_queue();
            }
            Action::Previous => {
                let _ = self.player.prev();
                self.refresh_album_art();
                self.persist_queue();
            }
            Action::VolumeUp => {
                self.player.volume_up();
                self.persist_player_setting("volume", &self.player.volume.to_string());
            }
            Action::VolumeDown => {
                self.player.volume_down();
                self.persist_player_setting("volume", &self.player.volume.to_string());
            }
            Action::SeekForward => {
                let _ = self.player.seek_forward(std::time::Duration::from_secs(5));
            }
            Action::SeekBackward => {
                let _ = self.player.seek_backward(std::time::Duration::from_secs(5));
            }
            Action::ToggleShuffle => {
                self.toggle_shuffle();
                self.persist_player_setting("shuffle", &self.player.shuffle.to_string());
            }
            Action::ToggleLoop => {
                self.toggle_loop();
                self.persist_player_setting("loop_mode", self.player.loop_mode.label());
            }

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
                    Panel::Queue => self.play_queue_selected(),
                    Panel::Lyrics => {
                        if self.lyrics.content.is_none()
                            && self.player.current_track.is_some()
                            && !matches!(self.lyrics.fetch_status, LyricsFetchStatus::Fetching)
                        {
                            self.fetch_lyrics_from_lrclib();
                        }
                    }
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
                            SidebarItem::Playlist(_) => Some("Playlists"),
                            _ => None,
                        };
                        if let Some(s) = section {
                            if *self.sidebar_expanded.get(s).unwrap_or(&true) {
                                self.sidebar_search_mode = true;
                                self.sidebar_search_section = Some(s.to_string());
                            }
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
                self.remove_from_queue(self.queue_index);
            }
            Action::ClearQueue => {
                self.player.stop();
                self.player.clear_queue();
                self.queue_index = 0;
                self.clear_album_art();
                self.persist_queue();
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
                    self.filtered_tracks = self.search_base_tracks.clone();
                    if !matches!(self.track_context, TrackContext::Playlist(_)) {
                        self.apply_sort();
                    }
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

            // Info
            Action::ToggleInfo => {
                self.show_info = !self.show_info;
                if !self.show_info {
                    self.info_editing = false;
                    self.info_detailed = false;
                    self.info_scroll = 0;
                    self.info_field_track_path = None;
                    self.info_field_values.clear();
                    self.info_readonly.clear();
                }
            }

            Action::ToggleFavorite => self.toggle_favorite_selected(),

            Action::GlobalSearch => {
                self.overlay = Overlay::GlobalSearch;
                self.global_search_query.clear();
                self.global_search_selected = 0;
                self.global_search_results.clear();
            }

            Action::InfoClose => {
                self.show_info = false;
                self.info_editing = false;
                self.info_detailed = false;
                self.info_scroll = 0;
                self.info_field_track_path = None;
                self.info_field_values.clear();
                self.info_readonly.clear();
            }

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
            Overlay::SetupDatabase {
                database_name,
                music_directory,
                ..
            } => {
                let expanded_db_path = expand_tilde(database_name);
                let expanded_music_dir = expand_tilde(music_directory);
                self.music_dir = expanded_music_dir.clone();
                self.db_path = expanded_db_path.clone();

                if let Some(parent) = Path::new(&expanded_db_path).parent() {
                    if let Err(err) = std::fs::create_dir_all(parent) {
                        self.status_message = Some(format!("Could not create database directory: {}", err));
                        return;
                    }
                }

                match Db::open(&expanded_db_path, &expanded_music_dir) {
                    Ok(db) => {
                        self.all_tracks = db
                            .all_tracks()
                            .unwrap_or_default()
                            .into_iter()
                            .map(Arc::new)
                            .collect();
                        self.filtered_tracks = self.all_tracks.clone();
                        let artists = db.distinct_artists().unwrap_or_default();
                        let albums = db.distinct_albums().unwrap_or_default();
                        let genres = db.distinct_genres().unwrap_or_default();
                        self.playlists = scan_playlists(&expanded_music_dir);
                        let playlist_names: Vec<String> = self.playlists.iter().map(|p| p.name.clone()).collect();
                        self.sidebar.artists = SidebarSectionState { items: artists, filter_indices: None };
                        self.sidebar.albums = SidebarSectionState { items: albums, filter_indices: None };
                        self.sidebar.genres = SidebarSectionState { items: genres, filter_indices: None };
                        self.sidebar.playlists = SidebarSectionState {
                            items: playlist_names,
                            filter_indices: None,
                        };
                        self.status_message = Some("Database created and loaded successfully.".to_string());
                        self.overlay = Overlay::None;
                        self.rebuild_sidebar();
                        self.apply_sort();

                        let mut config = load_config();
                        config.files.database_name = database_name.clone();
                        config.files.music_directory = music_directory.clone();
                        let _ = save_config(&config);
                    }
                    Err(err) => {
                        self.status_message = Some(format!("Could not open database: {}", err));
                    }
                }
            }
            Overlay::GlobalSearch | Overlay::None => {}
        }
    }

    fn handle_overlay_char(&mut self, c: char) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                let mut name = name.clone();
                name.push(c);
                self.overlay = Overlay::NewPlaylist(name);
            }
            Overlay::SetupDatabase {
                database_name,
                music_directory,
                active_field,
            } => {
                let mut database_name = database_name.clone();
                let mut music_directory = music_directory.clone();
                match active_field {
                    SetupField::DatabaseName => database_name.push(c),
                    SetupField::MusicDirectory => music_directory.push(c),
                }
                self.overlay = Overlay::SetupDatabase {
                    database_name,
                    music_directory,
                    active_field: active_field.clone(),
                };
            }
            Overlay::AddToPlaylist { .. } => {}
            Overlay::GlobalSearch | Overlay::None => {}
        }
    }

    fn handle_overlay_backspace(&mut self) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                let mut name = name.clone();
                name.pop();
                self.overlay = Overlay::NewPlaylist(name);
            }
            Overlay::SetupDatabase {
                database_name,
                music_directory,
                active_field,
            } => {
                let mut database_name = database_name.clone();
                let mut music_directory = music_directory.clone();
                match active_field {
                    SetupField::DatabaseName => {
                        database_name.pop();
                    }
                    SetupField::MusicDirectory => {
                        music_directory.pop();
                    }
                }
                self.overlay = Overlay::SetupDatabase {
                    database_name,
                    music_directory,
                    active_field: active_field.clone(),
                };
            }
            Overlay::AddToPlaylist { .. } => {}
            Overlay::GlobalSearch | Overlay::None => {}
        }
    }

    fn handle_overlay_navigate(&mut self, delta: i32) {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(_) => {}
            Overlay::SetupDatabase {
                database_name,
                music_directory,
                active_field,
            } => {
                let next_field = match active_field {
                    SetupField::DatabaseName => SetupField::MusicDirectory,
                    SetupField::MusicDirectory => SetupField::DatabaseName,
                };
                self.overlay = Overlay::SetupDatabase {
                    database_name: database_name.clone(),
                    music_directory: music_directory.clone(),
                    active_field: next_field,
                };
            }
            Overlay::AddToPlaylist { track_paths, selected } => {
                let mut new_selected = *selected as i32 + delta;
                new_selected = new_selected.max(0).min(self.playlists.len().saturating_sub(1) as i32);
                self.overlay = Overlay::AddToPlaylist {
                    track_paths: track_paths.clone(),
                    selected: new_selected as usize,
                };
            }
            Overlay::GlobalSearch | Overlay::None => {}
        }
    }

    /// Handle common navigation keys for lists. Returns true if handled.
    /// Operates on a mutable index and max length.
    fn handle_list_navigation(key: KeyCode, index: &mut usize, max_len: usize) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if max_len > 0 {
                    *index = if *index == 0 { max_len - 1 } else { *index - 1 };
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if max_len > 0 {
                    *index = if *index + 1 >= max_len { 0 } else { *index + 1 };
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
            KeyCode::Left | KeyCode::Char('h') => {
                self.active_panel = Panel::TrackList;
            }
            KeyCode::Right | KeyCode::Char('l') if self.lyrics.visible => {
                self.active_panel = Panel::Lyrics;
            }
            _ => {}
        }
    }

    fn handle_lyrics_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = self.lyrics.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = self.lyrics.scroll.saturating_add(1);
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = self.lyrics.scroll.saturating_sub(10);
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = self.lyrics.scroll.saturating_add(10);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.lyrics.manual_scroll = true;
                self.lyrics.scroll = usize::MAX;
            }
            KeyCode::Char('r') => {
                self.load_lyrics();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.lyrics.manual_scroll = false;
                self.active_panel = Panel::Queue;
            }
            _ => {}
        }
    }

    /// Get the currently-focused track for the info popup (matches track_info.rs logic).
    fn info_focused_track(&self) -> Option<&Track> {
        if self.active_panel == Panel::Queue {
            if self.player.queue.is_empty() {
                None
            } else {
                Some(&*self.player.queue[self.queue_index.min(self.player.queue.len() - 1)])
            }
        } else if self.filtered_tracks.is_empty() {
            None
        } else {
            Some(&*self.filtered_tracks[self.track_list_index.min(self.filtered_tracks.len() - 1)])
        }
    }

    /// Return the ordered list of fields visible in the popup for the current mode.
    pub fn info_visible_fields(&self) -> &'static [crate::types::EditField] {
        if self.info_detailed {
            crate::types::EditField::DETAILED
        } else {
            crate::types::EditField::SIMPLE
        }
    }

    /// Ensure `info_field_values` and `info_readonly` are populated for the
    /// currently-focused track. Reads the file tag if the cache is stale.
    pub fn ensure_info_cache(&mut self) {
        let path = self.info_focused_track().map(|t| t.path.clone());
        let Some(path) = path else {
            self.info_field_values.clear();
            self.info_field_track_path = None;
            self.info_readonly.clear();
            return;
        };
        if self.info_field_track_path.as_deref() == Some(&path) {
            return;
        }
        let track = self.info_focused_track().cloned();
        let mut values = crate::lyrics::read_metadata_from_tag(&path);
        // Overlay core-field values from the in-memory Track (DB is source of truth).
        if let Some(t) = &track {
            use crate::types::EditField as F;
            values.insert(F::Title, t.title.clone());
            values.insert(F::Artist, t.artist.clone());
            values.insert(F::Album, t.album.clone());
            values.insert(F::AlbumArtist, t.albumartist.clone());
            values.insert(
                F::Year,
                if t.year > 0 { t.year.to_string() } else { String::new() },
            );
            values.insert(F::Genre, t.genre.clone());
        }
        self.info_field_values = values;
        self.info_readonly = crate::lyrics::read_file_properties(&path);
        self.info_field_track_path = Some(path);
    }

    fn info_scroll_by(&mut self, delta: i32) {
        let cur = self.info_scroll as i64 + delta as i64;
        let clamped = cur.max(0).min(self.info_max_scroll as i64) as usize;
        self.info_scroll = clamped;
    }

    fn toggle_info_detailed(&mut self) {
        self.info_detailed = !self.info_detailed;
        self.info_scroll = 0;
        // Snap active field back into the visible set if switching to simple view.
        let visible = self.info_visible_fields();
        if !visible.contains(&self.info_edit_field) {
            self.info_edit_field = visible[0];
        }
    }

    fn enter_info_edit_mode(&mut self) {
        self.ensure_info_cache();
        if self.info_field_track_path.is_none() {
            return;
        }
        self.info_editing = true;
        self.info_edit_field = self.info_visible_fields()[0];
    }

    fn move_edit_field(&mut self, delta: i32) {
        let visible = self.info_visible_fields();
        let cur = visible
            .iter()
            .position(|f| *f == self.info_edit_field)
            .unwrap_or(0) as i32;
        let n = visible.len() as i32;
        let next = ((cur + delta) % n + n) % n;
        self.info_edit_field = visible[next as usize];
    }

    fn handle_info_edit_key(&mut self, key: KeyCode) {
        let field = self.info_edit_field;
        match key {
            KeyCode::Esc => {
                self.info_editing = false;
                // Discard any unsaved edits by reloading from disk/track.
                self.info_field_track_path = None;
                self.ensure_info_cache();
            }
            KeyCode::Enter => {
                self.save_focused_track_metadata();
                self.info_editing = false;
            }
            KeyCode::Tab | KeyCode::Down => self.move_edit_field(1),
            KeyCode::BackTab | KeyCode::Up => self.move_edit_field(-1),
            KeyCode::Backspace => {
                if let Some(buf) = self.info_field_values.get_mut(&field) {
                    buf.pop();
                } else {
                    self.info_field_values.insert(field, String::new());
                }
            }
            KeyCode::Char(c) => {
                if field.is_numeric() && !c.is_ascii_digit() {
                    return;
                }
                self.info_field_values
                    .entry(field)
                    .or_default()
                    .push(c);
            }
            _ => {}
        }
    }

    /// Write the current cached field values to the file tag and (for core fields)
    /// the SQLite DB + in-memory Track structs.
    fn save_focused_track_metadata(&mut self) {
        let Some(path) = self.info_field_track_path.clone() else {
            return;
        };

        // Refuse to rewrite the tag of the file currently being streamed —
        // rewriting mid-playback corrupts decoding.
        let is_playing = self.player.current_track.as_ref().map(|t| t.path.as_str())
            == Some(path.as_str())
            && self.player.state != PlayerState::Stopped;
        if is_playing {
            self.set_status(
                "Cannot edit tags of currently-playing track — stop playback first.".to_string(),
            );
            return;
        }

        let values = self.info_field_values.clone();
        let wrote_tag = crate::lyrics::write_metadata_to_tag(&path, &values);

        // Update in-memory Track (core fields only) so the UI reflects the change.
        use crate::types::EditField as F;
        let title = values.get(&F::Title).cloned().unwrap_or_default();
        let artist = values.get(&F::Artist).cloned().unwrap_or_default();
        let album = values.get(&F::Album).cloned().unwrap_or_default();
        let albumartist = values.get(&F::AlbumArtist).cloned().unwrap_or_default();
        let year: i32 = values
            .get(&F::Year)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let genre = values.get(&F::Genre).cloned().unwrap_or_default();

        let apply = |t: &mut Track| {
            t.title = title.clone();
            t.artist = artist.clone();
            t.album = album.clone();
            t.albumartist = albumartist.clone();
            t.year = year;
            t.genre = genre.clone();
        };
        for t in self.all_tracks.iter_mut().filter(|t| t.path == path) {
            apply(Arc::make_mut(t));
        }
        for t in self.filtered_tracks.iter_mut().filter(|t| t.path == path) {
            apply(Arc::make_mut(t));
        }
        for t in self.player.queue.iter_mut().filter(|t| t.path == path) {
            apply(Arc::make_mut(t));
        }
        if let Some(ct) = self.player.current_track.as_mut() {
            if ct.path == path {
                apply(Arc::make_mut(ct));
            }
        }

        let wrote_db = match Db::open(&self.db_path, &self.music_dir) {
            Ok(db) => db
                .update_track_metadata(&path, &title, &artist, &album, &albumartist, year, &genre)
                .is_ok(),
            Err(_) => false,
        };

        if wrote_tag && wrote_db {
            self.refresh_sidebar_after_metadata_edit();
            self.invalidate_wrap_cache();
            let display = if title.is_empty() { path.as_str() } else { title.as_str() };
            self.set_status(format!("Saved metadata for {}", display));
        } else if !wrote_tag {
            self.set_status("Failed to write tag to file.".to_string());
        } else {
            self.set_status("Wrote tag but failed to update database.".to_string());
        }
    }

    fn refresh_sidebar_after_metadata_edit(&mut self) {
        if let Ok(db) = Db::open(&self.db_path, &self.music_dir) {
            if let Ok(a) = db.distinct_artists() {
                self.sidebar.artists.items = a;
                self.sidebar.artists.filter_indices = None;
            }
            if let Ok(a) = db.distinct_albums() {
                self.sidebar.albums.items = a;
                self.sidebar.albums.filter_indices = None;
            }
            if let Ok(g) = db.distinct_genres() {
                self.sidebar.genres.items = g;
                self.sidebar.genres.filter_indices = None;
            }
        }
        self.rebuild_sidebar();
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

        self.track_heading = item.title();

        self.search_query.clear();
        self.clear_selection();
        self.track_context = TrackContext::Library;

        self.filtered_tracks = match &item {
            SidebarItem::AllTracks => self.all_tracks.clone(),
            SidebarItem::Favorites => self
                .all_tracks
                .iter()
                .filter(|t| t.favorite)
                .cloned()
                .collect(),
            SidebarItem::RecentlyAdded => {
                let mut sorted = self.all_tracks.clone();
                sorted.sort_by(|a, b| b.added_at.cmp(&a.added_at));
                sorted.truncate(50);
                sorted
            }
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

        match item {
            SidebarItem::RecentlyAdded => {
                self.sort_field = SortField::DateAdded;
                self.sort_order = SortOrder::Desc;
            }
            _ => {
                self.sort_field = SortField::Artist;
                self.sort_order = SortOrder::Asc;
            }
        }

        self.track_list_index = 0;
        self.track_list_offset = 0;
        if self.track_context == TrackContext::Library && !matches!(item, SidebarItem::RecentlyAdded) {
            self.apply_sort();
        }
        self.invalidate_wrap_cache();
    }

    fn load_playlist_tracks(&self, name: &str) -> Vec<Arc<Track>> {
        let pl = match self.playlists.iter().find(|p| p.name == name) {
            Some(p) => p,
            _ => return Vec::new(),
        };
        let by_path: HashMap<&str, &Arc<Track>> = self
            .all_tracks
            .iter()
            .map(|t| (t.path.as_str(), t))
            .collect();
        pl.entries
            .iter()
            .filter_map(|ep| by_path.get(ep.as_str()).map(|t| Arc::clone(t)))
            .collect()
    }

    fn create_playlist(&mut self, name: &str) {
        let file_name = format!("{}.m3u8", name.replace('/', "_"));
        let path_str = Path::new(&self.music_dir)
            .join(&file_name)
            .to_string_lossy()
            .to_string();
        match Playlist::create(&path_str, name) {
            Ok(pl) => {
                self.playlists.push(pl);
                self.playlists
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                let playlist_names: Vec<String> =
                    self.playlists.iter().map(|p| p.name.clone()).collect();
                self.sidebar.playlists = SidebarSectionState {
                    items: playlist_names,
                    filter_indices: None,
                };
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
        self.lyrics.scroll = 0;
        self.lyrics.fetch_status = LyricsFetchStatus::Idle;
        match &self.player.current_track {
            Some(track) => {
                self.lyrics.track_path = Some(track.path.clone());
                let cached = self.pending_lyrics_writes.get(&track.path).cloned();
                let text = cached.or_else(|| {
                    crate::lyrics::extract_lyrics(&track.path)
                        .filter(|t| !t.trim().is_empty())
                });
                self.lyrics.content = text.map(|t| crate::lyrics::parse_lyrics(&t));
            }
            None => {
                self.lyrics.track_path = None;
                self.lyrics.content = None;
            }
        }
    }

    /// Force-flush all pending lyric tag writes. Call on app shutdown so
    /// fetched-but-unpersisted lyrics aren't lost. Stops the player first to
    /// release any file handle on the currently-playing track.
    pub fn flush_all_pending_lyrics(&mut self) {
        if self.pending_lyrics_writes.is_empty() {
            return;
        }
        self.player.stop();
        for (path, text) in self.pending_lyrics_writes.drain() {
            crate::lyrics::write_lyrics_to_tag(&path, &text);
        }
    }

    /// Kick off a background fetch of lyrics from lrclib for the current track.
    pub fn fetch_lyrics_from_lrclib(&mut self) {
        if matches!(self.lyrics.fetch_status, LyricsFetchStatus::Fetching) {
            return;
        }
        let track = match &self.player.current_track {
            Some(t) => t.clone(),
            None => return,
        };
        self.lyrics.fetch_status = LyricsFetchStatus::Fetching;
        let (tx, rx) = mpsc::channel();
        self.lyrics_fetch_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(LyricsFetchResult::Error(e.to_string()));
                    return;
                }
            };
            let client = match reqwest::Client::builder()
                .user_agent("lyre/0.1 (https://github.com/rkliman/lyre)")
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(LyricsFetchResult::Error(e.to_string()));
                    return;
                }
            };
            let artist = deunicode::deunicode(track.display_artist());
            let title = deunicode::deunicode(track.display_title());
            let album = if track.album.is_empty() {
                None
            } else {
                Some(track.album.as_str())
            };
            let dur = if track.duration > 0 {
                Some(track.duration as u32)
            } else {
                None
            };

            let result = rt.block_on(crate::indexer::lrclib::resolve_lyrics(
                &client, &artist, &title, album, dur,
            ));

            match result {
                Ok(Some(resolution)) => {
                    let _ = tx.send(LyricsFetchResult::Found {
                        path: track.path.clone(),
                        text: resolution.tag_text(),
                    });
                }
                Ok(None) => {
                    let _ = tx.send(LyricsFetchResult::NotFound);
                }
                Err(e) => {
                    let _ = tx.send(LyricsFetchResult::Error(e.to_string()));
                }
            }
        });
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
            self.filtered_tracks = self.search_base_tracks.clone();
            return;
        }
        let q = self.search_query.clone();
        let matcher = &self.matcher;
        let mut scored: Vec<(i64, Arc<Track>)> = self
            .search_base_tracks
            .iter()
            .filter_map(|t| {
                let h = format!("{} {} {} {}", t.title, t.artist, t.albumartist, t.album);
                matcher.fuzzy_match(&h, &q).map(|s| (s, Arc::clone(t)))
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
            SortField::DateAdded => self.filtered_tracks.sort_by(|a, b| {
                let c = a.added_at.cmp(&b.added_at);
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
                if self.lyrics.visible {
                    self.load_lyrics();
                    Panel::Lyrics
                } else {
                    Panel::Sidebar
                }
            }
            Panel::Lyrics => {
                self.lyrics.manual_scroll = false;
                Panel::Sidebar
            }
        };
    }

    fn cycle_panel_backward(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Sidebar => {
                if self.lyrics.visible {
                    self.load_lyrics();
                    Panel::Lyrics
                } else {
                    Panel::Queue
                }
            }
            Panel::TrackList => Panel::Sidebar,
            Panel::Queue => Panel::TrackList,
            Panel::Lyrics => {
                self.lyrics.manual_scroll = false;
                Panel::Queue
            }
        };
    }

    fn toggle_lyrics_panel(&mut self) {
        if self.lyrics.visible {
            self.lyrics.visible = false;
            self.lyrics.manual_scroll = false;
            if self.active_panel == Panel::Lyrics {
                self.active_panel = Panel::Queue;
            }
        } else {
            // Show lyrics panel
            self.lyrics.visible = true;
            self.load_lyrics();
            self.active_panel = Panel::Lyrics;
        }
    }

    fn toggle_art_window(&mut self) {
        self.art_window_visible = !self.art_window_visible;
    }

    fn enter_search(&mut self) {
        self.search_mode = true;
        self.search_base_tracks = self.filtered_tracks.clone();
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
        self.persist_queue();
    }

    fn play_queue_selected(&mut self) {
        if self.queue_index < self.player.queue.len() {
            self.player.queue_index = self.queue_index;
            let track = self.player.queue[self.queue_index].clone();
            match self.player.play_track(track) {
                Ok(_) => self.refresh_album_art(),
                Err(e) => self.set_status(format!("Error playing track: {}", e)),
            }
            self.persist_queue();
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
        self.persist_queue();
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
        self.persist_queue();
    }

    fn toggle_favorite_selected(&mut self) {
        if self.filtered_tracks.is_empty() {
            return;
        }
        let indices: Vec<usize> = if self.selected_tracks.is_empty() {
            vec![self.track_list_index.min(self.filtered_tracks.len() - 1)]
        } else {
            self.get_selected_indices_ascending()
        };

        let paths: Vec<String> = indices
            .iter()
            .filter_map(|&i| self.filtered_tracks.get(i).map(|t| t.path.clone()))
            .collect();
        if paths.is_empty() {
            return;
        }

        // Determine target state: if any selected track is not favorited, favorite all; else unfavorite all.
        let any_unfavorited = paths.iter().any(|p| {
            self.all_tracks
                .iter()
                .find(|t| &t.path == p)
                .map(|t| !t.favorite)
                .unwrap_or(false)
        });
        let target = any_unfavorited;

        let db = Db::open(&self.db_path, &self.music_dir).ok();
        let mut updated = 0usize;
        for p in &paths {
            if let Some(ref d) = db {
                if d.set_favorite(p, target).is_ok() {
                    updated += 1;
                }
            }
            for t in self.all_tracks.iter_mut().filter(|t| &t.path == p) {
                Arc::make_mut(t).favorite = target;
            }
            for t in self.filtered_tracks.iter_mut().filter(|t| &t.path == p) {
                Arc::make_mut(t).favorite = target;
            }
        }

        // If viewing favorites, drop unfavorited rows from the current list.
        if matches!(self.sidebar_items.get(self.sidebar_index), Some(SidebarItem::Favorites))
            && !target
        {
            self.filtered_tracks.retain(|t| t.favorite);
            if self.track_list_index >= self.filtered_tracks.len() && self.track_list_index > 0 {
                self.track_list_index = self.filtered_tracks.len().saturating_sub(1);
            }
        }

        self.invalidate_wrap_cache();
        self.set_status(format!(
            "{} {} track{}",
            if target { "Favorited" } else { "Unfavorited" },
            updated,
            if updated == 1 { "" } else { "s" }
        ));
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    pub fn persist_queue(&self) {
        if let Some(ref sdb) = self.state_db {
            let _ = sdb.save_queue(&self.player.queue, self.player.queue_index);
        }
    }

    fn persist_player_setting(&self, key: &str, value: &str) {
        if let Some(ref sdb) = self.state_db {
            let _ = sdb.save_state(key, value);
        }
    }

    /// Call this after any change to filtered_tracks so the wrap cache is rebuilt.
    pub fn invalidate_wrap_cache(&mut self) {
        self.wrapped_width = 0; // force rebuild on next render
    }

    /// Rebuild the wrap cache for the given total panel width.
    /// Called from ui.rs when the width changes or cache is invalid.
    pub fn rebuild_wrapped_tracks(&mut self, panel_w: usize) {
        let title_w1 = (panel_w * 32 / 100).saturating_sub(5);
        let artist_w = panel_w * 28 / 100;
        let album_w = panel_w * 28 / 100;

        self.wrapped_tracks = self
            .filtered_tracks
            .iter()
            .map(|track| {
                let fav = if track.favorite { FAVORITE_ICON } else { " " };
                let title = track.display_title();
                let artist = track.display_artist();
                let album = track.display_album();
                let dur = track.duration_str();

                // ── Collapsed: truncate each field with … if it overflows ─────────
                let t0 = truncate_field(title, title_w1);
                let a0 = truncate_field(artist, artist_w);
                let b0 = truncate_field(album, album_w);
                let collapsed = format!("  {}  {}  {}  {}  {:>4}", fav, t0, a0, b0, dur);

                // ── Expanded: word-wrap each field across as many rows as needed ──
                let tc = wrap_field(title, title_w1);
                let ac = wrap_field(artist, artist_w);
                let bc = wrap_field(album, album_w);

                let n = [tc.len(), ac.len(), bc.len()]
                    .into_iter()
                    .max()
                    .unwrap_or(1)
                    .max(1);

                let empty_t = " ".repeat(title_w1);
                let empty_a = " ".repeat(artist_w);
                let empty_b = " ".repeat(album_w);

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
                        let h = if row == 0 { fav } else { " " };
                        if row == 0 {
                            format!("  {}  {}  {}  {}  {:>4}", h, t, a, b, dur)
                        } else {
                            format!("  {}  {}  {}  {}", h, t, a, b)
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
        self.art.clear();
    }

    fn remove_from_queue(&mut self, index: usize) {
        let was_playing_current = index == self.player.queue_index;
        if was_playing_current {
            self.player.stop();
        }
        self.player.remove_from_queue(index);
        if self.queue_index >= self.player.queue.len() && self.queue_index > 0 {
            self.queue_index -= 1;
        }
        if was_playing_current {
            if index < self.player.queue.len() {
                let track = self.player.queue[self.player.queue_index].clone();
                match self.player.play_track(track) {
                    Ok(_) => self.refresh_album_art(),
                    Err(e) => self.set_status(format!("Error playing track: {}", e)),
                }
            } else {
                self.clear_album_art();
            }
        }
        self.persist_queue();
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
        if self.art.path.as_deref() == Some(&path) {
            return;
        }
        self.art.path = Some(path.clone());
        self.art.mpris_url = None; // will be lazily re-extracted by main.rs
        // Clear cached art window rendering since track changed
        self.art.window_cache.clear();

        // Load album art: extract once and cache the bytes
        if let Some(bytes) = crate::art::extract_cover_bytes(&path) {
            self.art.image = crate::art::load_cover_image(&bytes);
            // Small version for player bar
            self.art.player_bar = crate::art::render_block_art(&bytes, 8, 3);
            // Cache the bytes to avoid re-extraction
            self.art.bytes = Some(bytes);
        } else {
            self.clear_album_art();
        }
    }

    /// Get the index of the currently active lyric line based on playback position.
    /// Returns None if lyrics are not timed or no lyrics are loaded.
    pub fn current_lyric_index(&self) -> Option<usize> {
        use crate::types::Lyrics;

        if let Some(Lyrics::Timed(lines)) = &self.lyrics.content {
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

    fn handle_global_search_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                self.global_search_query.clear();
                self.global_search_selected = 0;
                self.global_search_results.clear();
            }
            KeyCode::Enter => {
                self.navigate_global_search_selected();
                self.overlay = Overlay::None;
                self.global_search_query.clear();
                self.global_search_selected = 0;
                self.global_search_results.clear();
            }
            KeyCode::Backspace => {
                self.global_search_query.pop();
                self.recompute_global_search();
            }
            KeyCode::Up => {
                let total = self.global_search_total();
                if total > 0 {
                    self.global_search_selected = if self.global_search_selected == 0 {
                        total - 1
                    } else {
                        self.global_search_selected - 1
                    };
                }
            }
            KeyCode::Down => {
                let total = self.global_search_total();
                if total > 0 {
                    self.global_search_selected = if self.global_search_selected + 1 >= total {
                        0
                    } else {
                        self.global_search_selected + 1
                    };
                }
            }
            KeyCode::PageUp => {
                self.global_search_selected = self.global_search_selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let total = self.global_search_total();
                if total > 0 {
                    self.global_search_selected =
                        (self.global_search_selected + 10).min(total - 1);
                }
            }
            KeyCode::Char(c) => {
                self.global_search_query.push(c);
                self.recompute_global_search();
            }
            _ => {}
        }
        false
    }

    pub fn global_search_total(&self) -> usize {
        self.global_search_results.len()
    }

    fn recompute_global_search(&mut self) {
        if self.global_search_query.is_empty() {
            self.global_search_results.clear();
            self.global_search_selected = 0;
            return;
        }

        let q = self.global_search_query.clone();
        let matcher = &self.matcher;

        let mut scored: Vec<(i64, GlobalSearchResult)> = Vec::new();

        for t in &self.all_tracks {
            let h = format!("{} {} {} {}", t.title, t.artist, t.albumartist, t.album);
            if let Some(s) = matcher.fuzzy_match(&h, &q) {
                scored.push((s, GlobalSearchResult::Track(Arc::clone(t))));
            }
        }
        for a in &self.sidebar.albums.items {
            if let Some(s) = matcher.fuzzy_match(a, &q) {
                scored.push((s, GlobalSearchResult::Album(a.clone())));
            }
        }
        for a in &self.sidebar.artists.items {
            if let Some(s) = matcher.fuzzy_match(a, &q) {
                scored.push((s, GlobalSearchResult::Artist(a.clone())));
            }
        }
        for p in &self.sidebar.playlists.items {
            if let Some(s) = matcher.fuzzy_match(p, &q) {
                scored.push((s, GlobalSearchResult::Playlist(p.clone())));
            }
        }
        for g in &self.sidebar.genres.items {
            if let Some(s) = matcher.fuzzy_match(g, &q) {
                scored.push((s, GlobalSearchResult::Genre(g.clone())));
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.global_search_results = scored.into_iter().take(50).map(|(_, r)| r).collect();
        self.global_search_selected = 0;
    }

    fn navigate_global_search_selected(&mut self) {
        let Some(result) = self.global_search_results.get(self.global_search_selected).cloned()
        else {
            return;
        };
        match result {
            GlobalSearchResult::Track(track) => {
                if track.album.is_empty() {
                    self.navigate_to_sidebar_item(SidebarItem::AllTracks);
                } else {
                    self.navigate_to_sidebar_item(SidebarItem::Album(track.album.clone()));
                }
                if let Some(idx) =
                    self.filtered_tracks.iter().position(|ft| ft.path == track.path)
                {
                    self.track_list_index = idx;
                }
                self.invalidate_wrap_cache();
            }
            GlobalSearchResult::Album(name) => {
                self.navigate_to_sidebar_item(SidebarItem::Album(name));
            }
            GlobalSearchResult::Artist(name) => {
                self.navigate_to_sidebar_item(SidebarItem::Artist(name));
            }
            GlobalSearchResult::Playlist(name) => {
                self.navigate_to_sidebar_item(SidebarItem::Playlist(name));
            }
            GlobalSearchResult::Genre(name) => {
                self.navigate_to_sidebar_item(SidebarItem::Genre(name));
            }
        }
    }

    fn navigate_to_sidebar_item(&mut self, item: SidebarItem) {
        self.sidebar_search_mode = false;
        self.sidebar_search_query.clear();
        self.sidebar_search_section = None;
        for sec in SidebarSection::ALL {
            self.sidebar.get_mut(sec).filter_indices = None;
        }

        match &item {
            SidebarItem::Artist(_) => {
                self.sidebar_expanded.insert("Artists".to_string(), true);
            }
            SidebarItem::Album(_) => {
                self.sidebar_expanded.insert("Albums".to_string(), true);
            }
            SidebarItem::Genre(_) => {
                self.sidebar_expanded.insert("Genres".to_string(), true);
            }
            SidebarItem::Playlist(_) => {
                self.sidebar_expanded.insert("Playlists".to_string(), true);
            }
            _ => {}
        }

        self.rebuild_sidebar();

        if let Some(idx) = self.sidebar_items.iter().position(|i| *i == item) {
            self.sidebar_index = idx;
            self.on_sidebar_select();
        }

        self.active_panel = Panel::TrackList;
        self.track_list_index = 0;
        self.track_list_offset = 0;
    }
}
