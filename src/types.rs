use std::sync::Arc;

/// An entry in the "Add to playlist" picker. Either the synthetic
/// "+ New Playlist" row or an existing playlist by its index in `App::playlists`.
#[derive(Debug, Clone, PartialEq)]
pub enum AddToPlaylistItem {
    NewPlaylist,
    Existing(usize),
}

#[derive(Debug, Clone)]
pub enum GlobalSearchResult {
    Track(Arc<Track>),
    Album(String),
    Artist(String),
    Playlist(String),
    Genre(String),
}

#[derive(Debug, Clone)]
pub struct Track {
    pub path: String,
    pub artist: String,
    pub album: String,
    pub albumartist: String,
    pub title: String,
    pub duration: i64,
    pub year: i32,
    pub genre: String,
    pub added_at: i64,
    pub favorite: bool,
    pub play_count: i64,
    pub last_played: i64,
}

impl Track {
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.path
        } else {
            &self.title
        }
    }

    pub fn display_artist(&self) -> &str {
        if !self.albumartist.is_empty() {
            &self.albumartist
        } else if !self.artist.is_empty() {
            &self.artist
        } else {
            "Unknown Artist"
        }
    }

    pub fn display_album(&self) -> &str {
        if self.album.is_empty() {
            "Unknown Album"
        } else {
            &self.album
        }
    }

    pub fn duration_str(&self) -> String {
        format_duration(self.duration)
    }
}

pub fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return "--:--".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

/// Renders a unix timestamp as a short relative time ("3d ago"), or "never"
/// for the zero/unset sentinel used by tracks that haven't been played yet.
pub fn format_relative(ts: i64) -> String {
    if ts <= 0 {
        return "never".to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(ts);
    let delta = (now - ts).max(0);
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else if delta < 86_400 * 30 {
        format!("{}d ago", delta / 86_400)
    } else if delta < 86_400 * 365 {
        format!("{}mo ago", delta / (86_400 * 30))
    } else {
        format!("{}y ago", delta / (86_400 * 365))
    }
}

/// An optional column that can be shown in the track list, toggled from the
/// settings panel. `ALL` order defines both the settings checklist order and
/// the on-screen column order when multiple are enabled.
///
/// `Artist` and `Album` are "wrap" columns: they get a proportional share of
/// the panel's flexible width (alongside the always-shown Title) and word-wrap
/// across multiple lines when a row is selected. Every other column is a
/// "flat" column: a fixed-width, single-line field. See `TrackColumn::is_wrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackColumn {
    Artist,
    Album,
    Duration,
    Year,
    Genre,
    PlayCount,
    DateAdded,
    LastPlayed,
}

impl TrackColumn {
    pub const ALL: &'static [Self] = &[
        Self::Artist,
        Self::Album,
        Self::Duration,
        Self::Year,
        Self::Genre,
        Self::PlayCount,
        Self::DateAdded,
        Self::LastPlayed,
    ];

    pub fn is_wrap(self) -> bool {
        matches!(self, Self::Artist | Self::Album)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Duration => "Duration",
            Self::Year => "Year",
            Self::Genre => "Genre",
            Self::PlayCount => "Plays",
            Self::DateAdded => "Added",
            Self::LastPlayed => "Last Played",
        }
    }

    /// Short label used in the flat-column table header (full `label()` text
    /// would overflow the fixed column width for `Duration`).
    pub fn header_label(self) -> &'static str {
        match self {
            Self::Duration => "Dur",
            other => other.label(),
        }
    }

    /// Stable identifier persisted in config.toml.
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Duration => "duration",
            Self::Year => "year",
            Self::Genre => "genre",
            Self::PlayCount => "play_count",
            Self::DateAdded => "date_added",
            Self::LastPlayed => "last_played",
        }
    }

    /// Fixed display width of a flat column. Not meaningful for wrap columns
    /// (`Artist`/`Album`), which are sized dynamically — see `is_wrap`.
    pub fn width(self) -> usize {
        match self {
            Self::Artist | Self::Album => unreachable!("wrap columns have no fixed width"),
            Self::Duration => 5,
            Self::Year => 4,
            Self::Genre => 12,
            Self::PlayCount => 5,
            Self::DateAdded => 10,
            Self::LastPlayed => 10,
        }
    }

    pub fn value(self, track: &Track) -> String {
        match self {
            Self::Artist => track.display_artist().to_string(),
            Self::Album => track.display_album().to_string(),
            Self::Duration => track.duration_str(),
            Self::Year => if track.year > 0 { track.year.to_string() } else { "-".to_string() },
            Self::Genre => if track.genre.is_empty() { "-".to_string() } else { track.genre.clone() },
            Self::PlayCount => track.play_count.to_string(),
            Self::DateAdded => format_relative(track.added_at),
            Self::LastPlayed => format_relative(track.last_played),
        }
    }

    fn is_numeric(self) -> bool {
        matches!(self, Self::Duration | Self::Year | Self::PlayCount)
    }

    /// Right-aligns numeric columns, left-aligns/truncates the rest — always
    /// exactly `self.width()` display columns wide. Flat columns only.
    fn cell_from(self, text: &str) -> String {
        use crate::util::truncate_field;
        use unicode_width::UnicodeWidthStr;
        let w = self.width();
        if self.is_numeric() {
            let tw = text.width();
            if tw >= w {
                text.to_string()
            } else {
                format!("{}{}", " ".repeat(w - tw), text)
            }
        } else {
            truncate_field(text, w)
        }
    }

    pub fn header_cell(self) -> String {
        self.cell_from(self.header_label())
    }

    pub fn value_cell(self, track: &Track) -> String {
        self.cell_from(&self.value(track))
    }

    pub fn blank_cell(self) -> String {
        " ".repeat(self.width())
    }
}

