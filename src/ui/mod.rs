use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::colors::ColorScheme;
use crate::types::{Overlay, PlayerState};

mod art_window;
mod lyrics;
mod overlays;
mod player;
mod queue;
mod sidebar;
mod track_info;
mod tracklist;

use lyrics::render_lyrics;
use overlays::{render_add_to_playlist_overlay, render_global_search_overlay, render_new_playlist_overlay, render_setup_database_overlay, render_help};
use player::render_player;
use queue::render_queue;
use sidebar::render_sidebar;
use track_info::render_track_info;
use tracklist::render_tracklist;

/// Shared visual state for a single track row, used by both the track list and queue panels.
pub(super) struct ListRowStyle {
    pub bg: Color,
    pub fg: Color,
    pub bold: bool,
    /// Single-character indicator: "▶", "⏸", "*", or " ".
    pub icon: &'static str,
}

impl ListRowStyle {
    pub fn to_style(&self) -> Style {
        let s = Style::default().fg(self.fg).bg(self.bg);
        if self.bold { s.add_modifier(Modifier::BOLD) } else { s }
    }
}

pub(super) fn list_row_style(
    c: &ColorScheme,
    is_selected: bool,
    is_in_multiselect: bool,
    is_playing: bool,
    active: bool,
    player_state: &PlayerState,
) -> ListRowStyle {
    let bg = if is_selected || is_in_multiselect { c.selection_bg } else { c.background };

    let fg = if is_selected && active {
        c.highlight
    } else if is_selected {
        c.accent2
    } else if is_in_multiselect {
        c.accent
    } else if is_playing {
        c.playing
    } else {
        c.foreground
    };

    let bold = (is_selected && active) || is_in_multiselect;

    let icon = if is_playing {
        match player_state {
            PlayerState::Playing => "▶",
            PlayerState::Paused => "⏸",
            PlayerState::Stopped => " ",
        }
    } else if is_in_multiselect {
        "*"
    } else {
        " "
    };

    ListRowStyle { bg, fg, bold, icon }
}

fn render_banner(f: &mut Frame, area: Rect, app: &App) {
    let c = &app.colors;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(c.border_style())
        .style(c.block_style());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(36)])
        .split(inner);

    use crate::keybindings::Action;
    let help_key = app.keybindings.keys_for_action(Action::ToggleHelp);

    let left = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled("lyre", c.highlight_bold_style()),
        Span::styled(" ", Style::default()),
        Span::styled("v0.1.0", c.dim_style()),
        Span::styled(" ", Style::default()),
        Span::styled("─", c.dim_style()),
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("a music player & library manager. press [{}] for help.", help_key),
            c.dim_style(),
        ),
    ]);

    let right = Line::from(vec![
        Span::styled("by ", c.dim_style()),
        Span::styled("@rkliman", c.accent_style()),
        Span::styled(" ", Style::default()),
    ]);

    f.render_widget(
        Paragraph::new(left).style(c.block_style()),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .style(c.block_style()),
        cols[1],
    );
}

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // banner
            Constraint::Min(1),    // body
            Constraint::Length(5), // player bar
        ])
        .split(area);

    let banner_area = root[0];
    let body_area = root[1];
    let player_area = root[2];

    render_banner(f, banner_area, app);
    render_player(f, app, player_area);

    if app.lyrics.visible {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Min(10),
                Constraint::Percentage(28),
            ])
            .split(body_area);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body[2]);

        render_sidebar(f, app, body[0]);
        render_tracklist(f, app, body[1]);
        render_queue(f, app, right[0]);
        render_lyrics(f, app, right[1]);
    } else {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Min(10),
                Constraint::Percentage(28),
            ])
            .split(body_area);

        render_sidebar(f, app, body[0]);
        render_tracklist(f, app, body[1]);
        render_queue(f, app, body[2]);
    }

    if app.show_help {
        render_help(f, area, app);
    }

    if app.show_info {
        render_track_info(f, app, area);
    }

    // Clone the overlay so renderers can also mutate `app` (e.g. to update
    // viewport offsets on NavigableList state).
    let overlay = app.overlay.clone();
    match overlay {
        Overlay::NewPlaylist { name, .. } => render_new_playlist_overlay(f, area, app, &name),
        Overlay::AddToPlaylist { track_paths } => {
            render_add_to_playlist_overlay(f, area, app, track_paths.len())
        }
        Overlay::SetupDatabase {
            database_name,
            music_directory,
            active_field,
        } => render_setup_database_overlay(f, area, app, &database_name, &music_directory, &active_field),
        Overlay::GlobalSearch => render_global_search_overlay(f, area, app),
        Overlay::None => {}
    }
}

pub(super) fn overlay_block<'a>(title: &'a str, app: &App) -> Block<'a> {
    let c = &app.colors;
    Block::default()
        .title(Span::styled(title, c.highlight_bold_style()))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(c.border_active_style())
        .style(Style::default().bg(c.overlay_bg))
}

pub(super) fn panel_block<'a>(title: &'a str, active: bool, app: &App) -> Block<'a> {
    let c = &app.colors;
    let border_style = if active {
        c.border_active_style()
    } else {
        c.border_inactive_style()
    };

    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(if active { c.highlight } else { c.dim })
                .add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(c.block_style())
}
