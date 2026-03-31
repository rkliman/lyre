use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};
use ratatui_image::StatefulImage;

use crate::{app::App, types::LoopMode};
use crate::types::format_duration;
use crate::types::{Overlay, Panel, PlayerState, SidebarItem, TrackContext};

fn render_banner(f: &mut Frame, area: Rect, app: &App) {
    let c = &app.colors;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c.accent2))
        .style(Style::default().bg(c.background));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner into left (lyre glyph + title) and right (author + version)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(36)])
        .split(inner);

    // ── Left: glyph + app name ────────────────────────────────────────────────
    use crate::keybindings::Action;
    let help_key = app.keybindings.keys_for_action(Action::ToggleHelp);

    let left = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            "lyre",
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled("v0.1.0", Style::default().fg(c.dim)),
        Span::styled(" ", Style::default()),
        Span::styled("─", Style::default().fg(c.dim)),
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("a music player & library manager. press '{}' for help.", help_key),
            Style::default().fg(c.dim),
        ),
    ]);

    // ── Right: author + version ───────────────────────────────────────────────
    let right = Line::from(vec![
        Span::styled("by ", Style::default().fg(c.dim)),
        Span::styled("@rkliman", Style::default().fg(c.accent)),
        Span::styled(" ", Style::default()),
    ]);

    f.render_widget(
        Paragraph::new(left).style(Style::default().bg(c.background)),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .style(Style::default().bg(c.background)),
        cols[1],
    );
}

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Top-level layout: banner + body + player bar ─────────────────────────
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

    // ── Body: sidebar | tracklist | queue | lyrics (optional) ────────────────
    render_banner(f, banner_area, app);
    render_player(f, app, player_area);

    if app.lyrics_visible {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // sidebar
                Constraint::Min(10),        // track list
                Constraint::Percentage(28), // queue
                Constraint::Percentage(28), // lyrics
            ])
            .split(body_area);

        render_sidebar(f, app, body[0]);
        render_tracklist(f, app, body[1]);
        render_queue(f, app, body[2]);
        render_lyrics(f, app, body[3]);
    } else {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // sidebar
                Constraint::Min(10),        // track list
                Constraint::Percentage(28), // queue
            ])
            .split(body_area);

        render_sidebar(f, app, body[0]);
        render_tracklist(f, app, body[1]);
        render_queue(f, app, body[2]);
    }

    if app.show_help {
        render_help(f, area, app);
    }

    // Overlays render on top of everything
    match &app.overlay {
        Overlay::NewPlaylist(name) => render_new_playlist_overlay(f, area, app, name),
        Overlay::AddToPlaylist { track_paths, selected } => {
            render_add_to_playlist_overlay(f, area, app, *selected, track_paths.len())
        }
        Overlay::None => {}
    }
}

fn panel_block<'a>(title: &'a str, active: bool, app: &App) -> Block<'a> {
    let c = &app.colors;
    let border_style = if active {
        Style::default().fg(c.accent)
    } else {
        Style::default().fg(c.dim)
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
        .style(Style::default().bg(c.background))
}

