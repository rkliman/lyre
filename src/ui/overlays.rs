use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{AddToPlaylistItem, GlobalSearchResult, SetupField};
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
        Paragraph::new(Span::styled("Playlist name:", c.dim_style())),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}█", name),
            c.highlight_bold_style(),
        )),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to confirm · Esc to cancel",
            c.dim_style(),
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
        .style(c.block_style());

    let db_box = Block::default()
        .title(Span::styled(
            " Database file ",
            Style::default().fg(if db_active { c.highlight } else { c.dim }),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(db_border_color))
        .style(c.block_style());

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
            c.dim_style(),
        )),
        rows[2],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to confirm · Esc to cancel",
            c.dim_style(),
        )),
        rows[3],
    );
}

pub(super) fn render_add_to_playlist_overlay(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    track_count: usize,
) {
    let c = &app.colors;
    let w = 50u16.min(area.width);
    let h = (app.add_to_playlist.len() as u16 + 4).min(area.height);
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

    let list_area = rows[0];
    let visible_height = list_area.height as usize;
    app.add_to_playlist.ensure_visible(visible_height);
    let offset = app.add_to_playlist.offset;
    let index = app.add_to_playlist.index;

    f.render_widget(
        Paragraph::new("").style(Style::default().bg(c.overlay_bg)),
        list_area,
    );

    for (row_i, (i, item)) in app
        .add_to_playlist
        .items
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .enumerate()
    {
        let is_selected = i == index;
        let icon = if is_selected { "▶ " } else { "  " };
        let (label, style) = match item {
            AddToPlaylistItem::NewPlaylist => {
                let style = if is_selected {
                    c.selected_style()
                } else {
                    c.dim_style().bg(c.overlay_bg)
                };
                (format!("{}+ New Playlist", icon), style)
            }
            AddToPlaylistItem::Existing(idx) => {
                let name = app
                    .playlists
                    .get(*idx)
                    .map(|p| p.name.as_str())
                    .unwrap_or("");
                let style = if is_selected {
                    c.selected_style()
                } else {
                    c.normal_style().bg(c.overlay_bg)
                };
                (format!("{}{}", icon, name), style)
            }
        };
        let bg = if is_selected { c.selection_bg } else { c.overlay_bg };
        let line_area = Rect {
            x: list_area.x,
            y: list_area.y + row_i as u16,
            width: list_area.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(label, style)))
                .style(Style::default().bg(bg)),
            line_area,
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter to add · Esc to cancel",
            app.colors.dim_style(),
        ))
        .alignment(Alignment::Center)
        .style(Style::default().bg(c.overlay_bg)),
        rows[1],
    );
}

