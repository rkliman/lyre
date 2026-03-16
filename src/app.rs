use anyhow::Result;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Deserialize;
use shellexpand;
use std::collections::HashMap;
use std::path::Path;

use crate::db::Db;
use crate::player::Player;
use crate::playlist::{scan_playlists, Playlist};
use crate::types::{Overlay, Panel, PlayerState, SidebarItem, SortField, SortOrder, Track, TrackContext};

#[derive(Debug, Deserialize)]
struct FilesConfig {
    database_name: String,
    music_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Settings {
    files: FilesConfig,
}

fn load_config() -> (String, String) {
    let config_path =
        shellexpand::tilde("~/.config/apollo-music/config.toml").to_string();
    if let Ok(cfg) = config::Config::builder()
        .add_source(config::File::with_name(&config_path))
        .build()
    {
        if let Ok(settings) = cfg.try_deserialize::<Settings>() {
            let db = settings.files.database_name;
            let music = settings.files.music_directory.unwrap_or_else(|| "~/Music".to_string());
            return (db, music);
        }
    }
    ("~/.local/share/apollo-music/music.db".to_string(), "~/Music".to_string())
}

pub struct App {
    pub all_tracks: Vec<Track>,
    pub filtered_tracks: Vec<Track>,
    pub track_context: TrackContext,
    pub sidebar_artists: Vec<String>,
    pub sidebar_albums: Vec<String>,
    pub sidebar_genres: Vec<String>,
    pub sidebar_items: Vec<SidebarItem>,
    pub sidebar_index: usize,
    pub sidebar_expanded: HashMap<String, bool>,
    pub playlists: Vec<Playlist>,
    pub music_dir: String,
    pub track_list_index: usize,
    pub track_list_offset: usize,
    pub queue_index: usize,
    pub queue_offset: usize,
    pub active_panel: Panel,
    pub sort_field: SortField,
    pub sort_order: SortOrder,
    pub search_mode: bool,
    pub search_query: String,
    pub player: Player,
    pub status_message: Option<String>,
    pub album_art: Option<crate::art::BlockArt>,
    pub album_art_path: Option<String>,
    pub overlay: Overlay,
    pub show_help: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let (db_path, music_dir_raw) = load_config();
        let music_dir = shellexpand::tilde(&music_dir_raw).to_string();
        let db = Db::open(&db_path)?;
        let all_tracks = db.all_tracks().unwrap_or_default();
        let filtered_tracks = all_tracks.clone();
        let artists = db.distinct_artists().unwrap_or_default();
        let albums  = db.distinct_albums().unwrap_or_default();
        let genres  = db.distinct_genres().unwrap_or_default();
        let playlists = scan_playlists(&music_dir);

        let mut sidebar_expanded: HashMap<String, bool> = HashMap::new();
        sidebar_expanded.insert("Artists".to_string(),   true);
        sidebar_expanded.insert("Albums".to_string(),    true);
        sidebar_expanded.insert("Genres".to_string(),    true);
        sidebar_expanded.insert("Playlists".to_string(), true);

        let player = Player::new().expect("Failed to initialize audio.");

        let mut app = Self {
            all_tracks, filtered_tracks,
            track_context: TrackContext::Library,
            sidebar_artists: artists, sidebar_albums: albums, sidebar_genres: genres,
            sidebar_items: Vec::new(), sidebar_index: 0, sidebar_expanded,
            playlists, music_dir,
            track_list_index: 0, track_list_offset: 0,
            queue_index: 0, queue_offset: 0,
            active_panel: Panel::Sidebar,
            sort_field: SortField::Artist, sort_order: SortOrder::Asc,
            search_mode: false, search_query: String::new(),
            player, status_message: None,
            album_art: None, album_art_path: None,
            overlay: Overlay::None,
            show_help: false,
        };
        app.rebuild_sidebar();
        app.apply_sort();
        Ok(app)
    }

