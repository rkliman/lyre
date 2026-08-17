use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{Panel, PlayerState};
use crate::util::truncate_field;

use super::panel_block;

pub(super) fn render_queue(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.colors;
    let active = app.active_panel == Panel::Queue;
    use crate::keybindings::Action;
    let jump_key = app.keybindings.keys_for_action(Action::JumpToQueue);
    let total_duration: i64 = app.player.queue.iter().map(|t| t.duration).sum();
    let title = format!(
        "Queue [{}] ({} tracks, {})",
        jump_key,
        app.player.queue.len(),
        crate::types::format_duration(total_duration)
    );
    let block = panel_block(&title, active, app);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.player.queue.is_empty() {
        let add_key = app.keybindings.keys_for_action(Action::AddToQueue);
        let add_all_key = app.keybindings.keys_for_action(Action::AddAllToQueue);
        let empty = Paragraph::new(format!(
            "No tracks in queue\n\nPress [{}] on a track\nor [{}] to add all",
            add_key, add_all_key
        ))
        .style(Style::default().fg(c.dim))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        f.render_widget(empty, inner);
        return;
    }

    let w = inner.width as usize;
    let items: Vec<ListItem> = app
        .player
        .queue
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_playing =
                i == app.player.queue_index && app.player.state != PlayerState::Stopped;
            let is_selected = i == app.queue_index;

            let icon = if is_playing {
                match app.player.state {
                    PlayerState::Playing => "▶ ",
                    PlayerState::Paused => "⏸ ",
                    PlayerState::Stopped => "  ",
                }
            } else {
                "  "
            };

            let dur_str = track.duration_str();
            let dur_width = dur_str.len();
            let icon_width = 2;
            let gap = 1;
            let title_max = w.saturating_sub(icon_width + dur_width + gap);
            let title_artist = format!("{} - {}", track.display_title(), track.display_artist());
            let title_str = truncate_field(&title_artist, title_max);

            let style = if is_selected && active {
                Style::default()
                    .fg(c.highlight)
                    .bg(c.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(c.accent2).bg(c.selection_bg)
            } else if is_playing {
                Style::default().fg(c.playing).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(c.foreground)
            };

            let dur_style = if is_selected && active {
                style
            } else if is_selected {
                style
            } else {
                Style::default().fg(c.dim)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}{}", icon, title_str), style),
                Span::styled(format!(" {}", dur_str), dur_style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.queue_index));

    let list = List::new(items)
        .style(Style::default().bg(c.background))
        .highlight_style(Style::default().bg(c.selection_bg));

    f.render_stateful_widget(list, inner, &mut state);
}