pub(super) fn render_global_search_overlay(f: &mut Frame, area: Rect, app: &mut App) {
    let c = &app.colors;

    let results: Vec<String> = app
        .global_search
        .items
        .iter()
        .map(|r| match r {
            GlobalSearchResult::Track(t) => {
                format!("🎵 {} — {}", t.display_title(), t.display_artist())
            }
            GlobalSearchResult::Album(name) => format!("💿 {}", name),
            GlobalSearchResult::Artist(name) => format!("🎤 {}", name),
            GlobalSearchResult::Playlist(name) => format!("📃 {}", name),
            GlobalSearchResult::Genre(name) => format!("🎸 {}", name),
        })
        .collect();

    let has_results = !results.is_empty();
    let empty_message = (!has_results && !app.global_search_query.is_empty())
        .then(|| "No results found.".to_string());

    // Size the popup to fit content: border(2) + search box(3) + content + footer(1 if results)
    let display_line_count = if has_results {
        results.len() as u16
    } else {
        empty_message.is_some() as u16
    };
    let footer_h: u16 = if has_results || !app.global_search_query.is_empty() { 1 } else { 0 };
    let natural_h = 2 + 4 + display_line_count + footer_h;
    let max_h = area.height * 80 / 100;
    let h = natural_h.max(5).min(max_h).min(area.height);

    let w = (area.width * 50 / 100).max(40).min(area.width);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let top = area.y + area.height * 15 / 100;
    let max_from_top = area.height.saturating_sub(top - area.y);
    let h = h.min(max_from_top);
    let popup = Rect { x, y: top, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = overlay_block(" 📂 Library Search ", app);
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let constraints = if footer_h > 0 {
        vec![
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Length(1), Constraint::Length(3), Constraint::Min(0)]
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let help_area = layout[0];
    let search_area = layout[1];
    let results_area = layout[2];

    // Search input box
    let search_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(c.border_active_style())
        .style(Style::default().bg(c.overlay_bg));

    let search_inner = search_box.inner(search_area);
    f.render_widget(search_box, search_area);

    let query_display = Line::from(vec![
        Span::styled(
            app.global_search_query.as_str(),
            c.highlight_bold_style(),
        ),
        Span::styled("█", c.accent_style()),
    ]);
    f.render_widget(
        Paragraph::new(query_display).style(Style::default().bg(c.overlay_bg)),
        search_inner,
    );

    let help_text = Line::from(vec![
        Span::styled(
            "🎵 Song - 💿 Album - 🎤 Artist - 📃 Playlist",
             c.header_style(),
        )]
    );

    //help
    f.render_widget(Paragraph::new(help_text).alignment(Alignment::Center), help_area);

    // Results
    let visible_height = results_area.height as usize;
    let max_text_width = results_area.width.saturating_sub(5) as usize;

    if has_results {
        app.global_search.ensure_visible(visible_height);
        let offset = app.global_search.offset;
        let index = app.global_search.index;

        for (row_i, (i, text)) in
            results.iter().enumerate().skip(offset).take(visible_height).enumerate()
        {
            let line_area = Rect {
                x: results_area.x,
                y: results_area.y + row_i as u16,
                width: results_area.width,
                height: 1,
            };
            let is_selected = i == index;
            let icon = if is_selected { "• " } else { "  " };
            let style = if is_selected {
                c.selected_style()
            } else {
                c.normal_style().bg(c.overlay_bg)
            };
            let display_text: String = if text.chars().count() > max_text_width {
                let truncated: String =
                    text.chars().take(max_text_width.saturating_sub(1)).collect();
                format!("{}…", truncated)
            } else {
                text.clone()
            };
            let bg = if is_selected { c.selection_bg } else { c.overlay_bg };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" {}{}", icon, display_text),
                    style,
                )))
                .style(Style::default().bg(bg)),
                line_area,
            );
        }
    } else if let Some(msg) = &empty_message {
        let line_area = Rect {
            x: results_area.x,
            y: results_area.y,
            width: results_area.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", msg),
                c.dim_style().bg(c.overlay_bg),
            )))
            .style(Style::default().bg(c.overlay_bg)),
            line_area,
        );
    }

    // Footer
    if footer_h > 0 {
        let footer_area = layout[3];
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Enter to select · Esc to cancel",
                c.dim_style(),
            )))
            .alignment(Alignment::Center)
            .style(Style::default().bg(c.overlay_bg)),
            footer_area,
        );
    }
}

pub(super) fn render_help(f: &mut Frame, area: Rect, app: &mut App) {
    let c = &app.colors;
    let w = 90u16.min(area.width);
    let h = area.height.saturating_sub(4);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            " ♫ Lyre — Keybindings ",
            c.highlight_bold_style(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(c.border_active_style())
        .style(Style::default().bg(c.overlay_bg));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let content_area = layout[0];
    let footer_area = layout[1];

    let sections = app.keybindings.help_sections();

    let mut all_lines: Vec<Line> = Vec::new();
    for (section, keys) in &sections {
        all_lines.push(Line::from(Span::styled(
            section.to_string(),
            c.accent_bold_style(),
        )));
        for (key, desc) in keys {
            all_lines.push(Line::from(vec![
                Span::styled(format!("  {:28}", key), c.normal_style()),
                Span::styled(desc.to_string(), c.dim_style()),
            ]));
        }
        all_lines.push(Line::from(""));
    }

    let visible_height = content_area.height as usize;
    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_offset = app.help_scroll.min(max_scroll);

    app.help_scroll = scroll_offset;

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let para = Paragraph::new(visible_lines).style(Style::default().bg(c.overlay_bg));
    f.render_widget(para, content_area);

    let footer_text = if total_lines > visible_height {
        format!(
            "↑/↓ to scroll ({}/{}) · Esc to close",
            scroll_offset + 1,
            total_lines - visible_height + 1
        )
    } else {
        "Esc or ? to close".to_string()
    };

    f.render_widget(
        Paragraph::new(Span::styled(footer_text, c.dim_style()))
            .alignment(Alignment::Center),
        footer_area,
    );
}