    pub fn rebuild_sidebar(&mut self) {
        let mut items = vec![SidebarItem::AllTracks];

        let artists_open = *self.sidebar_expanded.get("Artists").unwrap_or(&true);
        items.push(SidebarItem::Artists);
        if artists_open { for a in &self.sidebar_artists { items.push(SidebarItem::Artist(a.clone())); } }

        let albums_open = *self.sidebar_expanded.get("Albums").unwrap_or(&true);
        items.push(SidebarItem::Albums);
        if albums_open { for a in &self.sidebar_albums { items.push(SidebarItem::Album(a.clone())); } }

        let genres_open = *self.sidebar_expanded.get("Genres").unwrap_or(&true);
        items.push(SidebarItem::Genres);
        if genres_open { for g in &self.sidebar_genres { items.push(SidebarItem::Genre(g.clone())); } }

        let playlists_open = *self.sidebar_expanded.get("Playlists").unwrap_or(&true);
        items.push(SidebarItem::Playlists);
        if playlists_open { for pl in &self.playlists { items.push(SidebarItem::Playlist(pl.name.clone())); } }

        if self.sidebar_index >= items.len() { self.sidebar_index = items.len().saturating_sub(1); }
        self.sidebar_items = items;
    }

    fn toggle_sidebar_section(&mut self, item: &SidebarItem) {
        let key = match item {
            SidebarItem::Artists   => "Artists",
            SidebarItem::Albums    => "Albums",
            SidebarItem::Genres    => "Genres",
            SidebarItem::Playlists => "Playlists",
            _ => return,
        };
        let current = *self.sidebar_expanded.get(key).unwrap_or(&true);
        self.sidebar_expanded.insert(key.to_string(), !current);
        self.rebuild_sidebar();
    }