fn render_art_window(f: &mut Frame, app: &mut App, area: Rect) {
    let c = &app.colors;
    use crate::keybindings::Action;
    let toggle_key = app.keybindings.keys_for_action(Action::ToggleArtWindow);
    let block = Block::default()
        .title(Span::styled(
            format!(" Album Art [{}] ", toggle_key),
            Style::default().fg(c.accent).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c.accent2))
        .style(Style::default().bg(c.background));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Use the full inner space for the art
    let art_rect = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height,
    };

    if art_rect.width > 0 && art_rect.height > 0 {
        if let Some(img) = app.album_art_image.as_ref() {
            // Try to use terminal graphics if available
            if let Some(picker) = app.image_picker.as_mut() {
                // Use cached protocol if dimensions haven't changed, otherwise regenerate
                let current_dims = (art_rect.width, art_rect.height);
                if app.cached_art_window_dims != current_dims || app.cached_art_window_protocol.is_none() {
                    // Dimensions changed or no cache - explicitly drop old protocol and regenerate
                    // This ensures proper cleanup of terminal graphics resources
                    app.cached_art_window_protocol = None; // Explicitly drop old protocol
                    let protocol = picker.new_resize_protocol(img.clone());
                    app.cached_art_window_protocol = Some(protocol);
                    app.cached_art_window_dims = current_dims;
                }

                // Render using the cached protocol
                if let Some(protocol) = app.cached_art_window_protocol.as_mut() {
                    let image_widget = StatefulImage::new(None);
                    f.render_stateful_widget(image_widget, art_rect, protocol);
                }
            } else {
                // Fall back to block art - use cache if dimensions haven't changed
                let current_dims = (art_rect.width, art_rect.height);
                if app.cached_art_window_dims != current_dims || app.cached_art_window_block.is_none() {
                    // Dimensions changed or no cache - regenerate
                    let art = crate::art::render_block_art_from_image(
                        img,
                        art_rect.width,
                        art_rect.height,
                    );
                    app.cached_art_window_block = Some(art);
                    app.cached_art_window_dims = current_dims;
                }

                // Render the cached block art
                if let Some(art) = &app.cached_art_window_block {
                    for (i, row) in art.rows.iter().enumerate().take(art_rect.height as usize) {
                        let row_rect = Rect {
                            x: art_rect.x,
                            y: art_rect.y + i as u16,
                            width: art_rect.width,
                            height: 1,
                        };
                        f.render_widget(
                            Paragraph::new(row.clone()).style(Style::default().bg(c.background)),
                            row_rect,
                        );
                    }
                }
            }
        } else {
            // No album art available - show placeholder
            use crate::keybindings::Action;
            let toggle_key = app.keybindings.keys_for_action(Action::ToggleArtWindow);
            let placeholder = Paragraph::new(format!("No album art available\n\nPress {} to hide", toggle_key))
                .style(Style::default().fg(c.dim))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(placeholder, art_rect);
        }
    }
}

fn render_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let c = app.colors.clone();
    let active = app.active_panel == Panel::Sidebar;

    // Split sidebar area if art window is visible
    let (sidebar_area, art_area_opt) = if app.art_window_visible {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // library list
                Constraint::Percentage(50), // art window
            ])
            .split(area);
        (split[0], Some(split[1]))
    } else {
        (area, None)
    };

    // Render art window if visible
    if let Some(art_area) = art_area_opt {
        render_art_window(f, app, art_area);
    }

    // Render library list
    use crate::keybindings::Action;
    let jump_key = app.keybindings.keys_for_action(Action::JumpToSidebar);
    let title = format!("Library [{}]", jump_key);
    let block = panel_block(&title, active, app);
    let inner = block.inner(sidebar_area);
    f.render_widget(block, sidebar_area);

    // Build items list, inserting search boxes where needed
    let mut display_items: Vec<(usize, Line)> = Vec::new();  // (original_index, display_line)
    let mut search_boxes: Vec<(usize, String, String)> = Vec::new();  // (insert_position, section_name, query)

    for (i, item) in app.sidebar_items.iter().enumerate() {
        let is_selected = i == app.sidebar_index;

        let style = if item.is_header() {
            Style::default().fg(c.accent).add_modifier(Modifier::BOLD)
        } else if is_selected && active {
            Style::default()
                .fg(c.highlight)
                .bg(c.selection_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(c.accent2).bg(c.selection_bg)
        } else {
            Style::default().fg(c.foreground)
        };

        // For headers, prepend an expand/collapse chevron
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

            // Check if we should show a search box after this header
            if expanded && !section_key.is_empty() {
                if let Some(search_section) = &app.sidebar_search_section {
                    if search_section == section_key && (app.sidebar_search_mode || !app.sidebar_search_query.is_empty()) {
                        search_boxes.push((display_items.len() + 1, section_key.to_string(), app.sidebar_search_query.clone()));
                    }
                }
            }

            format!("{}{}", chevron, item.label().trim_start())
        } else {
            item.label()
        };

        display_items.push((i, Line::from(Span::styled(label, style))));
    }

    // Render items manually line by line to insert search boxes
    let mut y_offset = 0u16;

    for (display_idx, (original_idx, line)) in display_items.iter().enumerate() {
        if y_offset >= inner.height {
            break;
        }

        // Check if we need to render a search box before this item
        if let Some((_, section, query)) = search_boxes.iter().find(|(pos, _, _)| *pos == display_idx) {
            // Render search box
            if y_offset + 3 <= inner.height {
                let search_area = Rect {
                    x: inner.x,
                    y: inner.y + y_offset,
                    width: inner.width,
                    height: 3,
                };
                render_sidebar_search_box(f, app, search_area, section, query);
                y_offset += 3;
            }
        }

        if y_offset >= inner.height {
            break;
        }

        // Render the item
        let item_area = Rect {
            x: inner.x,
            y: inner.y + y_offset,
            width: inner.width,
            height: 1,
        };

        let is_highlighted = *original_idx == app.sidebar_index;
        let item_style = if is_highlighted {
            Style::default().bg(c.selection_bg)
        } else {
            Style::default().bg(c.background)
        };

        f.render_widget(
            Paragraph::new(line.clone()).style(item_style),
            item_area,
        );

        y_offset += 1;
    }
}

