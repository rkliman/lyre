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

// Rosé Pine Dawn — light variant
const ROSE_PINE_DAWN: PresetColors = PresetColors {
    foreground:   "#464261", // text
    background:   "#faf4ed", // base
    accent:       "#b4637a", // love
    accent2:      "#d7827e", // rose
    dim:          "#9893a5", // muted
    highlight:    "#ea9d34", // gold
    playing:      "#56949f", // foam
    header_bg:    "#fffaf3", // surface
    selection_bg: "#f2e9e1", // overlay
    overlay_bg:   "#faf4ed",
    gauge_bg:     "#f2e9e1",
    art_bg:       "#faf4ed",
    art_border:   "#f2e9e1",
};

// Tokyo Night — https://github.com/folke/tokyonight.nvim
const TOKYO_NIGHT: PresetColors = PresetColors {
    foreground:   "#c0caf5",
    background:   "#1a1b26",
    accent:       "#7aa2f7", // blue
    accent2:      "#bb9af7", // purple
    dim:          "#565f89",
    highlight:    "#e0af68", // yellow
    playing:      "#9ece6a", // green
    header_bg:    "#16161e",
    selection_bg: "#283457",
    overlay_bg:   "#13131a",
    gauge_bg:     "#292e42",
    art_bg:       "#1a1b26",
    art_border:   "#292e42",
};

// Tokyo Night Storm — slightly lighter Tokyo Night variant
const TOKYO_NIGHT_STORM: PresetColors = PresetColors {
    foreground:   "#c0caf5",
    background:   "#24283b",
    accent:       "#7aa2f7",
    accent2:      "#bb9af7",
    dim:          "#565f89",
    highlight:    "#e0af68",
    playing:      "#9ece6a",
    header_bg:    "#1f2335",
    selection_bg: "#2e3c64",
    overlay_bg:   "#1a1b2e",
    gauge_bg:     "#3b4261",
    art_bg:       "#24283b",
    art_border:   "#3b4261",
};

// Tokyo Night Light
const TOKYO_NIGHT_LIGHT: PresetColors = PresetColors {
    foreground:   "#3760bf",
    background:   "#e1e2e7",
    accent:       "#2e7de9", // blue
    accent2:      "#9854f1", // purple
    dim:          "#848cb5", // comment
    highlight:    "#8c6c3e",
    playing:      "#587539", // green
    header_bg:    "#d0d5e3", // bg_dark
    selection_bg: "#c4c8da", // bg_highlight
    overlay_bg:   "#cbccd6",
    gauge_bg:     "#c4c8da",
    art_bg:       "#e1e2e7",
    art_border:   "#c4c8da",
};

// Solarized Dark — https://ethanschoonover.com/solarized
const SOLARIZED_DARK: PresetColors = PresetColors {
    foreground:   "#839496", // base0
    background:   "#002b36", // base03
    accent:       "#268bd2", // blue
    accent2:      "#2aa198", // cyan
    dim:          "#586e75", // base01
    highlight:    "#b58900", // yellow
    playing:      "#859900", // green
    header_bg:    "#073642", // base02
    selection_bg: "#073642",
    overlay_bg:   "#001f27",
    gauge_bg:     "#073642",
    art_bg:       "#002b36",
    art_border:   "#073642",
};

// Solarized Light
const SOLARIZED_LIGHT: PresetColors = PresetColors {
    foreground:   "#657b83", // base00
    background:   "#fdf6e3", // base3
    accent:       "#268bd2", // blue
    accent2:      "#2aa198", // cyan
    dim:          "#93a1a1", // base1
    highlight:    "#b58900", // yellow
    playing:      "#859900", // green
    header_bg:    "#eee8d5", // base2
    selection_bg: "#eee8d5",
    overlay_bg:   "#f5f0e4",
    gauge_bg:     "#eee8d5",
    art_bg:       "#fdf6e3",
    art_border:   "#eee8d5",
};

