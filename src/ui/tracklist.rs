use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::Panel;
use crate::util::FAVORITE_ICON;

use super::{list_row_style, panel_block};

pub(super) fn render_tracklist(f: &mut Frame, app: &mut App, area: Rect) {
    let c = app.colors.clone();
    let active = app.active_panel == Panel::TrackList;

    use crate::keybindings::Action;
    let jump_key = app.keybindings.keys_for_action(Action::JumpToTracks);
    let title_str = format!(
        " Tracks [{}] — {} {} ",
        jump_key,
        app.sort_field.label(),
        if app.sort_order == crate::types::SortOrder::Asc {
            "↑"
        } else {
            "↓"
        }
    );
    let block = panel_block(&title_str, active, app).title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let heading = app.track_heading.clone();

    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let title_area = inner_rows[0];
    let search_area = inner_rows[1];
    let header_area = inner_rows[2];
    let list_area = inner_rows[3];

    let total_dur: i64 = app.track_list.items.iter().map(|t| t.duration).sum();
    let track_count = app.track_list.items.len();
    let summary = format!(
        "({} track{}, {})",
        track_count,
        if track_count == 1 { "" } else { "s" },
        crate::types::format_duration(total_dur),
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {}", heading),
                app.colors.highlight_bold_style(),
            ),
            Span::styled(
                format!("  {}", summary),
                app.colors.dim_style(),
            ),
        ])),
        Rect {
            x: title_area.x,
            y: title_area.y,
            width: title_area.width,
            height: 1,
        },
    );

    let search_active = app.search_mode;
    let has_query = !app.search_query.is_empty();

    let box_bg = if search_active { c.overlay_bg } else { c.background };
    let box_border_color = if search_active {
        c.accent
    } else if has_query {
        c.accent2
    } else {
        c.dim
    };

    let result_hint = if has_query {
        format!(
            "  {} result{}",
            app.track_list.items.len(),
            if app.track_list.items.len() == 1 { "" } else { "s" }
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

    let search_inner = search_block.inner(search_area);
    f.render_widget(search_block, search_area);

    let content = if search_active {
        Line::from(vec![
            Span::styled(
                app.search_query.clone(),
                c.highlight_bold_style().bg(box_bg),
            ),
            Span::styled("█", c.accent_style().bg(box_bg)),
        ])
    } else if has_query {
        Line::from(Span::styled(
            app.search_query.clone(),
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

    let w = inner.width as usize;
    let title_w = w * 32 / 100;
    let title_hw = title_w.saturating_sub(5);
    let artist_w = w * 28 / 100;
    let album_w = w * 28 / 100;

    let header = format!(
        "  {}  {:<tw$}  {:<aw$}  {:<bw$} {:>4}",
        FAVORITE_ICON,
        "Title",
        "Artist",
        "Album",
        "Dur",
        tw = title_hw,
        aw = artist_w,
        bw = album_w,
    );
    let header_widget = Paragraph::new(header).style(
        c.accent_bold_style().bg(c.header_bg),
    );
    f.render_widget(header_widget, header_area);

    let playing_path = app.player.current_track.as_ref().map(|t| t.path.clone());
    let visible_height = list_area.height as usize;

    if app.wrapped_width != w {
        app.rebuild_wrapped_tracks(w);
    }

    let sel = app.track_list.index;
    let row_heights: Vec<usize> = app
        .wrapped_tracks
        .iter()
        .enumerate()
        .map(|(i, (_, expanded))| if i == sel { expanded.len() } else { 1 })
        .collect();

    if !row_heights.is_empty() {
        let sel_clamped = sel.min(row_heights.len().saturating_sub(1));
        let sel_start: usize = row_heights[..sel_clamped].iter().sum();
        let sel_end = sel_start + row_heights[sel_clamped];
        let offset_rows: usize = row_heights[..app.track_list.offset.min(row_heights.len())]
            .iter()
            .sum();

        if sel_start < offset_rows {
            app.track_list.offset = sel_clamped;
        } else if sel_end > offset_rows + visible_height {
            let mut consumed = 0usize;
            let mut new_offset = sel_clamped;
            for idx in (0..=sel_clamped).rev() {
                consumed += row_heights[idx];
                if consumed >= visible_height {
                    new_offset = idx + 1;
                    break;
                }
                if idx == 0 {
                    new_offset = 0;
                }
            }
            app.track_list.offset = new_offset;
        }
    }

    let mut y = list_area.y;
    let bottom = list_area.y + list_area.height;

    for (i, (collapsed, expanded)) in app
        .wrapped_tracks
        .iter()
        .enumerate()
        .skip(app.track_list.offset)
    {
        if y >= bottom {
            break;
        }

        let track = &app.track_list.items[i];
        let is_selected = i == app.track_list.index;
        let is_in_multiselect = app.track_list.selected.contains(&i);
        let is_playing = playing_path.as_deref() == Some(&track.path);

        let row = list_row_style(&c, is_selected, is_in_multiselect, is_playing, active, &app.player.state);
        let (row_bg, fg, bold, play_icon) = (row.bg, row.fg, row.bold, row.icon);

        let render_lines: &[String] = if is_selected {
            expanded
        } else {
            std::slice::from_ref(collapsed)
        };

        for (row_idx, line_text) in render_lines.iter().enumerate() {
            if y >= bottom {
                break;
            }

            let (line_fg, line_bold) = if row_idx == 0 {
                (fg, bold)
            } else {
                let cfg = if is_selected && active { c.accent } else { c.dim };
                (cfg, false)
            };

            let text = if row_idx == 0 {
                format!("{}{}", play_icon, &line_text[1..])
            } else {
                line_text.clone()
            };

            let style = Style::default()
                .fg(line_fg)
                .bg(row_bg)
                .add_modifier(if line_bold {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(text, style)))
                    .style(Style::default().bg(row_bg)),
                Rect {
                    x: list_area.x,
                    y,
                    width: list_area.width,
                    height: 1,
                },
            );
            y += 1;
        }
    }

    while y < bottom {
        f.render_widget(
            Paragraph::new("").style(c.block_style()),
            Rect {
                x: list_area.x,
                y,
                width: list_area.width,
                height: 1,
            },
        );
        y += 1;
    }

}