fn render_sidebar_search_box(f: &mut Frame, app: &App, area: Rect, section: &str, query: &str) {
    let c = &app.colors;
    let search_active = app.sidebar_search_mode && app.sidebar_search_section.as_deref() == Some(section);
    let has_query = !query.is_empty();

    let box_bg = if search_active {
        c.overlay_bg
    } else {
        c.background
    };
    let box_border_color = if search_active {
        c.accent
    } else if has_query {
        c.accent2
    } else {
        c.dim
    };

    let result_count = match section {
        "Artists" => app.filtered_sidebar_artists.len(),
        "Albums" => app.filtered_sidebar_albums.len(),
        "Genres" => app.filtered_sidebar_genres.len(),
        _ => 0,
    };

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
        Span::styled(
            " search ",
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        )
    } else if has_query {
        Span::styled(
            format!(" search{} ", result_hint),
            Style::default().fg(c.accent),
        )
    } else {
        Span::styled(" search ", Style::default().fg(c.dim))
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
                Style::default()
                    .fg(c.highlight)
                    .bg(box_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(c.accent).bg(box_bg)),
        ])
    } else if has_query {
        Line::from(Span::styled(
            query.to_string(),
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
}

fn render_tracklist(f: &mut Frame, app: &mut App, area: Rect) {
    let c = app.colors.clone();
    let active = app.active_panel == Panel::TrackList;

    use crate::keybindings::Action;
    // Build title — show playlist name or sort info
    let title_str = match &app.track_context {
        TrackContext::Playlist(name) => {
            let move_up_key = app.keybindings.keys_for_action(Action::MoveTrackUp);
            let move_down_key = app.keybindings.keys_for_action(Action::MoveTrackDown);
            let remove_key = app.keybindings.keys_for_action(Action::RemoveFromPlaylist);
            let add_key = app.keybindings.keys_for_action(Action::AddToPlaylist);
            format!(" ♪ {} [playlist]  {}/{}:move  {}:remove  {}:add-to ",
                name, move_up_key, move_down_key, remove_key, add_key)
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

    // ── Inner layout: search box | column headers | track rows ──────────────
    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // bordered search box
            Constraint::Length(1), // column headers
            Constraint::Min(1),    // track list
        ])
        .split(inner);

    let search_area = inner_rows[0];
    let header_area = inner_rows[1];
    let list_area = inner_rows[2];

    // ── Search box ────────────────────────────────────────────────────────────
    let search_active = app.search_mode;
    let has_query = !app.search_query.is_empty();

    let box_bg = if search_active {
        c.overlay_bg
    } else {
        c.background
    };
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
            if app.filtered_tracks.len() == 1 {
                ""
            } else {
                "s"
            }
        )
    } else {
        String::new()
    };

    let box_title = if search_active {
        Span::styled(
            " search ",
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        )
    } else if has_query {
        Span::styled(
            format!(" search{} ", result_hint),
            Style::default().fg(c.accent),
        )
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

    // Render column headers
    // Layout: [icon] Title  Artist  Album  Genre  Dur
    let w = inner.width as usize;
    let title_w = w * 28 / 100;
    let artist_w = w * 22 / 100;
    let album_w = w * 22 / 100;
    let genre_w = w * 16 / 100;

    let header = format!(
        "{:<tw$}  {:<aw$}  {:<bw$}  {:<gw$}  {:>4}",
        "Title",
        "Artist",
        "Album",
        "Genre",
        "Dur",
        tw = title_w,
        aw = artist_w,
        bw = album_w,
        gw = genre_w
    );
    let header_widget = Paragraph::new(header).style(
        Style::default()
            .fg(c.accent)
            .add_modifier(Modifier::BOLD)
            .bg(c.header_bg),
    );
    f.render_widget(header_widget, header_area);

    // ── Variable-height track rows ────────────────────────────────────────────
    let playing_path = app.player.current_track.as_ref().map(|t| t.path.clone());
    let visible_height = list_area.height as usize;

    // Rebuild wrap cache if width changed or cache was invalidated
    if app.wrapped_width != w {
        app.rebuild_wrapped_tracks(w);
    }

    // Non-selected rows are always 1 pixel row; selected row expands to full height
    let sel = app.track_list_index;
    let row_heights: Vec<usize> = app
        .wrapped_tracks
        .iter()
        .enumerate()
        .map(|(i, (_, expanded))| if i == sel { expanded.len() } else { 1 })
        .collect();

    // Scroll: keep selection visible, counting pixel rows
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

    // Render
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

        // Non-selected: show truncated single line. Selected: show all wrapped lines.
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
                let cfg = if is_selected && active {
                    c.accent
                } else {
                    c.dim
                };
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

    // Fill remaining space
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

    // Count indicator
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

fn render_queue(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.colors;
    let active = app.active_panel == Panel::Queue;
    use crate::keybindings::Action;
    let jump_key = app.keybindings.keys_for_action(Action::JumpToQueue);
    let title = format!("Queue [{}] ({} tracks)", jump_key, app.player.queue.len());
    let block = panel_block(&title, active, app);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.player.queue.is_empty() {
        use crate::keybindings::Action;
        let add_key = app.keybindings.keys_for_action(Action::AddToQueue);
        let add_all_key = app.keybindings.keys_for_action(Action::AddAllToQueue);
        let empty = Paragraph::new(format!("No tracks in queue\n\nPress [{}] on a track\nor [{}] to add all", add_key, add_all_key))
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

            let title_str = truncate(track.display_title(), w.saturating_sub(2));
            let line = format!("{}{}", icon, title_str);

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

            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.queue_index));

    let list = List::new(items)
        .style(Style::default().bg(c.background))
        .highlight_style(Style::default().bg(c.selection_bg));

    f.render_stateful_widget(list, inner, &mut state);
}

fn render_player(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.colors;

    // Outer block
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", "Now Playing"),
            Style::default()
                .fg(c.dim)
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c.accent2))
        .style(Style::default().bg(c.background));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // ── Horizontal split: [art | info | progress] ────────────────────────────
    // Art is 10 chars wide + 4 for centering padding; info takes ~50%; progress takes the rest.
    let art_area_w = 9u16; // 10 block cols
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(art_area_w),  // album art
            Constraint::Percentage(50),       // now playing info
            Constraint::Min(10),              // progress + controls
        ])
        .split(inner);

    // ── Album art ─────────────────────────────────────────────────────────────
    let art_area = cols[0];
    // Center the art within the available area (both horizontally and vertically)
    let art_w = 10u16;
    let art_h = 3u16;
    let h_pad = (art_area.width.saturating_sub(art_w)) / 2;
    let v_pad = (art_area.height.saturating_sub(art_h)) / 2;
    let art_rect = Rect {
        x: art_area.x + h_pad,
        y: art_area.y + v_pad,
        width: art_w,
        height: art_h.min(art_area.height),
    };

    if art_rect.height > 0 {
        // Always use block art for the player bar
        let art = app.album_art.as_ref().cloned().unwrap_or_else(|| {
            crate::art::BlockArt::placeholder(8, art_h, c.art_border, c.art_bg)
        });

        for (i, row) in art.rows.iter().enumerate().take(art_rect.height as usize) {
            let row_rect = Rect {
                x: art_rect.x,
                y: art_rect.y + i as u16,
                width: art_rect.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(row.clone()).style(Style::default().bg(c.background)),
                row_rect,
            );
        }
    }

    // ── Now playing info ──────────────────────────────────────────────────────
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(cols[1]);

    if let Some(track) = &app.player.current_track {
        let state_icon = match app.player.state {
            PlayerState::Playing => "▶",
            PlayerState::Paused => "⏸",
            PlayerState::Stopped => "■",
        };

        let title_line = Line::from(vec![
            Span::styled(format!("{} ", state_icon), Style::default().fg(c.playing)),
            Span::styled(
                truncate(track.display_title(), cols[1].width as usize - 4),
                Style::default()
                    .fg(c.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let artist_line = Line::from(Span::styled(
            format!("  {} — {}", track.display_artist(), track.display_album()),
            Style::default().fg(c.accent),
        ));

        let meta_line = Line::from(Span::styled(
            format!(
                "  {} {}",
                if track.year > 0 {
                    track.year.to_string()
                } else {
                    "Unknown Year".to_string()
                },
                if !track.genre.is_empty() {
                    format!("· {}", track.genre.split(',').next().unwrap_or("").trim())
                } else {
                    "· Unknown Genre".to_string() 
                }
            ),
            Style::default().fg(c.dim),
        ));

        f.render_widget(Paragraph::new(title_line), left_rows[0]);
        f.render_widget(Paragraph::new(artist_line), left_rows[1]);
        f.render_widget(Paragraph::new(meta_line), left_rows[2]);
    } else {
        let idle = Paragraph::new("■ lyre — no track playing").style(Style::default().fg(c.dim));
        f.render_widget(idle, left_rows[0]);

        use crate::keybindings::Action;
        let play_key = app.keybindings.keys_for_action(Action::PlayPause);
        let help_key = app.keybindings.keys_for_action(Action::ToggleHelp);
        let hint = Paragraph::new(format!("  Press [{}] to play · [{}] for help", play_key, help_key))
            .style(Style::default().fg(c.dim));
        f.render_widget(hint, left_rows[1]);
    }

    // ── Progress bar + time ───────────────────────────────────────────────────
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(cols[2]);

    let elapsed = app.player.elapsed_secs();
    let total = app
        .player
        .current_track
        .as_ref()
        .map(|t| t.duration)
        .unwrap_or(0);
    let progress = app.player.progress();

    use crate::keybindings::Action;
    let status = Span::styled(
        format!(
            "{}:play/pause  {}:next  {}:prev  {}:shuffle  {}:loop  {}:help",
            app.keybindings.keys_for_action(Action::PlayPause),
            app.keybindings.keys_for_action(Action::Next),
            app.keybindings.keys_for_action(Action::Previous),
            app.keybindings.keys_for_action(Action::ToggleShuffle),
            app.keybindings.keys_for_action(Action::ToggleLoop),
            app.keybindings.keys_for_action(Action::ToggleHelp),
        ),
        Style::default().fg(c.dim),
    );
    f.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        right_rows[0],
    );

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(c.accent).bg(c.gauge_bg))
        .ratio(progress)
        .label("");
    f.render_widget(gauge, right_rows[1]);

    // Time centered across the full width
    let time_line = Line::from(vec![
        Span::styled(format_duration(elapsed), Style::default().fg(c.foreground)),
        Span::styled(" / ", Style::default().fg(c.dim)),
        Span::styled(format_duration(total), Style::default().fg(c.dim)),
    ]);
    f.render_widget(
        Paragraph::new(time_line).alignment(Alignment::Center),
        right_rows[2],
    );

    // Shuffle and loop indicators on the right (rendered on top)
    let shuffle_indicator = if app.player.shuffle { "⤮" } else { "⇉" };
    let shuffle_color = if app.player.shuffle { Style::default().fg(c.playing) } else { Style::default().fg(c.dim) };
    let loop_indicator = format!("{}", app.player.loop_mode.icon());
    let loop_color = if app.player.loop_mode == LoopMode::Off { Style::default().fg(c.dim) } else { Style::default().fg(c.playing) };
    let controls_line = Line::from(vec![
        Span::styled(format!("vol {:.0}%  ", app.player.volume * 100.0), Style::default().fg(c.dim)),
        Span::styled(shuffle_indicator, shuffle_color),
        Span::styled("  ", Style::default()),
        Span::styled(loop_indicator, loop_color),
    ]);
    f.render_widget(
        Paragraph::new(controls_line).alignment(Alignment::Left),
        right_rows[2],
    )
}

fn render_help(f: &mut Frame, area: Rect, app: &mut App) {
    let c = &app.colors;
    let w = 90u16.min(area.width);
    let h = area.height.saturating_sub(4);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            " ♫ Lyre — Keybindings ",
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(c.accent))
        .style(Style::default().bg(c.overlay_bg));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Split into content and footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let content_area = layout[0];
    let footer_area = layout[1];

    // Get sections from keybindings
    let sections = app.keybindings.help_sections();

    // Build all lines
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

    // Apply scrolling
    let visible_height = content_area.height as usize;
    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_offset = app.help_scroll.min(max_scroll);

    // Write back clamped value to prevent unbounded scrolling
    app.help_scroll = scroll_offset;

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let para = Paragraph::new(visible_lines).style(Style::default().bg(c.overlay_bg));
    f.render_widget(para, content_area);

    // Show scroll indicator
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

fn render_new_playlist_overlay(f: &mut Frame, area: Rect, app: &App, name: &str) {
    let c = &app.colors;
    let w = 50u16.min(area.width);
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            " New Playlist ",
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c.accent))
        .style(Style::default().bg(c.overlay_bg));

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
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
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