// One Dark — https://github.com/atom/atom/tree/master/packages/one-dark-syntax
const ONE_DARK: PresetColors = PresetColors {
    foreground:   "#abb2bf",
    background:   "#282c34",
    accent:       "#61afef", // blue
    accent2:      "#c678dd", // purple
    dim:          "#5c6370", // comment
    highlight:    "#e5c07b", // yellow
    playing:      "#98c379", // green
    header_bg:    "#21252b",
    selection_bg: "#3e4451",
    overlay_bg:   "#1d2026",
    gauge_bg:     "#31353f",
    art_bg:       "#282c34",
    art_border:   "#3e4451",
};

// Monokai — https://monokai.pro
const MONOKAI: PresetColors = PresetColors {
    foreground:   "#f8f8f2",
    background:   "#272822",
    accent:       "#f92672", // pink/red — Monokai's signature
    accent2:      "#ae81ff", // purple
    dim:          "#75715e", // comment
    highlight:    "#e6db74", // yellow
    playing:      "#a6e22e", // green
    header_bg:    "#1e1f1c",
    selection_bg: "#49483e",
    overlay_bg:   "#1a1b19",
    gauge_bg:     "#3c3d37",
    art_bg:       "#272822",
    art_border:   "#3c3d37",
};

// Gruvbox Light — https://github.com/morhetz/gruvbox
const GRUVBOX_LIGHT: PresetColors = PresetColors {
    foreground:   "#3c3836", // dark1
    background:   "#fbf1c7", // light0
    accent:       "#af3a03", // orange dark
    accent2:      "#b57614", // yellow dark
    dim:          "#928374", // gray
    highlight:    "#b57614",
    playing:      "#79740e", // green dark
    header_bg:    "#f2e5bc", // light1
    selection_bg: "#d5c4a1", // light2
    overlay_bg:   "#f9f5d7", // light0 hard
    gauge_bg:     "#ebdbb2", // light3
    art_bg:       "#fbf1c7",
    art_border:   "#d5c4a1",
};

// Kanagawa Wave — https://github.com/rebelot/kanagawa.nvim
const KANAGAWA: PresetColors = PresetColors {
    foreground:   "#dcd7ba", // fujiWhite
    background:   "#1f1f28", // sumiInk0
    accent:       "#7e9cd8", // crystalBlue
    accent2:      "#957fb8", // oniViolet
    dim:          "#727169", // fujiGray
    highlight:    "#e6c384", // carpYellow
    playing:      "#98bb6c", // springGreen
    header_bg:    "#16161d",
    selection_bg: "#2d4f67", // waveBlue1
    overlay_bg:   "#16161d",
    gauge_bg:     "#223249",
    art_bg:       "#1f1f28",
    art_border:   "#2a2a37",
};

// Everforest Dark — https://github.com/sainnhe/everforest
const EVERFOREST: PresetColors = PresetColors {
    foreground:   "#d3c6aa",
    background:   "#2d353b",
    accent:       "#7fbbb3", // aqua
    accent2:      "#d699b6", // purple
    dim:          "#859289",
    highlight:    "#dbbc7f", // yellow
    playing:      "#a7c080", // green
    header_bg:    "#343f44", // bg1
    selection_bg: "#3d484d", // bg2
    overlay_bg:   "#232a2e", // hard bg
    gauge_bg:     "#475258", // bg3
    art_bg:       "#2d353b",
    art_border:   "#475258",
};

// Ayu Dark — https://github.com/ayu-theme/ayu-vim
const AYU_DARK: PresetColors = PresetColors {
    foreground:   "#e6e1cf",
    background:   "#0f1419",
    accent:       "#ffb454", // function/orange
    accent2:      "#36a3d9", // tag/blue
    dim:          "#5c6773", // comment
    highlight:    "#e6b673", // special/yellow
    playing:      "#b8cc52", // string/green
    header_bg:    "#14191f", // panel
    selection_bg: "#253340", // selection
    overlay_bg:   "#14191f",
    gauge_bg:     "#151a1e", // line
    art_bg:       "#0f1419",
    art_border:   "#2d3640", // guide
};

