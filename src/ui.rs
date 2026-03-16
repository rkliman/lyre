use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use crate::app::App;
use crate::types::{Overlay, Panel, PlayerState, SidebarItem, TrackContext};
use crate::types::format_duration;
use unicode_width::UnicodeWidthStr;

// ── Colour palette ────────────────────────────────────────────────────────────
const FG: Color = Color::Rgb(220, 215, 205);       // warm off-white
const BG: Color = Color::Rgb(18, 16, 14);          // near-black
const ACCENT: Color = Color::Rgb(214, 163, 98);    // warm gold
const ACCENT2: Color = Color::Rgb(178, 120, 68);   // darker gold
const DIM: Color = Color::Rgb(100, 92, 80);        // muted
const HIGHLIGHT: Color = Color::Rgb(240, 195, 120);// bright gold highlight
const PLAYING: Color = Color::Rgb(130, 200, 140);  // soft green for playing indicator
const HEADER_BG: Color = Color::Rgb(30, 26, 22);   // slightly lighter bg for headers
const SEL_BG: Color = Color::Rgb(45, 38, 28);      // selection background

fn render_banner(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT2))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner into left (lyre glyph + title) and right (author + version)
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(36)])
        .split(inner);

    // ── Left: glyph + app name ────────────────────────────────────────────────
    // The lyre glyph (𝄞 is a musical symbol — U+1D11E treble clef,
    // but or the lyre emoji reads better in most terminals)
    let left = Line::from(vec![
        Span::styled(" lyre", Style::default()
            .fg(HIGHLIGHT)
            .add_modifier(Modifier::BOLD)),
        Span::styled("  ", Style::default()),
        Span::styled("─── ", Style::default().fg(DIM)),
        Span::styled("a music library & player. press '?' for help.",
            Style::default().fg(DIM)),
    ]);

    // ── Right: author + version ───────────────────────────────────────────────
    let right = Line::from(vec![
        Span::styled("Randall Kliman", Style::default().fg(ACCENT)),
        Span::styled("  ·  ", Style::default().fg(DIM)),
        Span::styled("v0.1.0", Style::default().fg(DIM)),
        Span::styled("  ", Style::default()),
    ]);

    f.render_widget(
        Paragraph::new(left).style(Style::default().bg(BG)),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .style(Style::default().bg(BG)),
        cols[1],
    );
}

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Top-level layout: banner + body + player bar ─────────────────────────
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // banner
            Constraint::Min(1),     // body
            Constraint::Length(5),  // player bar
        ])
        .split(area);

    let banner_area = root[0];
    let body_area   = root[1];
    let player_area = root[2];

    // ── Body: sidebar | tracklist | queue ─────────────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),  // sidebar
            Constraint::Min(10),         // track list
            Constraint::Percentage(28),  // queue
        ])
        .split(body_area);

    render_banner(f, banner_area);
    render_sidebar(f, app, body[0]);
    render_tracklist(f, app, body[1]);
    render_queue(f, app, body[2]);
    render_player(f, app, player_area);

    if app.show_help {
        render_help(f, area);
    }

    // Overlays render on top of everything
    match &app.overlay {
        Overlay::NewPlaylist(name) => render_new_playlist_overlay(f, area, name),
        Overlay::AddToPlaylist { selected, .. } => render_add_to_playlist_overlay(f, area, app, *selected),
        Overlay::None => {}
    }
}

fn panel_block<'a>(title: &'a str, active: bool) -> Block<'a> {
    let border_style = if active {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(DIM)
    };

    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(if active { HIGHLIGHT } else { DIM })
                .add_modifier(if active { Modifier::BOLD } else { Modifier::empty() }),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(BG))
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let active = app.active_panel == Panel::Sidebar;
    let block = panel_block("Library [1]", active);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .sidebar_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.sidebar_index;

            let style = if item.is_header() {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else if is_selected && active {
                Style::default().fg(HIGHLIGHT).bg(SEL_BG).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(ACCENT2).bg(SEL_BG)
            } else {
                Style::default().fg(FG)
            };

            // For headers, prepend an expand/collapse chevron
            let label = if item.is_header() {
                let section_key = match item {
                    SidebarItem::Artists   => "Artists",
                    SidebarItem::Albums    => "Albums",
                    SidebarItem::Genres    => "Genres",
                    SidebarItem::Playlists => "Playlists",
                    _ => "",
                };
                let expanded = *app.sidebar_expanded.get(section_key).unwrap_or(&true);
                let chevron = if expanded { "▼ " } else { "▶ " };
                format!("{}{}", chevron, item.label().trim_start())
            } else {
                item.label()
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.sidebar_index));

    let list = List::new(items)
        .style(Style::default().bg(BG))
        .highlight_style(Style::default().bg(SEL_BG));

    f.render_stateful_widget(list, inner, &mut state);
}