fn render_add_to_playlist_overlay(f: &mut Frame, area: Rect, app: &App, selected: usize, track_count: usize) {
    let c = &app.colors;
    let w = 50u16.min(area.width);
    let h = (app.playlists.len() as u16 + 4).min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect {
        x,
        y,
        width: w,
        height: h,
    };

    f.render_widget(Clear, popup);

    let title = if track_count == 1 {
        " Add to Playlist ".to_string()
    } else {
        format!(" Add {} tracks to Playlist ", track_count)
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(c.highlight)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(c.accent))
        .style(Style::default().bg(c.overlay_bg));

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
                Style::default()
                    .fg(c.highlight)
                    .bg(c.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(c.foreground)
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
            Style::default().fg(c.dim),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );
}

fn render_lyrics(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::types::Lyrics;
    let c = &app.colors;
    let active = app.active_panel == Panel::Lyrics;

    use crate::keybindings::Action;
    let toggle_key = app.keybindings.keys_for_action(Action::ToggleLyrics);
    let title = format!("Lyrics [{}]", toggle_key);
    let block = panel_block(&title, active, app);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lyrics = match &app.lyrics_content {
        Some(l) => l,
        None => {
            use crate::keybindings::Action;
            let lyrics_key = app.keybindings.keys_for_action(Action::ToggleLyrics);
            let cycle_key = app.keybindings.keys_for_action(Action::CycleForward);
            let placeholder = Paragraph::new(format!("Press '{}' or {} to view lyrics for the current track", lyrics_key, cycle_key))
                .style(Style::default().fg(c.dim))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(placeholder, inner);
            return;
        }
    };

    match lyrics {
        Lyrics::Plain(text) => {
            // Plain text lyrics (no timestamps)
            let lines: Vec<&str> = text.lines().collect();
            let visible_height = inner.height as usize;
            let max_scroll = lines.len().saturating_sub(visible_height);
            let scroll_offset = app.lyrics_scroll.min(max_scroll);

            // Write back clamped value to prevent unbounded scrolling
            app.lyrics_scroll = scroll_offset;

            let visible_lines: Vec<Line> = lines
                .iter()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|line| Line::from(Span::styled(*line, Style::default().fg(c.foreground))))
                .collect();

            let paragraph = Paragraph::new(visible_lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(c.background));

            f.render_widget(paragraph, inner);
        }
        Lyrics::Timed(lyric_lines) => {
            // Timestamped lyrics with highlighting
            let current_index = app.current_lyric_index();
            let visible_height = inner.height as usize;

            // Auto-scroll to keep current lyric centered
            if let Some(current) = current_index {
                let target_scroll = current.saturating_sub(visible_height / 2);
                let max_scroll = lyric_lines.len().saturating_sub(visible_height);
                app.lyrics_scroll = target_scroll.min(max_scroll);
            }

            let scroll_offset = app.lyrics_scroll.min(lyric_lines.len().saturating_sub(visible_height));

            let visible_lines: Vec<Line> = lyric_lines
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|(i, lyric_line)| {
                    let is_current = current_index == Some(i);
                    let style = if is_current {
                        Style::default().fg(c.highlight).bold()
                    } else {
                        Style::default().fg(c.dim)
                    };
                    Line::from(Span::styled(lyric_line.text.clone(), style))
                })
                .collect();

            let paragraph = Paragraph::new(visible_lines)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(c.background));

            f.render_widget(paragraph, inner);
        }
    }
}

// ── Utility ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    if max == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut w = 0usize;
    let chars: Vec<char> = s.chars().collect();
    for (_i, &ch) in chars.iter().enumerate() {
        let cw = ch.width().unwrap_or(1);
        if w + cw > max {
            // Truncate — add ellipsis if there's room
            if max >= 1 {
                // Remove last char(s) if needed to fit the ellipsis
                while w > max - 1 {
                    if let Some(last) = result.pop() {
                        w -= last.width().unwrap_or(1);
                    } else {
                        break;
                    }
                }
                result.push('…');
            }
            // Pad to max display width
            let current_w: usize = result.chars().map(|c| c.width().unwrap_or(1)).sum();
            if current_w < max {
                result.push_str(&" ".repeat(max - current_w));
            }
            return result;
        }
        result.push(ch);
        w += cw;
    }
    // String fit — pad to max
    if w < max {
        result.push_str(&" ".repeat(max - w));
    }
    result
}