// Ayu Mirage
const AYU_MIRAGE: PresetColors = PresetColors {
    foreground:   "#d9d7ce",
    background:   "#212733",
    accent:       "#ffd57f", // function/orange
    accent2:      "#5ccfe6", // tag/blue
    dim:          "#5c6773", // comment
    highlight:    "#ffc44c", // special/yellow
    playing:      "#bae67e", // string/green
    header_bg:    "#272d38", // panel
    selection_bg: "#343f4c", // selection
    overlay_bg:   "#161b22",
    gauge_bg:     "#242b38", // line
    art_bg:       "#212733",
    art_border:   "#3d4751", // guide
};

// Tomorrow Night — https://github.com/chriskempson/tomorrow-theme
const TOMORROW_NIGHT: PresetColors = PresetColors {
    foreground:   "#c5c8c6",
    background:   "#1d1f21",
    accent:       "#81a2be", // blue
    accent2:      "#b294bb", // purple
    dim:          "#969896",
    highlight:    "#f0c674", // yellow
    playing:      "#b5bd68", // green
    header_bg:    "#282a2e",
    selection_bg: "#373b41",
    overlay_bg:   "#14161a",
    gauge_bg:     "#282a2e",
    art_bg:       "#1d1f21",
    art_border:   "#373b41",
};

// Oxocarbon — IBM Carbon design system, https://github.com/nyoom-engineering/oxocarbon.nvim
const OXOCARBON: PresetColors = PresetColors {
    foreground:   "#f2f4f8",
    background:   "#161616",
    accent:       "#78a9ff", // blue
    accent2:      "#be95ff", // purple
    dim:          "#5c5c5c", // base03 (30% blend)
    highlight:    "#08bdba", // base07 — teal
    playing:      "#42be65", // base13 — green
    header_bg:    "#2a2a2a", // base01 (8.5% blend)
    selection_bg: "#404040", // base02 (18% blend)
    overlay_bg:   "#131313", // blend value
    gauge_bg:     "#2a2a2a",
    art_bg:       "#161616",
    art_border:   "#393939",
};

// Nightfox — https://github.com/EdenEast/nightfox.nvim
const NIGHTFOX: PresetColors = PresetColors {
    foreground:   "#cdcecf",
    background:   "#192330",
    accent:       "#719cd6", // blue
    accent2:      "#9d79d6", // purple
    dim:          "#738091",
    highlight:    "#dbc074", // yellow
    playing:      "#81b29a", // green
    header_bg:    "#131a24",
    selection_bg: "#2b3b51",
    overlay_bg:   "#131a24", // bg0
    gauge_bg:     "#212e3f",
    art_bg:       "#192330",
    art_border:   "#2b3b51",
};

// Material Dark (Material Theme) — https://material-theme.site
const MATERIAL_DARK: PresetColors = PresetColors {
    foreground:   "#eeffff",
    background:   "#263238",
    accent:       "#89ddff", // cyan
    accent2:      "#c792ea", // purple
    dim:          "#546e7a",
    highlight:    "#ffcb6b", // yellow
    playing:      "#c3e88d", // green
    header_bg:    "#1e272c",
    selection_bg: "#314549",
    overlay_bg:   "#1a2327",
    gauge_bg:     "#2e3c43",
    art_bg:       "#263238",
    art_border:   "#314549",
};

// Synthwave '84 — https://github.com/robb0wen/synthwave-vscode
const SYNTHWAVE84: PresetColors = PresetColors {
    foreground:   "#ffffff",
    background:   "#262335",
    accent:       "#ff7edb", // hot pink
    accent2:      "#03edf9", // neon cyan
    dim:          "#848bbd", // comments
    highlight:    "#fede5d", // neon yellow
    playing:      "#72f1b8", // neon green
    header_bg:    "#241b2e",
    selection_bg: "#3d3b6e",
    overlay_bg:   "#1a1631",
    gauge_bg:     "#34294f",
    art_bg:       "#262335",
    art_border:   "#34294f",
};

// Cyberdream — https://github.com/scottmckendry/cyberdream.nvim
const CYBERDREAM: PresetColors = PresetColors {
    foreground:   "#ffffff",
    background:   "#16181a",
    accent:       "#ff5ea0", // pink
    accent2:      "#ff5ef1", // magenta
    dim:          "#7b8496", // grey
    highlight:    "#f1ff5e", // neon yellow
    playing:      "#5eff6c", // neon green
    header_bg:    "#1e2124", // bg_alt
    selection_bg: "#3c4048", // bg_highlight
    overlay_bg:   "#111315",
    gauge_bg:     "#1e2124",
    art_bg:       "#16181a",
    art_border:   "#3c4048",
};

