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

#[derive(Debug, Clone, PartialEq)]
pub enum SortField {
    Title,
    Artist,
    Album,
    Year,
    Genre,
    Duration,
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
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SortField::Title => SortField::Artist,
            SortField::Artist => SortField::Album,
            SortField::Album => SortField::Year,
            SortField::Year => SortField::Genre,
            SortField::Genre => SortField::Duration,
            SortField::Duration => SortField::Title,
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
            SidebarItem::AllTracks => "♫  All Tracks".to_string(),
            SidebarItem::Artists => "  Artists".to_string(),
            SidebarItem::Artist(a) => format!("    {}", a),
            SidebarItem::Albums => "  Albums".to_string(),
            SidebarItem::Album(a) => format!("    {}", a),
            SidebarItem::Genres => "  Genres".to_string(),
            SidebarItem::Genre(g) => format!("    {}", g),
            SidebarItem::Playlists => "  Playlists".to_string(),
            SidebarItem::Playlist(p) => format!("    {}", p),
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

/// Overlay / modal modes that sit on top of the normal UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    /// Typing a name for a new playlist.
    NewPlaylist(String),
    /// Picking a playlist to add track(s) to.
    AddToPlaylist {
        track_paths: Vec<String>,
        selected: usize,
    },
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
    /// Audio playback errors
    AudioPlayback(String),
    /// File I/O errors
    FileIO(String),
    /// Configuration errors
    ConfigError(String),
    /// Playlist-related errors
    PlaylistError(String),
    /// MPRIS/D-Bus errors
    MPRISError(String),
    /// Generic error with custom message
    Other(String),
}

impl std::fmt::Display for LyreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LyreError::Database(msg) => write!(f, "Database error: {}", msg),
            LyreError::AudioPlayback(msg) => write!(f, "Audio playback error: {}", msg),
            LyreError::FileIO(msg) => write!(f, "File I/O error: {}", msg),
            LyreError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            LyreError::PlaylistError(msg) => write!(f, "Playlist error: {}", msg),
            LyreError::MPRISError(msg) => write!(f, "MPRIS error: {}", msg),
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
