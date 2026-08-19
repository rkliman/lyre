use ratatui::style::Color;
use crate::config::UiColorsConfig;

/// Default color values for UI theme
pub mod default_colors {
    pub const FOREGROUND: &str = "#dcd7cd";
    pub const BACKGROUND: &str = "#12100e";
    pub const ACCENT: &str = "#d6a362";
    pub const ACCENT2: &str = "#b27844";
    pub const DIM: &str = "#645c50";
    pub const HIGHLIGHT: &str = "#f0c378";
    pub const PLAYING: &str = "#82c88c";
    pub const HEADER_BG: &str = "#1e1a16";
    pub const SELECTION_BG: &str = "#2d261c";
    pub const OVERLAY_BG: &str = "#16120e";
    pub const GAUGE_BG: &str = "#28231c";
    pub const ART_BG: &str = "#12100e";
    pub const ART_BORDER: &str = "#28231c";
}

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub foreground: Color,
    pub background: Color,
    pub accent: Color,
    pub accent2: Color,
    pub dim: Color,
    pub highlight: Color,
    pub playing: Color,
    pub header_bg: Color,
    pub selection_bg: Color,
    pub overlay_bg: Color,
    pub gauge_bg: Color,
    pub art_bg: Color,
    pub art_border: Color,
}

fn parse_color(color_str: &str) -> Color {
    let trimmed = color_str.trim();

    // Try hex format: #RRGGBB
    if trimmed.starts_with('#') && trimmed.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&trimmed[1..3], 16),
            u8::from_str_radix(&trimmed[3..5], 16),
            u8::from_str_radix(&trimmed[5..7], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }

    // Try comma-separated RGB: "R, G, B"
    if trimmed.contains(',') {
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].trim().parse::<u8>(),
                parts[1].trim().parse::<u8>(),
                parts[2].trim().parse::<u8>(),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
    }

    // Fallback to white
    Color::Rgb(255, 255, 255)
}

impl ColorScheme {
    pub fn from_config(colors: &UiColorsConfig) -> Self {
        Self {
            foreground: parse_color(&colors.foreground),
            background: parse_color(&colors.background),
            accent: parse_color(&colors.accent),
            accent2: parse_color(&colors.accent2),
            dim: parse_color(&colors.dim),
            highlight: parse_color(&colors.highlight),
            playing: parse_color(&colors.playing),
            header_bg: parse_color(&colors.header_bg),
            selection_bg: parse_color(&colors.selection_bg),
            overlay_bg: parse_color(&colors.overlay_bg),
            gauge_bg: parse_color(&colors.gauge_bg),
            art_bg: parse_color(&colors.art_bg),
            art_border: parse_color(&colors.art_border),
        }
    }

    /// Style for normal text
    pub fn normal_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.foreground)
    }

    /// Style for dimmed/secondary text
    pub fn dim_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.dim)
    }

    /// Style for accent text
    pub fn accent_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent)
    }

    /// Style for bold accent text (headers, titles)
    pub fn accent_bold_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.accent)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn header_style(&self) -> ratatui::style::Style {
        self.accent_bold_style().bg(self.header_bg)
    }

    /// Style for highlighted text
    pub fn highlight_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.highlight)
    }

    /// Style for bold highlighted text
    pub fn highlight_bold_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.highlight)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    /// Style for background blocks
    pub fn block_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().bg(self.background)
    }

    /// Style for selected items
    pub fn selected_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.highlight)
            .bg(self.selection_bg)
            .add_modifier(ratatui::style::Modifier::BOLD)
    }

    /// Style for selected items (with accent2 foreground)
    pub fn selected_accent2_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default()
            .fg(self.accent2)
            .bg(self.selection_bg)
    }

    /// Style for borders
    pub fn border_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent2)
    }

    /// Style for active borders
    pub fn border_active_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.accent)
    }

    /// Style for inactive borders
    pub fn border_inactive_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.dim)
    }
}