fn render_tracklist(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.active_panel == Panel::TrackList;

    // Build title — show playlist name or sort info
    let title_str = match &app.track_context {
        TrackContext::Playlist(name) => {
            format!(" ♪ {} [playlist]  K/J:move  x:remove  P:add-to ", name)
        }
        TrackContext::Library => {
            format!(
                " Tracks [2] — {} {} ",
                app.sort_field.label(),
                if app.sort_order == crate::types::SortOrder::Asc { "↑" } else { "↓" }
            )
        }
    };
    let block = panel_block(&title_str, active)
        .title_alignment(Alignment::Left);

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
    let list_area   = inner_rows[2];

    // ── Search box ────────────────────────────────────────────────────────────
    let search_active = app.search_mode;
    let has_query     = !app.search_query.is_empty();

    let box_bg             = if search_active { Color::Rgb(35, 28, 18) } else { Color::Rgb(22, 19, 15) };
    let box_border_color   = if search_active { ACCENT } else if has_query { ACCENT2 } else { DIM };

    let result_hint = if has_query {
        format!("  {} result{}", app.filtered_tracks.len(), if app.filtered_tracks.len() == 1 { "" } else { "s" })
    } else {
        String::new()
    };

    let box_title = if search_active {
        Span::styled(" search ", Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD))
    } else if has_query {
        Span::styled(format!(" search{} ", result_hint), Style::default().fg(ACCENT))
    } else {
        Span::styled(" search ", Style::default().fg(DIM))
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
                Style::default().fg(HIGHLIGHT).bg(box_bg).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(ACCENT).bg(box_bg)),
        ])
    } else if has_query {
        Line::from(Span::styled(
            app.search_query.clone(),
            Style::default().fg(ACCENT2).bg(box_bg),
        ))
    } else {
        Line::from(Span::styled(
            "press / to search…",
            Style::default().fg(DIM).bg(box_bg),
        ))
    };

    f.render_widget(
        Paragraph::new(content).style(Style::default().bg(box_bg)),
        search_inner,
    );

    // Render column headers
    // Layout: [icon] Title  Artist  Album  Genre  Dur
    let w = inner.width as usize;
    let title_w  = w * 28 / 100;
    let artist_w = w * 22 / 100;
    let album_w  = w * 22 / 100;
    let genre_w  = w * 16 / 100;

    let header = format!(
        "{:<tw$}  {:<aw$}  {:<bw$}  {:<gw$}  {:>4}",
        "Title", "Artist", "Album", "Genre", "Dur",
        tw = title_w, aw = artist_w, bw = album_w, gw = genre_w
    );
    let header_widget = Paragraph::new(header)
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD).bg(HEADER_BG));
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
    let row_heights: Vec<usize> = app.wrapped_tracks.iter().enumerate()
        .map(|(i, (_, expanded))| if i == sel { expanded.len() } else { 1 })
        .collect();

    // Scroll: keep selection visible, counting pixel rows
    if !row_heights.is_empty() {
        let sel_clamped = sel.min(row_heights.len().saturating_sub(1));
        let sel_start: usize = row_heights[..sel_clamped].iter().sum();
        let sel_end = sel_start + row_heights[sel_clamped];
        let offset_rows: usize = row_heights[..app.track_list_offset.min(row_heights.len())].iter().sum();

        if sel_start < offset_rows {
            app.track_list_offset = sel_clamped;
        } else if sel_end > offset_rows + visible_height {
            let mut consumed = 0usize;
            let mut new_offset = sel_clamped;
            for idx in (0..=sel_clamped).rev() {
                consumed += row_heights[idx];
                if consumed >= visible_height { new_offset = idx + 1; break; }
                if idx == 0 { new_offset = 0; }
            }
            app.track_list_offset = new_offset;
        }
    }

    // Render
    let mut y = list_area.y;
    let bottom = list_area.y + list_area.height;

    for (i, (collapsed, expanded)) in app.wrapped_tracks.iter().enumerate().skip(app.track_list_offset) {
        if y >= bottom { break; }

        let track = &app.filtered_tracks[i];
        let is_selected = i == app.track_list_index;
        let is_playing  = playing_path.as_deref() == Some(&track.path);

        let row_bg = if is_selected { SEL_BG } else { BG };
        let fg = if is_selected && active { HIGHLIGHT }
                 else if is_selected      { ACCENT2 }
                 else if is_playing       { PLAYING }
                 else                     { FG };
        let bold = is_selected && active;

        let play_icon = if is_playing {
            match app.player.state {
                PlayerState::Playing => "▶",
                PlayerState::Paused  => "⏸",
                PlayerState::Stopped => " ",
            }
        } else { " " };

        // Non-selected: show truncated single line. Selected: show all wrapped lines.
        let render_lines: &[String] = if is_selected { expanded } else {
            std::slice::from_ref(collapsed)
        };

        for (row_idx, line_text) in render_lines.iter().enumerate() {
            if y >= bottom { break; }

            let (line_fg, line_bold) = if row_idx == 0 {
                (fg, bold)
            } else {
                let cfg = if is_selected && active { ACCENT } else { DIM };
                (cfg, false)
            };

            let text = if row_idx == 0 {
                format!("{}{}", play_icon, &line_text[1..])
            } else {
                line_text.clone()
            };

            let style = Style::default().fg(line_fg).bg(row_bg)
                .add_modifier(if line_bold { Modifier::BOLD } else { Modifier::empty() });

            f.render_widget(
                Paragraph::new(Line::from(Span::styled(text, style)))
                    .style(Style::default().bg(row_bg)),
                Rect { x: list_area.x, y, width: list_area.width, height: 1 },
            );
            y += 1;
        }
    }

    // Fill remaining space
    while y < bottom {
        f.render_widget(
            Paragraph::new("").style(Style::default().bg(BG)),
            Rect { x: list_area.x, y, width: list_area.width, height: 1 },
        );
        y += 1;
    }

    // Count indicator
    f.render_widget(
        Paragraph::new(format!(" {}/{} ", app.track_list_index + 1, app.filtered_tracks.len()))
            .style(Style::default().fg(DIM))
            .alignment(Alignment::Right),
        Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
    );
}