/// Total display width (including the "  " separator before each column)
/// reserved for a set of enabled flat (non-wrap) columns.
pub fn extra_columns_width(columns: &[TrackColumn]) -> usize {
    columns.iter().filter(|c| !c.is_wrap()).map(|c| c.width() + 2).sum()
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortField {
    Title,
    Artist,
    Album,
    Year,
    Genre,
    Duration,
    DateAdded,
    PlayCount,
    LastPlayed,
}

impl SortField {
    pub fn label(&self) -> &str {
        match self {
            SortField::Title => "Title",
            SortField::Artist => "Artist",
            SortField::Album => "Album",
            SortField::Year => "Year",
            SortField::Genre => "Genre",
            SortField::Duration => "Duration",
            SortField::DateAdded => "Date Added",
            SortField::PlayCount => "Play Count",
            SortField::LastPlayed => "Last Played",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SortField::Title => SortField::Artist,
            SortField::Artist => SortField::Album,
            SortField::Album => SortField::Year,
            SortField::Year => SortField::Genre,
            SortField::Genre => SortField::Duration,
            SortField::Duration => SortField::DateAdded,
            SortField::DateAdded => SortField::PlayCount,
            SortField::PlayCount => SortField::LastPlayed,
            SortField::LastPlayed => SortField::Title,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Sidebar,
    TrackList,
    Queue,
    Lyrics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarItem {
    AllTracks,
    RecentlyAdded,
    RecentlyPlayed,
    MostPlayed,
    Favorites,
    Artists,
    Artist(String),
    Albums,
    Album(String),
    Genres,
    Genre(String),
    Playlists,
    Playlist(String), // playlist name
}

impl SidebarItem {
    pub fn label(&self) -> String {
        match self {
            SidebarItem::AllTracks => "♫ All Tracks".to_string(),
            SidebarItem::Favorites => "♫ Favorites".to_string(),
            SidebarItem::RecentlyAdded => "♫ Recently Added".to_string(),
            SidebarItem::RecentlyPlayed => "♫ Recently Played".to_string(),
            SidebarItem::MostPlayed => "♫ Most Played".to_string(),
            SidebarItem::Artists => "  Artists".to_string(),
            SidebarItem::Artist(a) => format!(" {}", a),
            SidebarItem::Albums => "  Albums".to_string(),
            SidebarItem::Album(a) => format!(" {}", a),
            SidebarItem::Genres => "  Genres".to_string(),
            SidebarItem::Genre(g) => format!(" {}", g),
            SidebarItem::Playlists => "  Playlists".to_string(),
            SidebarItem::Playlist(p) => format!(" {}", p),
        }
    }

    pub fn title(&self) -> String {
        match self {
            SidebarItem::AllTracks => "All Tracks".to_string(),
            SidebarItem::RecentlyAdded => "Recently Added".to_string(),
            SidebarItem::RecentlyPlayed => "Recently Played".to_string(),
            SidebarItem::MostPlayed => "Most Played".to_string(),
            SidebarItem::Favorites => "Favorites".to_string(),
            SidebarItem::Artists => "Artists".to_string(),
            SidebarItem::Artist(a) => a.clone(),
            SidebarItem::Albums => "Albums".to_string(),
            SidebarItem::Album(a) => a.clone(),
            SidebarItem::Genres => "Genres".to_string(),
            SidebarItem::Genre(g) => g.clone(),
            SidebarItem::Playlists => "Playlists".to_string(),
            SidebarItem::Playlist(p) => p.clone(),
        }
    }

    pub fn is_header(&self) -> bool {
        matches!(
            self,
            SidebarItem::Artists
                | SidebarItem::Albums
                | SidebarItem::Genres
                | SidebarItem::Playlists
        )
    }
}

/// Whether the track list is currently showing a playlist (and which one).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackContext {
    /// Showing normal library / search results — no playlist editing.
    Library,
    /// Showing the contents of a playlist — editing is live.
    Playlist(String), // playlist name
}

/// Which field is currently active in a setup overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum SetupField {
    DatabaseName,
    MusicDirectory,
}

/// Which metadata field is currently being edited in the track info popup.
/// Ordered roughly as fields appear in the popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditField {
    // Core (also stored in DB)
    Title, Artist, Album, AlbumArtist, Year, Genre,
    // People
    Composer, Lyricist, Conductor, Producer, Remixer,
    // Position
    TrackNumber, TrackTotal, DiscNumber, DiscTotal,
    // Classical
    Work, Movement, MovementNumber, MovementTotal,
    // Descriptive
    Comment, Description, ContentGroup, Compilation,
    Mood, Language, InitialKey, Bpm,
    // Publishing
    Publisher, Label, Copyright, Isrc, Barcode, CatalogNumber,
    // Dates & origin
    RecordingDate, ReleaseDate, OriginalReleaseDate,
    OriginalArtist, OriginalAlbum,
    // Sort tags
    SortTitle, SortArtist, SortAlbum, SortAlbumArtist,
    // MusicBrainz IDs
    MusicBrainzTrackId, MusicBrainzRecordingId, MusicBrainzReleaseId, MusicBrainzArtistId,
    // ReplayGain
    ReplayGainTrackGain, ReplayGainTrackPeak, ReplayGainAlbumGain, ReplayGainAlbumPeak,
    // Technical
    EncoderSoftware,
}

impl EditField {
    /// Fields shown in the simple (default) view.
    pub const SIMPLE: &'static [Self] = &[
        Self::Title, Self::Artist, Self::Album, Self::Year, Self::Genre,
    ];

    /// Fields shown in the detailed view (in display order).
    pub const DETAILED: &'static [Self] = &[
        Self::Title, Self::Artist, Self::Album, Self::AlbumArtist, Self::Year, Self::Genre,
        Self::Composer, Self::Lyricist, Self::Conductor, Self::Producer, Self::Remixer,
        Self::TrackNumber, Self::TrackTotal, Self::DiscNumber, Self::DiscTotal,
        Self::Work, Self::Movement, Self::MovementNumber, Self::MovementTotal,
        Self::Comment, Self::Description, Self::ContentGroup, Self::Compilation,
        Self::Mood, Self::Language, Self::InitialKey, Self::Bpm,
        Self::Publisher, Self::Label, Self::Copyright, Self::Isrc, Self::Barcode, Self::CatalogNumber,
        Self::RecordingDate, Self::ReleaseDate, Self::OriginalReleaseDate,
        Self::OriginalArtist, Self::OriginalAlbum,
        Self::SortTitle, Self::SortArtist, Self::SortAlbum, Self::SortAlbumArtist,
        Self::MusicBrainzTrackId, Self::MusicBrainzRecordingId,
        Self::MusicBrainzReleaseId, Self::MusicBrainzArtistId,
        Self::ReplayGainTrackGain, Self::ReplayGainTrackPeak,
        Self::ReplayGainAlbumGain, Self::ReplayGainAlbumPeak,
        Self::EncoderSoftware,
    ];

    /// Numeric-only fields (input filtered).
    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::Year
                | Self::TrackNumber | Self::TrackTotal
                | Self::DiscNumber | Self::DiscTotal
                | Self::MovementNumber | Self::MovementTotal
                | Self::Bpm
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::AlbumArtist => "Album Artist",
            Self::Year => "Year",
            Self::Genre => "Genre",
            Self::Composer => "Composer",
            Self::Lyricist => "Lyricist",
            Self::Conductor => "Conductor",
            Self::Producer => "Producer",
            Self::Remixer => "Remixer",
            Self::TrackNumber => "Track #",
            Self::TrackTotal => "Total Tracks",
            Self::DiscNumber => "Disc #",
            Self::DiscTotal => "Total Discs",
            Self::Work => "Work",
            Self::Movement => "Movement",
            Self::MovementNumber => "Movement #",
            Self::MovementTotal => "Total Movements",
            Self::Comment => "Comment",
            Self::Description => "Description",
            Self::ContentGroup => "Grouping",
            Self::Compilation => "Compilation",
            Self::Mood => "Mood",
            Self::Language => "Language",
            Self::InitialKey => "Initial Key",
            Self::Bpm => "BPM",
            Self::Publisher => "Publisher",
            Self::Label => "Label",
            Self::Copyright => "Copyright",
            Self::Isrc => "ISRC",
            Self::Barcode => "Barcode",
            Self::CatalogNumber => "Catalog #",
            Self::RecordingDate => "Recording Date",
            Self::ReleaseDate => "Release Date",
            Self::OriginalReleaseDate => "Original Release Date",
            Self::OriginalArtist => "Original Artist",
            Self::OriginalAlbum => "Original Album",
            Self::SortTitle => "Sort · Title",
            Self::SortArtist => "Sort · Artist",
            Self::SortAlbum => "Sort · Album",
            Self::SortAlbumArtist => "Sort · Album Artist",
            Self::MusicBrainzTrackId => "MB Track ID",
            Self::MusicBrainzRecordingId => "MB Recording ID",
            Self::MusicBrainzReleaseId => "MB Release ID",
            Self::MusicBrainzArtistId => "MB Artist ID",
            Self::ReplayGainTrackGain => "RG Track Gain",
            Self::ReplayGainTrackPeak => "RG Track Peak",
            Self::ReplayGainAlbumGain => "RG Album Gain",
            Self::ReplayGainAlbumPeak => "RG Album Peak",
            Self::EncoderSoftware => "Encoder",
        }
    }
}

