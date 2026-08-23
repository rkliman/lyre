use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::colors::ColorScheme;
use crate::text_input::TextInput;
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
use overlays::{render_add_to_playlist_overlay, render_global_search_overlay, render_new_playlist_overlay, render_setup_database_overlay, render_help, render_settings_overlay};
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
        Overlay::Settings => render_settings_overlay(f, area, app),
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

pub(super) fn render_search_box(
    f: &mut Frame,
    area: Rect,
    input: &TextInput,
    active: bool,
    result_count: Option<usize>,
    colors: &ColorScheme,
) {
    let has_query = !input.is_empty();

    let box_bg = if active { colors.overlay_bg } else { colors.background };
    let box_border_color = if active {
        colors.accent
    } else if has_query {
        colors.accent2
    } else {
        colors.dim
    };

    let result_hint = match result_count {
        Some(count) if has_query => format!(
            "  {} result{}",
            count,
            if count == 1 { "" } else { "s" }
        ),
        _ => String::new(),
    };

    let box_title = if active {
        Span::styled(" search ", colors.highlight_bold_style())
    } else if has_query {
        Span::styled(format!(" search{} ", result_hint), colors.accent_style())
    } else {
        Span::styled(" search ", colors.dim_style())
    };

    let search_block = Block::default()
        .title(box_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(box_border_color))
        .style(Style::default().bg(box_bg));

    let search_inner = search_block.inner(area);
    f.render_widget(search_block, area);

    let content = if active {
        render_text_input_line(input, colors, box_bg)
    } else if has_query {
        Line::from(Span::styled(
            input.as_str().to_string(),
            colors.border_style().bg(box_bg),
        ))
    } else {
        Line::from(Span::styled(
            "press / to search\u{2026}",
            colors.dim_style().bg(box_bg),
        ))
    };

    f.render_widget(
        Paragraph::new(content).style(Style::default().bg(box_bg)),
        search_inner,
    );
}

pub(super) fn render_text_input_line(input: &TextInput, colors: &ColorScheme, bg: Color) -> Line<'static> {
    let (before, cursor_char, after) = input.split_at_cursor();
    let text_style = colors.highlight_bold_style().bg(bg);
    let mut spans = vec![Span::styled(before.to_string(), text_style)];
    if cursor_char.is_empty() {
        spans.push(Span::styled("\u{2588}", colors.accent_style().bg(bg)));
    } else {
        spans.push(Span::styled(
            cursor_char.to_string(),
            Style::default().fg(bg).bg(colors.accent),
        ));
        if !after.is_empty() {
            spans.push(Span::styled(after.to_string(), text_style));
        }
    }
    Line::from(spans)
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