// Horizon — https://github.com/ntk148v/vim-horizon
const HORIZON: PresetColors = PresetColors {
    foreground:   "#d5d8da",
    background:   "#1c1e26",
    accent:       "#e95678", // hot pink/red
    accent2:      "#b877db", // purple
    dim:          "#6c6f93", // comments
    highlight:    "#fab795", // peach/orange
    playing:      "#09f7a0", // neon green
    header_bg:    "#17171b",
    selection_bg: "#272c42",
    overlay_bg:   "#17171b",
    gauge_bg:     "#2e303e",
    art_bg:       "#1c1e26",
    art_border:   "#2e303e",
};

// Poimandres — https://github.com/olivercederborg/poimandres.nvim
const POIMANDRES: PresetColors = PresetColors {
    foreground:   "#e4f0fb", // text
    background:   "#1b1e28", // background2
    accent:       "#d0679d", // pink3
    accent2:      "#5de4c7", // teal1
    dim:          "#767c9d", // blueGray2
    highlight:    "#fffac2", // yellow
    playing:      "#5fb3a1", // teal2
    header_bg:    "#171922", // background3
    selection_bg: "#303340", // background1
    overlay_bg:   "#141720",
    gauge_bg:     "#303340",
    art_bg:       "#1b1e28",
    art_border:   "#303340",
};

// Fairy Floss — https://github.com/sailorhg/fairyfloss
const FAIRY_FLOSS: PresetColors = PresetColors {
    foreground:   "#f8f8f2",
    background:   "#5a5475",
    accent:       "#ffb8d1", // pink keywords
    accent2:      "#c5a3ff", // lavender constants
    dim:          "#a19bbb", // muted purple
    highlight:    "#fff352", // bright yellow
    playing:      "#c2ffdf", // mint green
    header_bg:    "#4c4468",
    selection_bg: "#6b618e",
    overlay_bg:   "#3e3a54",
    gauge_bg:     "#6b618e",
    art_bg:       "#5a5475",
    art_border:   "#6b618e",
};

// Girlypop Dark — original
const GIRLYPOP_DARK: PresetColors = PresetColors {
    foreground:   "#ffe0f0", // rosy white
    background:   "#1a0818", // dark magenta-black
    accent:       "#ff1493", // deep pink
    accent2:      "#ff69b4", // hot pink / bubblegum
    dim:          "#c24d82", // muted pink
    highlight:    "#ff85c2", // bright bubblegum
    playing:      "#ff006e", // electric magenta — stands out
    header_bg:    "#2a0a20",
    selection_bg: "#4a1535",
    overlay_bg:   "#120614",
    gauge_bg:     "#3a1030",
    art_bg:       "#1a0818",
    art_border:   "#4a1535",
};

// Girlypop Light
const GIRLYPOP_LIGHT: PresetColors = PresetColors {
    foreground:   "#7a0040", // dark crimson-pink — readable on light
    background:   "#fff0f8", // near-white with pink blush
    accent:       "#ff1493", // deep pink
    accent2:      "#cc0066", // darker hot pink
    dim:          "#d4608a", // muted rose
    highlight:    "#ff006e", // electric magenta
    playing:      "#e60073", // vivid pink
    header_bg:    "#ffd6ed",
    selection_bg: "#ffb3d9",
    overlay_bg:   "#ffe8f5",
    gauge_bg:     "#ffb3d9",
    art_bg:       "#fff0f8",
    art_border:   "#ffb3d9",
};

// Mellow — https://github.com/mellow-theme/mellow.nvim
const MELLOW: PresetColors = PresetColors {
    foreground:   "#c9c7cd",
    background:   "#161617",
    accent:       "#ea83a5", // rose/pink (listed as "cyan" but is rose)
    accent2:      "#e29eca", // magenta/pink
    dim:          "#757581", // gray05
    highlight:    "#e6b99d", // warm peach
    playing:      "#90b99f", // green
    header_bg:    "#131314", // bg_dark
    selection_bg: "#2a2a2d", // gray02
    overlay_bg:   "#18181a", // gray00
    gauge_bg:     "#27272a",
    art_bg:       "#161617",
    art_border:   "#27272a",
};