/// The four fixed groupings shown in the sidebar library list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarSection {
    Artists,
    Albums,
    Genres,
    Playlists,
}

impl SidebarSection {
    pub const ALL: [Self; 4] = [Self::Artists, Self::Albums, Self::Genres, Self::Playlists];

    pub fn key(self) -> &'static str {
        match self {
            Self::Artists => "Artists",
            Self::Albums => "Albums",
            Self::Genres => "Genres",
            Self::Playlists => "Playlists",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sec| sec.key() == s)
    }
}

/// Full and filtered items for a single sidebar section.
///
/// `filter_indices` is `None` when no search filter is active (avoids cloning
/// `items` on every keystroke to reset). When `Some`, it holds indices into
/// `items` in match-score order.
#[derive(Default, Debug, Clone)]
pub struct SidebarSectionState {
    pub items: Vec<String>,
    pub filter_indices: Option<Vec<usize>>,
}

impl SidebarSectionState {
    /// Iterate visible items — either all `items` (no filter) or the
    /// filtered subset in scored order.
    pub fn visible(&self) -> Box<dyn Iterator<Item = &String> + '_> {
        match &self.filter_indices {
            None => Box::new(self.items.iter()),
            Some(idx) => Box::new(idx.iter().filter_map(|&i| self.items.get(i))),
        }
    }

    pub fn visible_len(&self) -> usize {
        match &self.filter_indices {
            None => self.items.len(),
            Some(idx) => idx.len(),
        }
    }
}

