use crate::cli::Commands;
use crate::config::{Config, DEFAULT_DATABASE_PATH};
use crate::util::expand_tilde;

mod compress;
mod dedupe;
mod export;
mod index;
mod info;
pub mod lrclib;
mod lyrics;
mod query;
mod stats;
mod sync;
mod util;

use compress::compress_tracks;
use dedupe::find_duplicates;
use export::export_tracks;
use index::{index_library, index_playlists};
use info::get_info;
use lyrics::add_lyrics;
use query::{list_genres, list_tracks, search_tracks};
use stats::get_stats;
use sync::sync_libraries;

pub fn dispatch(command: Commands, config: &Config) {
    let music_dir = expand_tilde(&config.files.music_directory);
    let db_path = expand_tilde(&config.files.database_name);

    match command {
        Commands::Index { dry_run, all, source } => {
            let (eff_music_dir, eff_db_path) = if let Some(src) = source {
                let src = expand_tilde(&src);
                let db_filename = std::path::Path::new(&db_path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        std::path::Path::new(DEFAULT_DATABASE_PATH)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string()
                    });
                let eff_db = format!("{}/{}", src, db_filename);
                (src, eff_db)
            } else {
                (music_dir, db_path)
            };
            index_library(
                &eff_music_dir,
                &eff_db_path,
                config.files.file_pattern.as_deref(),
                config.files.ignore.as_ref(),
                config.replace.as_ref(),
                dry_run,
                all,
            );
            index_playlists(&eff_music_dir, &eff_db_path);
        }
        Commands::Dupes { fix } => find_duplicates(&db_path, &music_dir, fix),
        Commands::Ls { query, genre } => list_tracks(&db_path, query, genre),
        Commands::Export => export_tracks(&db_path),
        Commands::Stats => get_stats(&music_dir, &db_path),
        Commands::Search { query } => search_tracks(&db_path, Some(query)),
        Commands::Genres => list_genres(&db_path),
        Commands::Compress {
            output_dir,
            format,
            bitrate,
            jobs,
            force,
            query,
        } => compress_tracks(&music_dir, &db_path, &output_dir, &format, &bitrate, jobs, force, query),
        Commands::Lyrics {
            query,
            overwrite,
            dry_run,
        } => add_lyrics(&db_path, &music_dir, query, overwrite, dry_run),
        Commands::Info { query } => get_info(&db_path, &music_dir, &query),
        Commands::Config { .. } => {}
        Commands::Sync { src, dst, dry_run, no_delete } => {
            sync_libraries(&src, &dst, dry_run, no_delete)
        }
    }
}
