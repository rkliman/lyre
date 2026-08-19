use ratatui::{
    layout::{Alignment, Rect},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub(super) fn render_art_window(f: &mut Frame, app: &mut App, area: Rect) {
    let c = &app.colors;
    use crate::keybindings::Action;
    let toggle_key = app.keybindings.keys_for_action(Action::ToggleArtWindow);
    let block = Block::default()
        .title(Span::styled(
            format!(" Album Art [{}] ", toggle_key),
            c.accent_bold_style(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(c.border_style())
        .style(c.block_style());

    let inner = block.inner(area);
    let background = c.background;
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if let Some(img) = app.art.image.as_ref() {
        app.art.window_cache.render(f, inner, img, app.art.picker.as_mut(), background);
    } else {
        let placeholder = Paragraph::new(format!("No album art available\n\nPress {} to hide", toggle_key))
            .style(c.dim_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(placeholder, inner);
    }
}
