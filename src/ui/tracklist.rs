use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::types::Panel;
use crate::util::FAVORITE_ICON;

use super::{list_row_style, panel_block, render_search_box};

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

    let result_count = if !app.search_input.is_empty() {
        Some(app.track_list.items.len())
    } else {
        None
    };
    render_search_box(f, search_area, &app.search_input, app.search_mode, result_count, &c);

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