/// Storage for all four sidebar sections. Accessed by enum via `get`/`get_mut`.
#[derive(Default, Debug, Clone)]
pub struct SidebarSections {
    pub artists: SidebarSectionState,
    pub albums: SidebarSectionState,
    pub genres: SidebarSectionState,
    pub playlists: SidebarSectionState,
}

impl SidebarSections {
    pub fn get(&self, s: SidebarSection) -> &SidebarSectionState {
        match s {
            SidebarSection::Artists => &self.artists,
            SidebarSection::Albums => &self.albums,
            SidebarSection::Genres => &self.genres,
            SidebarSection::Playlists => &self.playlists,
        }
    }

    pub fn get_mut(&mut self, s: SidebarSection) -> &mut SidebarSectionState {
        match s {
            SidebarSection::Artists => &mut self.artists,
            SidebarSection::Albums => &mut self.albums,
            SidebarSection::Genres => &mut self.genres,
            SidebarSection::Playlists => &mut self.playlists,
        }
    }
}

/// UI state for the lyrics panel.
#[derive(Default, Debug, Clone)]
pub struct LyricsState {
    pub visible: bool,
    pub content: Option<Lyrics>,
    pub scroll: usize,
    /// True when the user has scrolled the lyrics manually; disables auto-follow.
    pub manual_scroll: bool,
    pub track_path: Option<String>,
    pub fetch_status: LyricsFetchStatus,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LyricsFetchStatus {
    #[default]
    Idle,
    Fetching,
    NotFound,
    Error(String),
}

/// Sections available in the settings overlay. Order of variants defines display order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsSection {
    Theme,
    Library,
    Columns,
}

