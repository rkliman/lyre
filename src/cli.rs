use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lyre",
    about = "Music player and library manager — run without arguments to open the TUI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index the music library and playlists
    Index {
        /// Show what would be moved but don't actually move files
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        /// Music directory to index (overrides config); the db is written there
        #[arg()]
        source: Option<String>,
    },
    /// Find duplicate tracks
    Dupes {
        /// Interactively fix duplicates
        #[arg(long, action = ArgAction::SetTrue)]
        fix: bool,
    },
    /// List all tracks
    Ls {
        /// Search query
        #[arg()]
        query: Option<String>,
        /// Filter by genre
        #[arg(long)]
        genre: Option<String>,
    },
    /// Export tracks to CSV
    Export,
    /// Show statistics
    Stats,
    /// Search library
    Search {
        #[arg(required = true)]
        query: String,
    },
    /// List all genres
    Genres,
    /// Compress audio files to mp3/aac/opus for mobile sync
    Compress {
        /// Output directory for compressed files
        #[arg(long, short = 'o')]
        output_dir: String,
        /// Output format (mp3, aac, opus)
        #[arg(long, default_value = "mp3")]
        format: String,
        /// Audio bitrate (e.g. 128k, 192k, 256k)
        #[arg(long, default_value = "192k")]
        bitrate: String,
        /// Number of parallel jobs (default: CPU count)
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        /// Re-convert even if the output file already exists
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        /// Optional search query to filter tracks
        #[arg()]
        query: Option<String>,
    },
    /// Fetch and embed lyrics from lrclib.net
    Lyrics {
        /// Optional search query to filter tracks
        #[arg()]
        query: Option<String>,
        /// Overwrite existing unsynced lyrics with synced lyrics
        #[arg(long, action = ArgAction::SetTrue)]
        overwrite: bool,
        /// Show which tracks would be updated without modifying any files
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
    },
    /// Show detailed info about a track
    Info {
        #[arg(required = true)]
        query: String,
    },
    /// Show or edit the configuration file
    Config {
        /// Open the config file in the system editor
        #[arg(long, action = ArgAction::SetTrue)]
        edit: bool,
    },
    /// Sync two music libraries by comparing their databases
    Sync {
        /// Path to the source database (music dir is its parent)
        #[arg()]
        src: String,
        /// Path to the destination database (music dir is its parent)
        #[arg()]
        dst: String,
        /// Show what would change without copying or deleting anything
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        /// Skip deleting tracks from destination that are absent in source
        #[arg(long, action = ArgAction::SetTrue)]
        no_delete: bool,
    },
}