    pub fn tick(&mut self) {
        if self.player.is_finished() {
            let _ = self.player.next();
            self.refresh_album_art(10, 3);
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> bool {
        if self.overlay != Overlay::None { return self.handle_overlay_key(key); }
        if self.show_help { self.show_help = false; return false; }
        if self.search_mode { return self.handle_search_key(key); }

        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Tab      => self.cycle_panel_forward(),
            KeyCode::BackTab  => self.cycle_panel_backward(),
            KeyCode::Char('1') => self.active_panel = Panel::Sidebar,
            KeyCode::Char('2') => self.active_panel = Panel::TrackList,
            KeyCode::Char('3') => self.active_panel = Panel::Queue,
            KeyCode::Char(' ') => {
                if self.player.state == PlayerState::Stopped { self.play_selected(); }
                else { self.player.toggle_pause(); }
            }
            KeyCode::Char('n') => { let _ = self.player.next();  self.refresh_album_art(10, 3); }
            KeyCode::Char('p') => { let _ = self.player.prev();  self.refresh_album_art(10, 3); }
            KeyCode::Char('s') => self.player.stop(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.player.volume_up(),
            KeyCode::Char('-') => self.player.volume_down(),
            KeyCode::Char('S') => self.cycle_sort(),
            KeyCode::Char('R') => self.toggle_sort_order(),
            KeyCode::Char('/') => self.enter_search(),
            // P is global — add selected track to a playlist from any panel
            KeyCode::Char('P') => self.open_add_to_playlist_overlay(),
            _ => self.handle_panel_key(key),
        }
        false
    }

    fn handle_panel_key(&mut self, key: KeyCode) {
        match self.active_panel {
            Panel::Sidebar   => self.handle_sidebar_key(key),
            Panel::TrackList => self.handle_tracklist_key(key),
            Panel::Queue     => self.handle_queue_key(key),
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.sidebar_index > 0 { self.sidebar_index -= 1; self.on_sidebar_select(); }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.sidebar_index + 1 < self.sidebar_items.len() { self.sidebar_index += 1; self.on_sidebar_select(); }
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                self.sidebar_index = self.sidebar_index.saturating_sub(10); self.on_sidebar_select();
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                self.sidebar_index = (self.sidebar_index + 10).min(self.sidebar_items.len().saturating_sub(1));
                self.on_sidebar_select();
            }
            KeyCode::Home | KeyCode::Char('g') => { self.sidebar_index = 0; self.on_sidebar_select(); }
            KeyCode::End  | KeyCode::Char('G') => { self.sidebar_index = self.sidebar_items.len().saturating_sub(1); self.on_sidebar_select(); }
            KeyCode::Enter => { self.active_panel = Panel::TrackList; }
            KeyCode::Right | KeyCode::Char('l') => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if item.is_header() {
                    let k = match &item {
                        SidebarItem::Artists => "Artists", SidebarItem::Albums => "Albums",
                        SidebarItem::Genres  => "Genres",  SidebarItem::Playlists => "Playlists",
                        _ => "",
                    };
                    if !k.is_empty() {
                        let open = *self.sidebar_expanded.get(k).unwrap_or(&true);
                        if !open { self.sidebar_expanded.insert(k.to_string(), true); self.rebuild_sidebar(); }
                        else { self.active_panel = Panel::TrackList; }
                    }
                } else { self.active_panel = Panel::TrackList; }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let item = self.sidebar_items[self.sidebar_index].clone();
                if item.is_header() { self.toggle_sidebar_section(&item); }
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
        let in_playlist = matches!(&self.track_context, TrackContext::Playlist(_));
        match key {
            KeyCode::Up   | KeyCode::Char('k') => { if self.track_list_index > 0 { self.track_list_index -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.track_list_index + 1 < self.filtered_tracks.len() { self.track_list_index += 1; } }
            KeyCode::PageUp   | KeyCode::Char('u') => { self.track_list_index = self.track_list_index.saturating_sub(10); }
            KeyCode::PageDown | KeyCode::Char('d') => { self.track_list_index = (self.track_list_index + 10).min(self.filtered_tracks.len().saturating_sub(1)); }
            KeyCode::Home | KeyCode::Char('g') => { self.track_list_index = 0; }
            KeyCode::End  | KeyCode::Char('G') => { self.track_list_index = self.filtered_tracks.len().saturating_sub(1); }
            KeyCode::Enter => self.play_selected(),
            KeyCode::Char('a') => self.add_selected_to_queue(),
            KeyCode::Char('A') => self.add_all_to_queue(),
            // / activates the search bar from within the track list too
            KeyCode::Char('/') => self.enter_search(),
            KeyCode::Char('x') | KeyCode::Delete if in_playlist => { self.playlist_remove_track(self.track_list_index); }
            KeyCode::Char('K') if in_playlist => { self.playlist_move_track_up(self.track_list_index); }
            KeyCode::Char('J') if in_playlist => { self.playlist_move_track_down(self.track_list_index); }
            KeyCode::Left | KeyCode::Char('h') => { self.active_panel = Panel::Sidebar; }
            _ => {}
        }
    }

    fn handle_queue_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up   | KeyCode::Char('k') => { if self.queue_index > 0 { self.queue_index -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.queue_index + 1 < self.player.queue.len() { self.queue_index += 1; } }
            KeyCode::Enter => {
                if self.queue_index < self.player.queue.len() {
                    self.player.queue_index = self.queue_index;
                    let track = self.player.queue[self.queue_index].clone();
                    let _ = self.player.play_track(track);
                    self.refresh_album_art(10, 3);
                }
            }
            KeyCode::Delete | KeyCode::Char('x') => {
                self.player.remove_from_queue(self.queue_index);
                if self.queue_index >= self.player.queue.len() && self.queue_index > 0 { self.queue_index -= 1; }
            }
            KeyCode::Char('c') => {
                self.player.stop();
                self.player.clear_queue();
                self.queue_index = 0;
                self.album_art = None;
                self.album_art_path = None;
            }
            KeyCode::Left | KeyCode::Char('h') => { self.active_panel = Panel::TrackList; }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.search_mode = false;
                // Only wipe the query (and reset to full list) if the bar is empty
                if self.search_query.is_empty() {
                    self.filtered_tracks = self.all_tracks.clone();
                    self.apply_sort();
                }
                // If there's a query, keep the filtered results visible but go inactive
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.active_panel = Panel::TrackList;
                self.track_list_index = 0;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.apply_fuzzy_search();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.apply_fuzzy_search();
            }
            _ => {}
        }
        false
    }

    fn handle_overlay_key(&mut self, key: KeyCode) -> bool {
        match &self.overlay.clone() {
            Overlay::NewPlaylist(name) => {
                let mut name = name.clone();
                match key {
                    KeyCode::Esc   => { self.overlay = Overlay::None; }
                    KeyCode::Enter => {
                        if !name.trim().is_empty() { self.create_playlist(name.trim()); }
                        self.overlay = Overlay::None;
                    }
                    KeyCode::Backspace => { name.pop(); self.overlay = Overlay::NewPlaylist(name); }
                    KeyCode::Char(c)   => { name.push(c); self.overlay = Overlay::NewPlaylist(name); }
                    _ => {}
                }
            }
            Overlay::AddToPlaylist { track_path, selected } => {
                let track_path = track_path.clone();
                let mut selected = *selected;
                match key {
                    KeyCode::Esc => { self.overlay = Overlay::None; }
                    KeyCode::Up   | KeyCode::Char('k') => { if selected > 0 { selected -= 1; } self.overlay = Overlay::AddToPlaylist { track_path, selected }; }
                    KeyCode::Down | KeyCode::Char('j') => { if selected + 1 < self.playlists.len() { selected += 1; } self.overlay = Overlay::AddToPlaylist { track_path, selected }; }
                    KeyCode::Enter => {
                        if selected < self.playlists.len() { self.add_track_to_playlist(selected, &track_path.clone()); }
                        self.overlay = Overlay::None;
                    }
                    _ => {}
                }
            }
            Overlay::None => {}
        }
        false
    }

    fn on_sidebar_select(&mut self) {
        let item = self.sidebar_items[self.sidebar_index].clone();
        self.search_query.clear();
        self.track_context = TrackContext::Library;

        self.filtered_tracks = match &item {
            SidebarItem::AllTracks     => self.all_tracks.clone(),
            SidebarItem::Artist(a)     => self.all_tracks.iter().filter(|t| t.artist.eq_ignore_ascii_case(a) || t.albumartist.eq_ignore_ascii_case(a)).cloned().collect(),
            SidebarItem::Album(a)      => self.all_tracks.iter().filter(|t| t.album.eq_ignore_ascii_case(a)).cloned().collect(),
            SidebarItem::Genre(g)      => self.all_tracks.iter().filter(|t| t.genre.split(',').any(|s| s.trim().eq_ignore_ascii_case(g))).cloned().collect(),
            SidebarItem::Playlist(name) => { self.track_context = TrackContext::Playlist(name.clone()); self.load_playlist_tracks(name) }
            _ => self.all_tracks.clone(),
        };

        self.track_list_index = 0;
        self.track_list_offset = 0;
        if self.track_context == TrackContext::Library { self.apply_sort(); }
    }

    fn load_playlist_tracks(&self, name: &str) -> Vec<Track> {
        let pl = match self.playlists.iter().find(|p| p.name == name) { Some(p) => p, None => return Vec::new() };
        pl.entries.iter().filter_map(|ep| self.all_tracks.iter().find(|t| t.path == *ep).cloned()).collect()
    }

    fn create_playlist(&mut self, name: &str) {
        let file_name = format!("{}.m3u", name.replace('/', "_"));
        let path_str = Path::new(&self.music_dir).join(&file_name).to_string_lossy().to_string();
        match Playlist::create(&path_str, name) {
            Ok(pl) => {
                self.playlists.push(pl);
                self.playlists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.rebuild_sidebar();
                self.set_status(format!("Created playlist: {}", name));
            }
            Err(e) => self.set_status(format!("Failed to create playlist: {}", e)),
        }
    }

    fn open_add_to_playlist_overlay(&mut self) {
        if self.playlists.is_empty() {
            self.set_status("No playlists — press N on the Playlists header to create one".to_string());
            return;
        }
        if let Some(track) = self.filtered_tracks.get(self.track_list_index) {
            self.overlay = Overlay::AddToPlaylist { track_path: track.path.clone(), selected: 0 };
        }
    }

    fn add_track_to_playlist(&mut self, playlist_idx: usize, track_path: &str) {
        if playlist_idx >= self.playlists.len() { return; }
        let pl = &mut self.playlists[playlist_idx];
        pl.add_entry(track_path);
        let pl_name = pl.name.clone();
        if let Err(e) = pl.save() { self.set_status(format!("Failed to save: {}", e)); return; }
        self.set_status(format!("Added to {}", pl_name));
        if self.track_context == TrackContext::Playlist(pl_name.clone()) {
            self.filtered_tracks = self.load_playlist_tracks(&pl_name);
        }
    }

    fn playlist_remove_track(&mut self, index: usize) {
        let pl_name = match &self.track_context { TrackContext::Playlist(n) => n.clone(), _ => return };
        if let Some(pl) = self.playlists.iter_mut().find(|p| p.name == pl_name) { pl.remove_entry(index); let _ = pl.save(); }
        self.filtered_tracks = self.load_playlist_tracks(&pl_name);
        if self.track_list_index >= self.filtered_tracks.len() && self.track_list_index > 0 { self.track_list_index -= 1; }
    }

    fn playlist_move_track_up(&mut self, index: usize) {
        let pl_name = match &self.track_context { TrackContext::Playlist(n) => n.clone(), _ => return };
        if let Some(pl) = self.playlists.iter_mut().find(|p| p.name == pl_name) { pl.move_entry_up(index); let _ = pl.save(); }
        self.filtered_tracks = self.load_playlist_tracks(&pl_name);
        if index > 0 { self.track_list_index -= 1; }
    }

    fn playlist_move_track_down(&mut self, index: usize) {
        let pl_name = match &self.track_context { TrackContext::Playlist(n) => n.clone(), _ => return };
        if let Some(pl) = self.playlists.iter_mut().find(|p| p.name == pl_name) { pl.move_entry_down(index); let _ = pl.save(); }
        self.filtered_tracks = self.load_playlist_tracks(&pl_name);
        if index + 1 < self.filtered_tracks.len() { self.track_list_index += 1; }
    }

    fn apply_fuzzy_search(&mut self) {
        if self.search_query.is_empty() { self.filtered_tracks = self.all_tracks.clone(); return; }
        let matcher = SkimMatcherV2::default();
        let q = &self.search_query;
        let mut scored: Vec<(i64, Track)> = self.all_tracks.iter()
            .filter_map(|t| {
                let h = format!("{} {} {} {}", t.title, t.artist, t.albumartist, t.album);
                matcher.fuzzy_match(&h, q).map(|s| (s, t.clone()))
            }).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.filtered_tracks = scored.into_iter().map(|(_, t)| t).collect();
        self.track_list_index = 0;
    }

    pub fn apply_sort(&mut self) {
        if matches!(self.track_context, TrackContext::Playlist(_)) { return; }
        let asc = self.sort_order == SortOrder::Asc;
        match self.sort_field {
            SortField::Title    => self.filtered_tracks.sort_by(|a, b| { let c = a.title.to_lowercase().cmp(&b.title.to_lowercase()); if asc { c } else { c.reverse() } }),
            SortField::Artist   => self.filtered_tracks.sort_by(|a, b| { let c = a.display_artist().to_lowercase().cmp(&b.display_artist().to_lowercase()); if asc { c } else { c.reverse() } }),
            SortField::Album    => self.filtered_tracks.sort_by(|a, b| { let c = a.album.to_lowercase().cmp(&b.album.to_lowercase()); if asc { c } else { c.reverse() } }),
            SortField::Year     => self.filtered_tracks.sort_by(|a, b| { let c = a.year.cmp(&b.year); if asc { c } else { c.reverse() } }),
            SortField::Genre    => self.filtered_tracks.sort_by(|a, b| { let c = a.genre.to_lowercase().cmp(&b.genre.to_lowercase()); if asc { c } else { c.reverse() } }),
            SortField::Duration => self.filtered_tracks.sort_by(|a, b| { let c = a.duration.cmp(&b.duration); if asc { c } else { c.reverse() } }),
        }
    }

    fn cycle_sort(&mut self) {
        self.sort_field = self.sort_field.next(); self.apply_sort();
        self.set_status(format!("Sort: {} {}", self.sort_field.label(), if self.sort_order == SortOrder::Asc { "↑" } else { "↓" }));
    }

    fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order { SortOrder::Asc => SortOrder::Desc, SortOrder::Desc => SortOrder::Asc };
        self.apply_sort();
        self.set_status(format!("Sort: {} {}", self.sort_field.label(), if self.sort_order == SortOrder::Asc { "↑" } else { "↓" }));
    }

