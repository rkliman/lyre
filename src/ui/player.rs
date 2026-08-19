use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{format_duration, LoopMode, PlayerState};
use crate::util::truncate_field;

pub(super) fn render_player(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.colors;

    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", "🎶 Now Playing"),
            c.dim_style(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(c.border_style())
        .style(c.block_style());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let art_area_w = 9u16;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(art_area_w),
            Constraint::Percentage(50),
            Constraint::Min(10),
        ])
        .split(inner);

    let art_area = cols[0];
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
        let art = app.art.player_bar.as_ref().cloned().unwrap_or_else(|| {
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
                Paragraph::new(row.clone()).style(c.block_style()),
                row_rect,
            );
        }
    }

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
                truncate_field(track.display_title(), cols[1].width as usize - 4),
                c.highlight_bold_style(),
            ),
        ]);

        let artist_line = Line::from(Span::styled(
            format!("  {} — {}", track.display_artist(), track.display_album()),
            c.accent_style(),
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
            c.dim_style(),
        ));

        f.render_widget(Paragraph::new(title_line), left_rows[0]);
        f.render_widget(Paragraph::new(artist_line), left_rows[1]);
        f.render_widget(Paragraph::new(meta_line), left_rows[2]);
    } else {
        let idle = Paragraph::new("■ lyre — no track playing").style(c.dim_style());
        f.render_widget(idle, left_rows[0]);

        use crate::keybindings::Action;
        let play_key = app.keybindings.keys_for_action(Action::PlayPause);
        let help_key = app.keybindings.keys_for_action(Action::ToggleHelp);
        let hint1 = Paragraph::new(format!("  Press [{}] to play", play_key))
            .style(c.dim_style());
        f.render_widget(hint1, left_rows[1]);
        let hint2 = Paragraph::new(format!("  Press [{}] for help", help_key))
            .style(c.dim_style());
        f.render_widget(hint2, left_rows[2]);
    }

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
            "[{}] play/pause  [{}] next  [{}] prev  [{}] shuffle  [{}] loop",
            app.keybindings.keys_for_action(Action::PlayPause),
            app.keybindings.keys_for_action(Action::Next),
            app.keybindings.keys_for_action(Action::Previous),
            app.keybindings.keys_for_action(Action::ToggleShuffle),
            app.keybindings.keys_for_action(Action::ToggleLoop),
        ),
        c.dim_style(),
    );
    f.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        right_rows[2],
    );

    let gauge = Gauge::default()
        .gauge_style(c.accent_style().bg(c.gauge_bg))
        .ratio(progress)
        .label("");
    f.render_widget(gauge, right_rows[0]);

    let time_line = Line::from(vec![
        Span::styled(format_duration(elapsed), c.normal_style()),
        Span::styled(" / ", c.dim_style()),
        Span::styled(format_duration(total), c.dim_style()),
    ]);
    f.render_widget(
        Paragraph::new(time_line).alignment(Alignment::Center),
        right_rows[1],
    );

    let shuffle_indicator = if app.player.shuffle { "⤮" } else { "⇉" };
    let shuffle_color = if app.player.shuffle {
        Style::default().fg(c.playing)
    } else {
        c.dim_style()
    };
    let loop_indicator = format!("{}", app.player.loop_mode.icon());
    let loop_color = if app.player.loop_mode == LoopMode::Off {
        c.dim_style()
    } else {
        Style::default().fg(c.playing)
    };
    let controls_line = Line::from(vec![
        Span::styled(
            format!("vol {:.0}%  ", app.player.volume * 100.0),
            c.dim_style(),
        ),
        Span::styled(shuffle_indicator, shuffle_color),
        Span::styled("  ", Style::default()),
        Span::styled(loop_indicator, loop_color),
    ]);
    f.render_widget(
        Paragraph::new(controls_line).alignment(Alignment::Left),
        right_rows[1],
    )
}
