use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::App;
use crate::types::{EditField, Panel};

pub(super) fn render_track_info(f: &mut Frame, app: &mut App, area: Rect) {
    // Clone scheme up front so we can hold `&mut app` freely later.
    let c = app.colors.clone();
    let overlay_bg = c.overlay_bg;

    // Popup height stays constant across simple/detailed so the album art
    // doesn't stretch — detailed view scrolls to reveal extra fields.
    let w = 100u16.min(area.width);
    let h = 22u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let title = if app.info_detailed {
        " ♫ Track Information (detailed) "
    } else {
        " ♫ Track Information "
    };
    let block = Block::default()
        .title(Span::styled(
            title,
            c.highlight_bold_style(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(c.border_active_style())
        .style(Style::default().bg(overlay_bg));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // No track available in current context
    let has_track = if app.active_panel == Panel::Queue {
        !app.player.queue.is_empty()
    } else {
        !app.filtered_tracks.is_empty()
    };
    if !has_track {
        let msg = Paragraph::new("No tracks in the current view.\n\nPress 'i' to close this window.")
            .style(c.dim_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(msg, inner);
        return;
    }

    // Ensure metadata cache is loaded for the focused track.
    app.ensure_info_cache();

    // Clone the values we need to render so we don't hold a borrow on app.
    let track_path = app.info_field_track_path.clone().unwrap_or_default();
    let values = app.info_field_values.clone();
    let readonly = app.info_readonly.clone();
    let duration_str = {
        let track = if app.active_panel == Panel::Queue {
            &app.player.queue[app.queue_index.min(app.player.queue.len() - 1)]
        } else {
            &app.filtered_tracks[app.track_list_index.min(app.filtered_tracks.len() - 1)]
        };
        track.duration_str()
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(10)])
        .split(inner);

    let art_area = cols[0];
    // Split the right column into scrolling content + a fixed 2-row footer for help.
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(cols[1]);
    let info_area = right_rows[0];
    let help_area = right_rows[1];

    if app.art.info_track_path.as_deref() != Some(&track_path) {
        app.art.info_track_path = Some(track_path.clone());
        app.art.info_image = if let Some(bytes) = crate::art::extract_cover_bytes(&track_path) {
            crate::art::load_cover_image(&bytes)
        } else {
            None
        };
        app.art.info_cache.clear();
    }

    if let Some(img) = app.art.info_image.as_ref() {
        let art_rect = Rect {
            x: art_area.x + 2,
            y: art_area.y + 1,
            width: art_area.width.saturating_sub(4),
            height: art_area.height.saturating_sub(2),
        };
        if art_rect.width > 0 && art_rect.height > 0 {
            app.art.info_cache.render(f, art_rect, img, app.art.picker.as_mut(), overlay_bg);
        }
    } else {
        let placeholder = Paragraph::new("No album art available")
            .style(c.dim_style())
            .alignment(Alignment::Center);
        f.render_widget(placeholder, art_area);
    }

    let editing = app.info_editing;
    let active = app.info_edit_field;
    let visible: &[EditField] = app.info_visible_fields();

    // Build all field lines with an index of where the active field sits.
    let label_style = c.accent_bold_style();
    let value_style = c.normal_style();
    let title_style = c.highlight_style();
    let active_style = Style::default()
        .fg(c.background)
        .bg(c.highlight)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    let mut active_line_index: Option<usize> = None;

    lines.push(Line::from(""));
    for &field in visible {
        let is_active = editing && field == active;
        if is_active {
            active_line_index = Some(lines.len());
        }
        let label = if is_active {
            format!("▶ {}", field.label())
        } else {
            field.label().to_string()
        };
        lines.push(Line::from(Span::styled(label, label_style)));

        let raw = values.get(&field).cloned().unwrap_or_default();
        let rendered = if is_active {
            format!("{}▏", raw)
        } else if raw.is_empty() {
            "—".to_string()
        } else {
            raw
        };
        let style = if is_active {
            active_style
        } else if field == EditField::Title {
            title_style
        } else {
            value_style
        };
        lines.push(Line::from(Span::styled(rendered, style)));
    }

    // Read-only section (no header — just extra fields at the bottom).
    lines.push(Line::from(Span::styled("File Path", label_style)));
    lines.push(Line::from(Span::styled(
        track_path.clone(),
        c.dim_style(),
    )));
    lines.push(Line::from(Span::styled("Duration", label_style)));
    lines.push(Line::from(Span::styled(duration_str, value_style)));
    for (label, value) in &readonly {
        lines.push(Line::from(Span::styled(label.clone(), label_style)));
        lines.push(Line::from(Span::styled(value.clone(), value_style)));
    }

    // Auto-scroll so the active field stays visible when editing, otherwise
    // honor the user's manual scroll position (clamped).
    let visible_rows = info_area.height as usize;
    let total = lines.len();
    let max_scroll = total.saturating_sub(visible_rows);
    let mut scroll = app.info_scroll.min(max_scroll);
    if let Some(idx) = active_line_index {
        let last = (idx + 1).min(total.saturating_sub(1));
        if last >= scroll + visible_rows {
            scroll = last + 1 - visible_rows;
        }
        if idx < scroll {
            scroll = idx.saturating_sub(1);
        }
    }
    app.info_scroll = scroll;
    app.info_max_scroll = max_scroll;

    let para = Paragraph::new(lines)
        .style(Style::default().bg(overlay_bg))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));
    f.render_widget(para, info_area);

    // Fixed footer with keybinding help + scroll indicator.
    let scroll_hint = if max_scroll > 0 {
        format!("  [{}/{}]", scroll + 1, max_scroll + 1)
    } else {
        String::new()
    };
    let help = if editing {
        format!("[Tab/↑↓] field [Enter] save [Esc] cancel [d] {}",
            if app.info_detailed { "simple" } else { "detailed" })
    } else {
        let mode = if app.info_detailed { "simple" } else { "detailed" };
        format!("[↑↓] scroll [e] edit [d] {} view [i] close{}", mode, scroll_hint)
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        help,
        c.dim_style(),
    )))
    .style(Style::default().bg(overlay_bg));
    f.render_widget(footer, help_area);
}