impl SettingsSection {
    pub const ALL: &'static [Self] = &[Self::Theme, Self::Library, Self::Columns];

    pub fn name(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::Library => "Library",
            Self::Columns => "Columns",
        }
    }
}

/// Overlay / modal modes that sit on top of the normal UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    /// Typing a name for a new playlist.
    NewPlaylist { name: String, pending_tracks: Vec<String> },
    /// Picking a playlist to add track(s) to. Selection lives in
    /// `App::add_to_playlist` so we can drive it with NavigableList.
    AddToPlaylist {
        track_paths: Vec<String>,
    },
    /// Setup a new database and music directory on first launch.
    SetupDatabase {
        database_name: String,
        music_directory: String,
        active_field: SetupField,
    },
    /// Global search overlay for finding songs, albums, artists, playlists, genres.
    GlobalSearch,
    /// Multi-section settings panel. All state lives in App::settings_* fields.
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopMode {
    Off,
    All,
    One,
}

impl LoopMode {
    pub fn label(&self) -> &str {
        match self {
            LoopMode::Off => "off",
            LoopMode::All => "all",
            LoopMode::One => "one",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            LoopMode::Off => "↦",
            LoopMode::All => "⟳",
            LoopMode::One => "⟳₁",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            LoopMode::Off => LoopMode::All,
            LoopMode::All => LoopMode::One,
            LoopMode::One => LoopMode::Off,
        }
    }
}

/// A single line of lyrics with optional timestamp.
#[derive(Debug, Clone)]
pub struct LyricLine {
    /// Timestamp in seconds (None for untimed lyrics)
    pub timestamp: Option<f64>,
    /// The lyric text
    pub text: String,
}

/// Parsed lyrics with timing information.
#[derive(Debug, Clone)]
pub enum Lyrics {
    /// Lyrics without timestamps
    Plain(String),
    /// Lyrics with timestamps (sorted by timestamp)
    Timed(Vec<LyricLine>),
}

/// Custom error type for Lyre application
#[derive(Debug)]
pub enum LyreError {
    /// Database-related errors
    Database(String),
    /// File I/O errors
    FileIO(String),
    /// Generic error with custom message
    Other(String),
}

impl std::fmt::Display for LyreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LyreError::Database(msg) => write!(f, "Database error: {}", msg),
            LyreError::FileIO(msg) => write!(f, "File I/O error: {}", msg),
            LyreError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LyreError {}

/// Convenience conversions from other error types
impl From<std::io::Error> for LyreError {
    fn from(err: std::io::Error) -> Self {
        LyreError::FileIO(err.to_string())
    }
}

impl From<rusqlite::Error> for LyreError {
    fn from(err: rusqlite::Error) -> Self {
        LyreError::Database(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for LyreError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        LyreError::Other(err.to_string())
    }
}

/// Type alias for Results using LyreError
pub type Result<T> = std::result::Result<T, LyreError>;