// Palenight (Material Palenight) — https://github.com/drewtempelmeyer/palenight.vim
const PALENIGHT: PresetColors = PresetColors {
    foreground:   "#bfc7d5", // white
    background:   "#292d3e", // black
    accent:       "#c792ea", // purple
    accent2:      "#82b1ff", // blue
    dim:          "#697098", // comment_grey
    highlight:    "#ffcb6b", // yellow
    playing:      "#c3e88d", // green
    header_bg:    "#252837",
    selection_bg: "#3e4452", // visual_grey
    overlay_bg:   "#1e2130",
    gauge_bg:     "#32374d",
    art_bg:       "#292d3e",
    art_border:   "#34394f",
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
    "gruvbox-light",
    "rose-pine",
    "rose-pine-dawn",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-light",
    "solarized-dark",
    "solarized-light",
    "one-dark",
    "monokai",
    "kanagawa",
    "everforest",
    "ayu-dark",
    "ayu-mirage",
    "tomorrow-night",
    "oxocarbon",
    "nightfox",
    "material-dark",
    "palenight",
    "synthwave84",
    "cyberdream",
    "horizon",
    "poimandres",
    "fairy-floss",
    "mellow",
    "girlypop-dark",
    "girlypop-light",
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
        "catppuccin" | "catppuccin-mocha"    => Some(&CATPPUCCIN_MOCHA),
        "catppuccin-latte"                   => Some(&CATPPUCCIN_LATTE),
        "catppuccin-frappe"                  => Some(&CATPPUCCIN_FRAPPE),
        "catppuccin-macchiato"               => Some(&CATPPUCCIN_MACCHIATO),
        "nord"                               => Some(&NORD),
        "dracula"                            => Some(&DRACULA),
        "gruvbox" | "gruvbox-dark"           => Some(&GRUVBOX_DARK),
        "gruvbox-light"                      => Some(&GRUVBOX_LIGHT),
        "rose-pine" | "rose-pine-main"       => Some(&ROSE_PINE),
        "rose-pine-dawn"                     => Some(&ROSE_PINE_DAWN),
        "tokyo-night"                        => Some(&TOKYO_NIGHT),
        "tokyo-night-storm"                  => Some(&TOKYO_NIGHT_STORM),
        "tokyo-night-light"                  => Some(&TOKYO_NIGHT_LIGHT),
        "solarized" | "solarized-dark"       => Some(&SOLARIZED_DARK),
        "solarized-light"                    => Some(&SOLARIZED_LIGHT),
        "one-dark" | "onedark"               => Some(&ONE_DARK),
        "monokai"                            => Some(&MONOKAI),
        "kanagawa" | "kanagawa-wave"         => Some(&KANAGAWA),
        "everforest" | "everforest-dark"     => Some(&EVERFOREST),
        "ayu" | "ayu-dark"                   => Some(&AYU_DARK),
        "ayu-mirage"                         => Some(&AYU_MIRAGE),
        "tomorrow-night"                     => Some(&TOMORROW_NIGHT),
        "oxocarbon"                          => Some(&OXOCARBON),
        "nightfox"                           => Some(&NIGHTFOX),
        "material" | "material-dark"                    => Some(&MATERIAL_DARK),
        "palenight" | "material-palenight"              => Some(&PALENIGHT),
        "synthwave84" | "synthwave-84" | "synthwave"   => Some(&SYNTHWAVE84),
        "cyberdream"                                    => Some(&CYBERDREAM),
        "horizon"                                       => Some(&HORIZON),
        "poimandres"                                    => Some(&POIMANDRES),
        "fairy-floss" | "fairyfloss"                   => Some(&FAIRY_FLOSS),
        "mellow"                                        => Some(&MELLOW),
        "girlypop" | "girlypop-dark"                   => Some(&GIRLYPOP_DARK),
        "girlypop-light"                               => Some(&GIRLYPOP_LIGHT),
        _                                               => None,
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