fn render_queue(f: &mut Frame, app: &App, area: Rect) {
    let active = app.active_panel == Panel::Queue;
    let title = format!("Queue [3] ({} tracks)", app.player.queue.len());
    let block = panel_block(&title, active);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.player.queue.is_empty() {
        let empty = Paragraph::new("No tracks in queue\n\nPress [a] on a track\nor [A] to add all")
            .style(Style::default().fg(DIM))
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
            let is_playing = i == app.player.queue_index
                && app.player.state != PlayerState::Stopped;
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
                Style::default().fg(HIGHLIGHT).bg(SEL_BG).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(ACCENT2).bg(SEL_BG)
            } else if is_playing {
                Style::default().fg(PLAYING).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };

            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.queue_index));

    let list = List::new(items)
        .style(Style::default().bg(BG))
        .highlight_style(Style::default().bg(SEL_BG));

    f.render_stateful_widget(list, inner, &mut state);
}

fn render_player(f: &mut Frame, app: &App, area: Rect) {
    // Outer block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT2))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // ── Horizontal split: [art | info | progress] ────────────────────────────
    // Art is 10 chars wide + 1 gap; info takes ~55%; progress takes the rest.
    let art_w = 11u16; // 10 block cols + 1 padding
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(art_w),   // album art
            Constraint::Percentage(50),  // now playing info
            Constraint::Min(10),         // progress + controls
        ])
        .split(inner);

    // ── Album art ─────────────────────────────────────────────────────────────
    let art_area = cols[0];
    // Vertically centre the 3-row art within the available height
    let art_h = 3u16;
    let v_pad = (art_area.height.saturating_sub(art_h)) / 2;
    let art_rect = Rect {
        x: art_area.x,
        y: art_area.y + v_pad,
        width: 10,
        height: art_h.min(art_area.height),
    };

    if art_rect.height > 0 {
        let art = app.album_art.as_ref().cloned()
            .unwrap_or_else(|| crate::art::BlockArt::placeholder(10, art_h));

        for (i, row) in art.rows.iter().enumerate().take(art_rect.height as usize) {
            let row_rect = Rect {
                x: art_rect.x,
                y: art_rect.y + i as u16,
                width: art_rect.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(row.clone()).style(Style::default().bg(BG)),
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
            PlayerState::Paused  => "⏸",
            PlayerState::Stopped => "■",
        };

        let title_line = Line::from(vec![
            Span::styled(format!("{} ", state_icon), Style::default().fg(PLAYING)),
            Span::styled(
                truncate(track.display_title(), cols[1].width as usize - 4),
                Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD),
            ),
        ]);

        let artist_line = Line::from(Span::styled(
            format!("  {} — {}", track.display_artist(), track.display_album()),
            Style::default().fg(ACCENT),
        ));

        let meta_line = Line::from(Span::styled(
            format!(
                "  {} {}  vol {:.0}%",
                if track.year > 0 { track.year.to_string() } else { String::new() },
                if !track.genre.is_empty() {
                    format!("· {}", track.genre.split(',').next().unwrap_or("").trim())
                } else {
                    String::new()
                },
                app.player.volume * 100.0
            ),
            Style::default().fg(DIM),
        ));

        f.render_widget(Paragraph::new(title_line), left_rows[0]);
        f.render_widget(Paragraph::new(artist_line), left_rows[1]);
        f.render_widget(Paragraph::new(meta_line), left_rows[2]);
    } else {
        let idle = Paragraph::new("■ lyre — no track playing")
            .style(Style::default().fg(DIM));
        f.render_widget(idle, left_rows[0]);

        let hint = Paragraph::new("  Press [Enter] to play · [?] for help")
            .style(Style::default().fg(DIM));
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

    let elapsed  = app.player.elapsed_secs();
    let total    = app.player.current_track.as_ref().map(|t| t.duration).unwrap_or(0);
    let progress = app.player.progress();

    let time_line = Line::from(vec![
        Span::styled(format_duration(elapsed), Style::default().fg(FG)),
        Span::styled(" / ", Style::default().fg(DIM)),
        Span::styled(format_duration(total), Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(time_line).alignment(Alignment::Center), right_rows[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(ACCENT).bg(Color::Rgb(40, 35, 28)))
        .ratio(progress)
        .label("");
    f.render_widget(gauge, right_rows[1]);

    let status = if let Some(msg) = &app.status_message {
        Span::styled(truncate(msg, cols[2].width as usize), Style::default().fg(DIM))
    } else {
        Span::styled(
            "spc:play/pause  n:next  p:prev  s:stop  S:sort  /:search",
            Style::default().fg(DIM),
        )
    };
    f.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        right_rows[2],
    );
}

fn render_help(f: &mut Frame, area: Rect) {
    let w = 66u16.min(area.width);
    let h = 36u16.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" ♫ Lyre — Keybindings ", Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(22, 18, 14)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("Navigation", vec![
            ("Tab / Shift-Tab",       "Cycle panels"),
            ("1 / 2 / 3",             "Jump to sidebar / tracks / queue"),
            ("j / k  or  ↑ / ↓",     "Move up / down"),
            ("g / G",                 "Go to top / bottom"),
            ("u / d  or  PgUp/PgDn", "Page up / down (all panels)"),
            ("h / l  or  ← / →",     "Switch panels"),
            ("→ / l  on section header", "Expand  (or focus tracks if open)"),
            ("← / h  on section header", "Collapse"),
        ]),
        ("Playback", vec![
            ("Enter  or  Space", "Play selected / toggle pause"),
            ("n",                "Next track in queue"),
            ("p",                "Previous track in queue"),
            ("s",                "Stop"),
            ("+ / -",            "Volume up / down"),
        ]),
        ("Queue", vec![
            ("a",       "Add selected track to queue"),
            ("A",       "Add all visible tracks to queue"),
            ("x / Del", "Remove selected from queue"),
            ("c",       "Clear entire queue"),
        ]),
        ("Library", vec![
            ("S",   "Cycle sort field (title→artist→album→year→genre→dur)"),
            ("R",   "Toggle sort order (asc / desc)"),
            ("/",   "Activate search bar (always visible above track list)"),
            ("Esc", "Deactivate search bar  (results stay if query non-empty)"),
        ]),
        ("Playlists", vec![
            ("N  on Playlists header", "Create a new empty playlist"),
            ("P  on any track",        "Add track to an existing playlist"),
            ("x / Del  in playlist",   "Remove track from playlist (saves immediately)"),
            ("K  in playlist",         "Move selected track up"),
            ("J  in playlist",         "Move selected track down"),
        ]),
        ("Other", vec![
            ("?", "Toggle this help overlay"),
            ("q", "Quit"),
        ]),
    ];

    let mut text_lines: Vec<Line> = Vec::new();
    for (section, keys) in &lines {
        text_lines.push(Line::from(Span::styled(
            *section,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in keys {
            text_lines.push(Line::from(vec![
                Span::styled(format!("  {:28}", key), Style::default().fg(FG)),
                Span::styled(*desc, Style::default().fg(DIM)),
            ]));
        }
        text_lines.push(Line::from(""));
    }

    let para = Paragraph::new(text_lines)
        .style(Style::default().bg(Color::Rgb(22, 18, 14)));
    f.render_widget(para, inner);
}

fn render_new_playlist_overlay(f: &mut Frame, area: Rect, name: &str) {
    let w = 50u16.min(area.width);
    let h = 5u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" New Playlist ", Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(22, 18, 14)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled("Playlist name:", Style::default().fg(DIM))),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{}█", name),
            Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD),
        )),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("Enter to confirm · Esc to cancel", Style::default().fg(DIM))),
        rows[2],
    );
}