    fn cycle_panel_forward(&mut self) {
        self.active_panel = match self.active_panel { Panel::Sidebar => Panel::TrackList, Panel::TrackList => Panel::Queue, Panel::Queue => Panel::Sidebar };
    }

    fn cycle_panel_backward(&mut self) {
        self.active_panel = match self.active_panel { Panel::Sidebar => Panel::Queue, Panel::TrackList => Panel::Sidebar, Panel::Queue => Panel::TrackList };
    }

    fn enter_search(&mut self) {
        self.search_mode = true;
        self.track_context = TrackContext::Library;
        // Switch to track list panel so keypresses go to the search bar
        self.active_panel = Panel::TrackList;
    }

    fn play_selected(&mut self) {
        if self.filtered_tracks.is_empty() { return; }
        let idx = self.track_list_index.min(self.filtered_tracks.len() - 1);
        self.player.set_queue(self.filtered_tracks.clone(), idx);
        let track = self.player.queue[idx].clone();
        match self.player.play_track(track) {
            Ok(_) => {
                self.set_status(format!("Playing: {} — {}",
                    self.player.current_track.as_ref().map(|t| t.display_title()).unwrap_or(""),
                    self.player.current_track.as_ref().map(|t| t.display_artist()).unwrap_or("")));
                self.refresh_album_art(10, 3);
            }
            Err(e) => self.set_status(format!("Error: {}", e)),
        }
    }

    fn add_selected_to_queue(&mut self) {
        if let Some(track) = self.filtered_tracks.get(self.track_list_index) {
            let title = track.display_title().to_string();
            self.player.add_to_queue(track.clone());
            self.set_status(format!("Added to queue: {}", title));
        }
    }

    fn add_all_to_queue(&mut self) {
        let n = self.filtered_tracks.len();
        for t in &self.filtered_tracks { self.player.queue.push(t.clone()); }
        self.set_status(format!("Added {} tracks to queue", n));
    }

    pub fn set_status(&mut self, msg: String) { self.status_message = Some(msg); }

    pub fn refresh_album_art(&mut self, char_w: u16, char_h: u16) {
        let path = match self.player.current_track.as_ref().map(|t| t.path.clone()) {
            Some(p) => p, None => { self.album_art = None; self.album_art_path = None; return; }
        };
        if self.album_art_path.as_deref() == Some(&path) { return; }
        self.album_art_path = Some(path.clone());
        self.album_art = crate::art::extract_cover_bytes(&path)
            .and_then(|bytes| crate::art::render_block_art(&bytes, char_w, char_h));
    }
}