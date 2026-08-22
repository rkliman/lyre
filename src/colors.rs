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

struct PresetColors {
    foreground: &'static str,
    background: &'static str,
    accent: &'static str,
    accent2: &'static str,
    dim: &'static str,
    highlight: &'static str,
    playing: &'static str,
    header_bg: &'static str,
    selection_bg: &'static str,
    overlay_bg: &'static str,
    gauge_bg: &'static str,
    art_bg: &'static str,
    art_border: &'static str,
}

// Catppuccin — https://catppuccin.com/palette
const CATPPUCCIN_MOCHA: PresetColors = PresetColors {
    foreground:   "#cdd6f4", // text
    background:   "#1e1e2e", // base
    accent:       "#cba6f7", // mauve
    accent2:      "#b4befe", // lavender
    dim:          "#7f849c", // overlay1
    highlight:    "#f5c2e7", // pink
    playing:      "#a6e3a1", // green
    header_bg:    "#181825", // mantle
    selection_bg: "#313244", // surface0
    overlay_bg:   "#11111b", // crust
    gauge_bg:     "#45475a", // surface1
    art_bg:       "#1e1e2e", // base
    art_border:   "#45475a", // surface1
};

const CATPPUCCIN_LATTE: PresetColors = PresetColors {
    foreground:   "#4c4f69", // text
    background:   "#eff1f5", // base
    accent:       "#8839ef", // mauve
    accent2:      "#7287fd", // lavender
    dim:          "#8c8fa1", // overlay1
    highlight:    "#fe640b", // peach
    playing:      "#40a02b", // green
    header_bg:    "#e6e9ef", // mantle
    selection_bg: "#ccd0da", // surface0
    overlay_bg:   "#dce0e8", // crust
    gauge_bg:     "#bcc0cc", // surface1
    art_bg:       "#eff1f5", // base
    art_border:   "#bcc0cc", // surface1
};

const CATPPUCCIN_FRAPPE: PresetColors = PresetColors {
    foreground:   "#c6d0f5", // text
    background:   "#303446", // base
    accent:       "#ca9ee6", // mauve
    accent2:      "#babbf1", // lavender
    dim:          "#838ba7", // overlay1
    highlight:    "#f4b8e4", // pink
    playing:      "#a6d189", // green
    header_bg:    "#292c3c", // mantle
    selection_bg: "#414559", // surface0
    overlay_bg:   "#232634", // crust
    gauge_bg:     "#51576d", // surface1
    art_bg:       "#303446", // base
    art_border:   "#51576d", // surface1
};

const CATPPUCCIN_MACCHIATO: PresetColors = PresetColors {
    foreground:   "#cad3f5", // text
    background:   "#24273a", // base
    accent:       "#c6a0f6", // mauve
    accent2:      "#b7bdf8", // lavender
    dim:          "#8087a2", // overlay1
    highlight:    "#f5bde6", // pink
    playing:      "#a6da95", // green
    header_bg:    "#1e2030", // mantle
    selection_bg: "#363a4f", // surface0
    overlay_bg:   "#181926", // crust
    gauge_bg:     "#494d64", // surface1
    art_bg:       "#24273a", // base
    art_border:   "#494d64", // surface1
};

// Nord — https://nordtheme.com/docs/colors-and-palettes
const NORD: PresetColors = PresetColors {
    foreground:   "#d8dee9", // nord4
    background:   "#2e3440", // nord0
    accent:       "#88c0d0", // nord8 — bright frost
    accent2:      "#81a1c1", // nord9
    dim:          "#4c566a", // nord3
    highlight:    "#ebcb8b", // nord13 — yellow
    playing:      "#a3be8c", // nord14 — green
    header_bg:    "#3b4252", // nord1
    selection_bg: "#434c5e", // nord2
    overlay_bg:   "#242933", // slightly darker than nord0
    gauge_bg:     "#3b4252", // nord1
    art_bg:       "#2e3440", // nord0
    art_border:   "#434c5e", // nord2
};

// Dracula — https://draculatheme.com/contribute
const DRACULA: PresetColors = PresetColors {
    foreground:   "#f8f8f2",
    background:   "#282a36",
    accent:       "#bd93f9", // purple
    accent2:      "#ff79c6", // pink
    dim:          "#6272a4", // comment
    highlight:    "#f1fa8c", // yellow
    playing:      "#50fa7b", // green
    header_bg:    "#44475a", // selection
    selection_bg: "#44475a", // selection
    overlay_bg:   "#21222c", // darker bg (standard Dracula extension)
    gauge_bg:     "#44475a", // selection
    art_bg:       "#282a36",
    art_border:   "#44475a", // selection
};

