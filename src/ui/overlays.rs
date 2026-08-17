use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::SetupField;
use super::overlay_block;

pub(super) fn render_new_playlist_overlay(f: &mut Frame, area: Rect, app: &App, name: &str) {
    let c = &app.colors;
    let w = 50u16.min(area.width);
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = overlay_block(" New Playlist ", app);

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled("Playlist name:", Style::default().fg(c.dim))),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}█", name),
            Style::default().fg(c.highlight).add_modifier(Modifier::BOLD),
        )),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to confirm · Esc to cancel",
            Style::default().fg(c.dim),
        )),
        rows[2],
    );
}

pub(super) fn render_setup_database_overlay(
    f: &mut Frame,
    area: Rect,
    app: &App,
    database_name: &str,
    music_directory: &str,
    active_field: &SetupField,
) {
    let c = &app.colors;
    let w = 80u16.min(area.width);
    let h = 11u16.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = overlay_block(" Setup Lyre ", app);

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let music_active = *active_field == SetupField::MusicDirectory;
    let db_active = *active_field == SetupField::DatabaseName;

    let music_border_color = if music_active { c.accent } else { c.dim };
    let db_border_color = if db_active { c.accent } else { c.dim };

    let music_box = Block::default()
        .title(Span::styled(
            " Music directory ",
            Style::default().fg(if music_active { c.highlight } else { c.dim }),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(music_border_color))
        .style(Style::default().bg(c.background));

    let db_box = Block::default()
        .title(Span::styled(
            " Database file ",
            Style::default().fg(if db_active { c.highlight } else { c.dim }),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(db_border_color))
        .style(Style::default().bg(c.background));

    let music_inner = music_box.inner(rows[0]);
    let db_inner = db_box.inner(rows[1]);

    f.render_widget(music_box, rows[0]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}█", music_directory),
            Style::default()
                .fg(if music_active { c.highlight } else { c.dim })
                .bg(c.background),
        )),
        music_inner,
    );
    f.render_widget(db_box, rows[1]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}█", database_name),
            Style::default()
                .fg(if db_active { c.highlight } else { c.dim })
                .bg(c.background),
        )),
        db_inner,
    );

    f.render_widget(
        Paragraph::new(Span::styled(
            "Use Tab / ↑↓ to switch fields.",
            Style::default().fg(c.dim),
        )),
        rows[2],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to confirm · Esc to cancel",
            Style::default().fg(c.dim),
        )),
        rows[3],
    );
}

pub(super) fn render_add_to_playlist_overlay(
    f: &mut Frame,
    area: Rect,
    app: &App,
    selected: usize,
    track_count: usize,
) {
    let c = &app.colors;
    let w = 50u16.min(area.width);
    let h = (app.playlists.len() as u16 + 4).min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let title = if track_count == 1 {
        " Add to Playlist ".to_string()
    } else {
        format!(" Add {} tracks to Playlist ", track_count)
    };

    let block = overlay_block(&title, app);

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .enumerate()
        .map(|(i, pl)| {
            let style = if i == selected {
                app.colors.selected_style()
            } else {
                app.colors.normal_style()
            };
            let icon = if i == selected { "▶ " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", icon, pl.name),
                style,
            )))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(items).style(Style::default().bg(c.overlay_bg));
    f.render_stateful_widget(list, rows[0], &mut state);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to add · Esc to cancel",
            app.colors.dim_style(),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );
}
