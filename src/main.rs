mod app;
mod art;
mod cli;
mod db;
mod indexer;
mod keybindings;
mod mpris;
mod player;
mod playlist;
mod state;
mod types;
mod ui;
mod lyrics;
mod colors;
mod config;
mod util;

use crate::types::PlayerState;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mpris::{MprisCommand, MprisPlaybackStatus, MprisState};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // ── CLI dispatch (runs without the TUI if a subcommand is given) ──────────
    // Kept outside the tokio runtime so subcommands like `lyrics` can spin up
    // their own runtimes without hitting a "runtime within a runtime" panic.
    let cli_args = cli::Cli::parse();
    if let Some(command) = cli_args.command {
        let cfg = config::load_config();
        if let cli::Commands::Config { edit } = command {
            let config_path = util::expand_tilde("~/.config/lyre/config.toml");
            if edit {
                let editor = std::env::var("VISUAL")
                    .or_else(|_| std::env::var("EDITOR"))
                    .unwrap_or_else(|_| "xdg-open".to_string());
                let _ = std::process::Command::new(&editor).arg(&config_path).status();
            } else {
                if let Ok(contents) = std::fs::read_to_string(&config_path) {
                    print!("{}", contents);
                }
            }
            return Ok(());
        }
        indexer::dispatch(command, &cfg);
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_tui())
}

async fn run_tui() -> Result<()> {
    // ── Spawn MPRIS D-Bus service ─────────────────────────────────────────────
    // If D-Bus is unavailable (e.g. headless), we fall back gracefully.
    let mpris_handles = mpris::spawn().await;
    let (mpris_tx, mut mpris_rx) = match mpris_handles {
        Ok((tx, rx)) => (Some(tx), Some(rx)),
        Err(e) => {
            eprintln!("MPRIS unavailable: {}", e);
            (None, None)
        }
    };

    // ── Setup terminal ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── Create app ────────────────────────────────────────────────────────────
    let mut app = App::new()?;

    // ── Main loop ─────────────────────────────────────────────────────────────
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        // ── Drain MPRIS commands (desktop → lyre) ─────────────────────────────
        if let Some(ref mut rx) = mpris_rx {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    MprisCommand::Play | MprisCommand::PlayPause => {
                        if app.player.state != PlayerState::Stopped {
                            app.player.toggle_pause();
                        }
                    }
                    MprisCommand::Pause => {
                        if app.player.state == PlayerState::Playing {
                            app.player.toggle_pause();
                        }
                    }
                    MprisCommand::Stop => {
                        app.player.stop();
                    }
                    MprisCommand::Next => {
                        let _ = app.player.next();
                        app.refresh_album_art();
                        app.persist_queue();
                    }
                    MprisCommand::Previous => {
                        let _ = app.player.prev();
                        app.refresh_album_art();
                        app.persist_queue();
                    }
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key) {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick();

            // ── Push state to MPRIS (lyre → desktop) ─────────────────────────
            if let Some(ref tx) = mpris_tx {
                let status = match app.player.state {
                    PlayerState::Playing => MprisPlaybackStatus::Playing,
                    PlayerState::Paused => MprisPlaybackStatus::Paused,
                    PlayerState::Stopped => MprisPlaybackStatus::Stopped,
                };
                let state = if let Some(ref track) = app.player.current_track {
                    // Reuse cached MPRIS art URL if available, otherwise generate from cached bytes
                    let art_url = if app.art.mpris_url.is_some() {
                        app.art.mpris_url.clone()
                    } else if let Some(ref bytes) = app.art.bytes {
                        // Use cached bytes to avoid re-extracting from file
                        let url = crate::art::write_bytes_to_temp_file(bytes);
                        app.art.mpris_url = url.clone();
                        url
                    } else {
                        // No cached bytes, need to extract (only happens on first load)
                        let url = crate::art::extract_cover_to_temp_file(&track.path);
                        app.art.mpris_url = url.clone();
                        url
                    };
                    MprisState {
                        status,
                        title: track.display_title().to_string(),
                        artist: track.display_artist().to_string(),
                        album: track.display_album().to_string(),
                        length: track.duration * 1_000_000,
                        position: app.player.elapsed_secs() * 1_000_000,
                        volume: app.player.volume as f64,
                        art_url,
                    }
                } else {
                    MprisState {
                        status,
                        ..Default::default()
                    }
                };
                let _ = tx.send(state);
            }

            last_tick = Instant::now();
        }
    }

    // ── Persist any lyrics fetched during this session but not yet written ────
    app.flush_all_pending_lyrics();

    // ── Restore terminal ──────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
