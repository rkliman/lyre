use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{Panel, SidebarItem};

use super::art_window::render_art_window;
use super::panel_block;

pub(super) fn render_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let c = app.colors.clone();
    let active = app.active_panel == Panel::Sidebar;

    let (sidebar_area, art_area_opt) = if app.art_window_visible {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    if let Some(art_area) = art_area_opt {
        render_art_window(f, app, art_area);
    }

    use crate::keybindings::Action;
    let jump_key = app.keybindings.keys_for_action(Action::JumpToSidebar);
    let title = format!("Library [{}]", jump_key);
    let block = panel_block(&title, active, app);
    let inner = block.inner(sidebar_area);
    f.render_widget(block, sidebar_area);

    let mut display_items: Vec<(usize, Line)> = Vec::new();
    let mut search_boxes: Vec<(usize, String, String)> = Vec::new();

    for (i, item) in app.sidebar_list.items.iter().enumerate() {
        let is_selected = i == app.sidebar_list.index;

        let style = if item.is_header() {
            c.accent_bold_style()
        } else if is_selected && active {
            c.selected_style()
        } else if is_selected {
            c.selected_accent2_style()
        } else {
            c.normal_style()
        };

        let label = if item.is_header() {
            let section_key = match item {
                SidebarItem::Artists => "Artists",
                SidebarItem::Albums => "Albums",
                SidebarItem::Genres => "Genres",
                SidebarItem::Playlists => "Playlists",
                _ => "",
            };
            let expanded = *app.sidebar_expanded.get(section_key).unwrap_or(&true);
            let chevron = if expanded { "▼ " } else { "▶ " };

            if expanded && !section_key.is_empty() {
                if let Some(search_section) = &app.sidebar_search_section {
                    if search_section.as_str() == section_key
                        && (app.sidebar_search_mode || !app.sidebar_search_query.is_empty())
                    {
                        search_boxes.push((
                            display_items.len() + 1,
                            section_key.to_string(),
                            app.sidebar_search_query.clone(),
                        ));
                    }
                }
            }

            format!("{}{}", chevron, item.label().trim_start())
        } else {
            item.label()
        };

        display_items.push((i, Line::from(Span::styled(label, style))));
    }

    let mut selected_display_line = 0usize;
    let mut current_line = 0usize;
    for (display_idx, (original_idx, _)) in display_items.iter().enumerate() {
        if search_boxes.iter().any(|(pos, _, _)| *pos == display_idx) {
            current_line += 3;
        }
        if *original_idx == app.sidebar_list.index {
            selected_display_line = current_line;
        }
        current_line += 1;
    }

    let visible_height = inner.height as usize;

    let total_display_lines = current_line;
    if total_display_lines <= visible_height {
        app.sidebar_list.offset = 0;
    } else if app.sidebar_list.offset > total_display_lines - visible_height {
        app.sidebar_list.offset = total_display_lines - visible_height;
    }

    if selected_display_line < app.sidebar_list.offset {
        app.sidebar_list.offset = selected_display_line;
    } else if selected_display_line >= app.sidebar_list.offset + visible_height {
        app.sidebar_list.offset = selected_display_line.saturating_sub(visible_height - 1);
    }

    let mut y_offset = 0u16;
    let mut current_display_line = 0usize;

    for (display_idx, (original_idx, line)) in display_items.iter().enumerate() {
        if let Some((_, section, query)) = search_boxes.iter().find(|(pos, _, _)| *pos == display_idx) {
            if current_display_line + 3 <= app.sidebar_list.offset {
                current_display_line += 3;
                continue;
            }

            if current_display_line < app.sidebar_list.offset + visible_height {
                let skip_lines = app.sidebar_list.offset.saturating_sub(current_display_line);
                if skip_lines < 3 {
                    let search_area = Rect {
                        x: inner.x,
                        y: inner.y + y_offset,
                        width: inner.width,
                        height: (3 - skip_lines).min((visible_height - y_offset as usize).min(3)) as u16,
                    };
                    render_sidebar_search_box(f, app, search_area, section, query);
                    y_offset += search_area.height;
                }
            }
            current_display_line += 3;
        }

        if current_display_line < app.sidebar_list.offset {
            current_display_line += 1;
            continue;
        }

        if y_offset >= inner.height {
            break;
        }

        let item_area = Rect {
            x: inner.x,
            y: inner.y + y_offset,
            width: inner.width,
            height: 1,
        };

        let is_highlighted = *original_idx == app.sidebar_list.index;
        let item_style = if is_highlighted {
            Style::default().bg(c.selection_bg)
        } else {
            c.block_style()
        };

        f.render_widget(
            Paragraph::new(line.clone()).style(item_style),
            item_area,
        );

        y_offset += 1;
        current_display_line += 1;
    }
}

fn render_sidebar_search_box(f: &mut Frame, app: &App, area: Rect, section: &str, query: &str) {
    let c = &app.colors;
    let search_active = app.sidebar_search_mode && app.sidebar_search_section.as_deref() == Some(section);
    let has_query = !query.is_empty();

    let box_bg = if search_active { c.overlay_bg } else { c.background };
    let box_border_color = if search_active {
        c.accent
    } else if has_query {
        c.accent2
    } else {
        c.dim
    };

    let result_count = crate::types::SidebarSection::from_key(section)
        .map(|s| app.sidebar.get(s).visible_len())
        .unwrap_or(0);

    let result_hint = if has_query {
        format!(
            "  {} result{}",
            result_count,
            if result_count == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };

    let box_title = if search_active {
        Span::styled(" search ", c.highlight_bold_style())
    } else if has_query {
        Span::styled(format!(" search{} ", result_hint), c.accent_style())
    } else {
        Span::styled(" search ", c.dim_style())
    };

    let search_block = Block::default()
        .title(box_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(box_border_color))
        .style(Style::default().bg(box_bg));

    let search_inner = search_block.inner(area);
    f.render_widget(search_block, area);

    let content = if search_active {
        Line::from(vec![
            Span::styled(
                query.to_string(),
                c.highlight_bold_style().bg(box_bg),
            ),
            Span::styled("█", c.accent_style().bg(box_bg)),
        ])
    } else if has_query {
        Line::from(Span::styled(
            query.to_string(),
            c.border_style().bg(box_bg),
        ))
    } else {
        Line::from(Span::styled(
            "press / to search…",
            c.dim_style().bg(box_bg),
        ))
    };

    f.render_widget(
        Paragraph::new(content).style(Style::default().bg(box_bg)),
        search_inner,
    );
}
