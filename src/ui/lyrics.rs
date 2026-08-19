use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types::{Lyrics, LyricsFetchStatus, Panel};

use super::panel_block;

pub(super) fn render_lyrics(f: &mut Frame, app: &mut App, area: Rect) {
    let c = &app.colors;
    let active = app.active_panel == Panel::Lyrics;

    use crate::keybindings::Action;
    let toggle_key = app.keybindings.keys_for_action(Action::ToggleLyrics);
    let title = format!("Lyrics [{}]", toggle_key);
    let block = panel_block(&title, active, app);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lyrics = match &app.lyrics.content {
        Some(l) => l,
        None => {
            render_lyrics_placeholder(f, app, inner);
            return;
        }
    };

    match lyrics {
        Lyrics::Plain(text) => {
            let visible_height = inner.height as usize;
            let all_lines: Vec<Line> = text
                .lines()
                .map(|line| Line::from(Span::styled(line, c.normal_style())))
                .collect();

            let paragraph = Paragraph::new(all_lines)
                .wrap(Wrap { trim: false })
                .style(app.colors.block_style());

            let max_scroll = paragraph.line_count(inner.width).saturating_sub(visible_height);
            app.lyrics.scroll = app.lyrics.scroll.min(max_scroll);

            let paragraph = paragraph.scroll((app.lyrics.scroll as u16, 0));
            f.render_widget(paragraph, inner);
        }
        Lyrics::Timed(lyric_lines) => {
            let current_index = app.current_lyric_index();
            let visible_height = inner.height as usize;

            let all_lines: Vec<Line> = lyric_lines
                .iter()
                .enumerate()
                .map(|(i, lyric_line)| {
                    let is_current = current_index == Some(i);
                    let style = if is_current {
                        app.colors.selected_style()
                    } else {
                        app.colors.dim_style()
                    };
                    Line::from(Span::styled(lyric_line.text.clone(), style))
                })
                .collect();

            let paragraph = Paragraph::new(all_lines)
                .wrap(Wrap { trim: false })
                .style(app.colors.block_style());

            let total_rows = paragraph.line_count(inner.width);
            let max_scroll = total_rows.saturating_sub(visible_height);

            if !app.lyrics.manual_scroll {
                if let Some(current) = current_index {
                    let visual_row: usize = lyric_lines
                        .iter()
                        .take(current)
                        .map(|ll| {
                            Paragraph::new(ll.text.as_str())
                                .wrap(Wrap { trim: false })
                                .line_count(inner.width)
                        })
                        .sum();
                    app.lyrics.scroll = visual_row.saturating_sub(visible_height / 2).min(max_scroll);
                }
            }

            app.lyrics.scroll = app.lyrics.scroll.min(max_scroll);

            let paragraph = paragraph.scroll((app.lyrics.scroll as u16, 0));
            f.render_widget(paragraph, inner);
        }
    }
}

fn render_lyrics_placeholder(f: &mut Frame, app: &App, inner: Rect) {
    let dim = app.colors.dim_style();
    let fg = app.colors.normal_style();
    let accent = app.colors.selected_style().add_modifier(Modifier::BOLD);

    // No track playing → simple message
    if app.player.current_track.is_none() {
        let para = Paragraph::new("No track is currently playing.")
            .style(dim)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(para, center_vertically(inner, 1));
        return;
    }

    let lines: Vec<Line> = match &app.lyrics.fetch_status {
        LyricsFetchStatus::Fetching => vec![
            Line::from(Span::styled("Fetching lyrics from lrclib…", fg)),
        ],
        LyricsFetchStatus::NotFound => vec![
            Line::from(Span::styled("No lyrics detected", fg)),
            Line::from(""),
            Line::from(Span::styled("No lyrics found on lrclib for this track.", dim)),
        ],
        LyricsFetchStatus::Error(e) => vec![
            Line::from(Span::styled("No lyrics detected", fg)),
            Line::from(""),
            Line::from(Span::styled(format!("Error fetching lyrics: {}", e), dim)),
            Line::from(""),
            Line::from(Span::styled("Press [Enter] to retry.", dim)),
        ],
        LyricsFetchStatus::Idle => vec![
            Line::from(Span::styled("No lyrics detected", fg)),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", dim),
                Span::styled("[Enter]", accent),
                Span::styled(" attempt to fetch.", dim),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Note: lyrics may not be found for all songs",
                dim,
            )),
        ],
    };

    let height = lines.len() as u16;
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(para, center_vertically(inner, height));
}

fn center_vertically(area: Rect, content_height: u16) -> Rect {
    let h = content_height.min(area.height);
    let top = area.y + area.height.saturating_sub(h) / 2;
    Rect {
        x: area.x,
        y: top,
        width: area.width,
        height: h,
    }
}
