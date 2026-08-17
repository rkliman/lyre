use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{Panel, PlayerState, TrackContext};
use crate::util::FAVORITE_ICON;

use super::panel_block;

pub(super) fn render_tracklist(f: &mut Frame, app: &mut App, area: Rect) {
    let c = app.colors.clone();
    let active = app.active_panel == Panel::TrackList;

    use crate::keybindings::Action;
    let title_str = match &app.track_context {
        TrackContext::Playlist(name) => {
            let move_up_key = app.keybindings.keys_for_action(Action::MoveTrackUp);
            let move_down_key = app.keybindings.keys_for_action(Action::MoveTrackDown);
            let remove_key = app.keybindings.keys_for_action(Action::RemoveFromPlaylist);
            let add_key = app.keybindings.keys_for_action(Action::AddToPlaylist);
            format!(
                " ♪ {} [playlist]  {}/{}:move  {}:remove  {}:add-to ",
                name, move_up_key, move_down_key, remove_key, add_key
            )
        }
        TrackContext::Library => {
            let jump_key = app.keybindings.keys_for_action(Action::JumpToTracks);
            format!(
                " Tracks [{}] — {} {} ",
                jump_key,
                app.sort_field.label(),
                if app.sort_order == crate::types::SortOrder::Asc {
                    "↑"
                } else {
                    "↓"
                }
            )
        }
    };
    let block = panel_block(&title_str, active, app).title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let search_area = inner_rows[0];
    let header_area = inner_rows[1];
    let list_area = inner_rows[2];

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
            app.filtered_tracks.len(),
            if app.filtered_tracks.len() == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };

    let box_title = if search_active {
        Span::styled(
            " search ",
            Style::default().fg(c.highlight).add_modifier(Modifier::BOLD),
        )
    } else if has_query {
        Span::styled(format!(" search{} ", result_hint), Style::default().fg(c.accent))
    } else {
        Span::styled(" search ", Style::default().fg(c.dim))
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
                Style::default()
                    .fg(c.highlight)
                    .bg(box_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(c.accent).bg(box_bg)),
        ])
    } else if has_query {
        Line::from(Span::styled(
            app.search_query.clone(),
            Style::default().fg(c.accent2).bg(box_bg),
        ))
    } else {
        Line::from(Span::styled(
            "press / to search…",
            Style::default().fg(c.dim).bg(box_bg),
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
        Style::default()
            .fg(c.accent)
            .add_modifier(Modifier::BOLD)
            .bg(c.header_bg),
    );
    f.render_widget(header_widget, header_area);

    let playing_path = app.player.current_track.as_ref().map(|t| t.path.clone());
    let visible_height = list_area.height as usize;

    if app.wrapped_width != w {
        app.rebuild_wrapped_tracks(w);
    }

    let sel = app.track_list_index;
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
        let offset_rows: usize = row_heights[..app.track_list_offset.min(row_heights.len())]
            .iter()
            .sum();

        if sel_start < offset_rows {
            app.track_list_offset = sel_clamped;
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
            app.track_list_offset = new_offset;
        }
    }

    let mut y = list_area.y;
    let bottom = list_area.y + list_area.height;

    for (i, (collapsed, expanded)) in app
        .wrapped_tracks
        .iter()
        .enumerate()
        .skip(app.track_list_offset)
    {
        if y >= bottom {
            break;
        }

        let track = &app.filtered_tracks[i];
        let is_selected = i == app.track_list_index;
        let is_in_multiselect = app.selected_tracks.contains(&i);
        let is_playing = playing_path.as_deref() == Some(&track.path);

        let row_bg = if is_selected {
            c.selection_bg
        } else if is_in_multiselect {
            c.selection_bg
        } else {
            c.background
        };
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

        let play_icon = if is_playing {
            match app.player.state {
                PlayerState::Playing => "▶",
                PlayerState::Paused => "⏸",
                PlayerState::Stopped => " ",
            }
        } else if is_in_multiselect {
            "*"
        } else {
            " "
        };

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
            Paragraph::new("").style(Style::default().bg(c.background)),
            Rect {
                x: list_area.x,
                y,
                width: list_area.width,
                height: 1,
            },
        );
        y += 1;
    }

    f.render_widget(
        Paragraph::new(format!(
            " {}/{} ",
            app.track_list_index + 1,
            app.filtered_tracks.len()
        ))
        .style(Style::default().fg(c.dim))
        .alignment(Alignment::Right),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );
}
