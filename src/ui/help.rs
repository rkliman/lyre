use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

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
            Style::default().fg(c.highlight).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(c.accent))
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
            Style::default().fg(c.accent).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in keys {
            all_lines.push(Line::from(vec![
                Span::styled(format!("  {:28}", key), Style::default().fg(c.foreground)),
                Span::styled(desc.to_string(), Style::default().fg(c.dim)),
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
        Paragraph::new(Span::styled(footer_text, Style::default().fg(c.dim)))
            .alignment(Alignment::Center),
        footer_area,
    );
}