// Gruvbox Dark — https://github.com/morhetz/gruvbox
const GRUVBOX_DARK: PresetColors = PresetColors {
    foreground:   "#ebdbb2", // light1
    background:   "#282828", // dark medium
    accent:       "#fe8019", // orange bright
    accent2:      "#fabd2f", // yellow bright
    dim:          "#a89984", // light4
    highlight:    "#fabd2f", // yellow bright
    playing:      "#b8bb26", // green bright
    header_bg:    "#3c3836", // dark1
    selection_bg: "#504945", // dark2
    overlay_bg:   "#1d2021", // dark hard
    gauge_bg:     "#3c3836", // dark1
    art_bg:       "#282828",
    art_border:   "#504945", // dark2
};

// Rosé Pine — https://github.com/rose-pine/rose-pine-palette
const ROSE_PINE: PresetColors = PresetColors {
    foreground:   "#e0def4", // text
    background:   "#191724", // base
    accent:       "#eb6f92", // love — punchy pink
    accent2:      "#ebbcba", // rose — softer pink
    dim:          "#6e6a86", // muted
    highlight:    "#f6c177", // gold
    playing:      "#9ccfd8", // foam
    header_bg:    "#1f1d2e", // surface
    selection_bg: "#26233a", // overlay
    overlay_bg:   "#191724", // base (darkest available)
    gauge_bg:     "#1f1d2e", // surface
    art_bg:       "#191724", // base
    art_border:   "#1f1d2e", // surface
};

/// All available theme preset names. Index 0 ("default") means no preset.
pub const THEME_NAMES: &[&str] = &[
    "default",
    "catppuccin-mocha",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "nord",
    "dracula",
    "gruvbox",
    "rose-pine",
];

/// Returns [accent, highlight, playing, accent2, dim] swatches for a theme.
pub fn theme_swatch(name: &str) -> [Color; 5] {
    if let Some(p) = get_preset(name) {
        [
            parse_color(p.accent),
            parse_color(p.highlight),
            parse_color(p.playing),
            parse_color(p.accent2),
            parse_color(p.dim),
        ]
    } else {
        [
            parse_color(default_colors::ACCENT),
            parse_color(default_colors::HIGHLIGHT),
            parse_color(default_colors::PLAYING),
            parse_color(default_colors::ACCENT2),
            parse_color(default_colors::DIM),
        ]
    }
}

fn get_preset(name: &str) -> Option<&'static PresetColors> {
    match name.to_lowercase().as_str() {
        "catppuccin" | "catppuccin-mocha" => Some(&CATPPUCCIN_MOCHA),
        "catppuccin-latte"                => Some(&CATPPUCCIN_LATTE),
        "catppuccin-frappe"               => Some(&CATPPUCCIN_FRAPPE),
        "catppuccin-macchiato"            => Some(&CATPPUCCIN_MACCHIATO),
        "nord"                            => Some(&NORD),
        "dracula"                         => Some(&DRACULA),
        "gruvbox" | "gruvbox-dark"        => Some(&GRUVBOX_DARK),
        "rose-pine" | "rose-pine-main"    => Some(&ROSE_PINE),
        _                                 => None,
    }
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
    pub fn from_config(colors: &UiColorsConfig, theme: Option<&str>) -> Self {
        let preset = theme.and_then(get_preset);

        macro_rules! resolve {
            ($field:ident, $default:expr) => {
                parse_color(
                    colors.$field.as_deref().unwrap_or_else(|| {
                        preset.map(|p| p.$field).unwrap_or($default)
                    }),
                )
            };
        }

        Self {
            foreground:   resolve!(foreground,   default_colors::FOREGROUND),
            background:   resolve!(background,   default_colors::BACKGROUND),
            accent:       resolve!(accent,       default_colors::ACCENT),
            accent2:      resolve!(accent2,      default_colors::ACCENT2),
            dim:          resolve!(dim,           default_colors::DIM),
            highlight:    resolve!(highlight,    default_colors::HIGHLIGHT),
            playing:      resolve!(playing,      default_colors::PLAYING),
            header_bg:    resolve!(header_bg,    default_colors::HEADER_BG),
            selection_bg: resolve!(selection_bg, default_colors::SELECTION_BG),
            overlay_bg:   resolve!(overlay_bg,   default_colors::OVERLAY_BG),
            gauge_bg:     resolve!(gauge_bg,     default_colors::GAUGE_BG),
            art_bg:       resolve!(art_bg,       default_colors::ART_BG),
            art_border:   resolve!(art_border,   default_colors::ART_BORDER),
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
