mod app;
mod art;
mod db;
mod keybindings;
mod mpris;
mod player;
mod playlist;
mod types;
mod ui;

use crate::types::PlayerState;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mpris::{MprisCommand, MprisPlaybackStatus, MprisState};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
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
                    }
                    MprisCommand::Previous => {
                        let _ = app.player.prev();
                        app.refresh_album_art();
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
                    let art_url = if app.mpris_art_url.is_some() {
                        app.mpris_art_url.clone()
                    } else if let Some(ref bytes) = app.album_art_bytes {
                        // Use cached bytes to avoid re-extracting from file
                        let url = crate::art::write_bytes_to_temp_file(bytes);
                        app.mpris_art_url = url.clone();
                        url
                    } else {
                        // No cached bytes, need to extract (only happens on first load)
                        let url = crate::art::extract_cover_to_temp_file(&track.path);
                        app.mpris_art_url = url.clone();
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