fn render_add_to_playlist_overlay(f: &mut Frame, area: Rect, app: &App, selected: usize) {
    let w = 50u16.min(area.width);
    let h = (app.playlists.len() as u16 + 4).min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Add to Playlist ", Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Rgb(22, 18, 14)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app.playlists.iter().enumerate().map(|(i, pl)| {
        let style = if i == selected {
            Style::default().fg(HIGHLIGHT).bg(SEL_BG).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        let icon = if i == selected { "▶ " } else { "  " };
        ListItem::new(Line::from(Span::styled(format!("{}{}", icon, pl.name), style)))
    }).collect();

    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(items).style(Style::default().bg(Color::Rgb(22, 18, 14)));
    f.render_stateful_widget(list, rows[0], &mut state);

    f.render_widget(
        Paragraph::new(Span::styled("Enter to add · Esc to cancel", Style::default().fg(DIM)))
            .alignment(Alignment::Center),
        rows[1],
    );
}

// ── Utility ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    if max == 0 { return String::new(); }
    let mut result = String::new();
    let mut w = 0usize;
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        let cw = ch.width().unwrap_or(1);
        if w + cw > max {
            // Truncate — add ellipsis if there's room
            if max >= 1 {
                // Remove last char(s) if needed to fit the ellipsis
                while w > max - 1 {
                    if let Some(last) = result.pop() {
                        w -= last.width().unwrap_or(1);
                    } else { break; }
                }
                result.push('…');
            }
            // Pad to max display width
            let current_w: usize = result.chars().map(|c| c.width().unwrap_or(1)).sum();
            if current_w < max { result.push_str(&" ".repeat(max - current_w)); }
            return result;
        }
        result.push(ch);
        w += cw;
    }
    // String fit — pad to max
    if w < max { result.push_str(&" ".repeat(max - w)); }
    result
}