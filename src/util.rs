/// Glyph shown in the tracklist favorite column for favorited tracks.
pub const FAVORITE_ICON: &str = "★";

pub fn get_num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    let exp = ((bytes as f64).ln() / 1024_f64.ln()).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);
    let value = bytes as f64 / 1024_f64.powi(exp as i32);
    format!("{:.2} {}", value, UNITS[exp])
}

/// Expand tilde (~) to home directory
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

/// Pad `s` to exactly `width` display columns by appending spaces.
/// Rust's `{:<width$}` uses char count, not display width — this fixes that.
pub fn pad_to(s: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let display_w = s.width();
    if display_w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_w))
    }
}

/// Truncate `s` to `max` display columns, appending `…` if it was cut.
/// Pads with spaces to exactly `max` columns. Unicode-aware.
pub fn truncate_field(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    use unicode_width::UnicodeWidthStr;
    if max == 0 {
        return String::new();
    }
    if s.width() <= max {
        return pad_to(s, max);
    }

    // Build up to max-1 display cols, then add …
    let mut result = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(1);
        if w + cw > max - 1 {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result.push('…');
    w += 1;
    // Pad if the ellipsis itself left a gap (e.g. last char was 2-wide)
    if w < max {
        result.push_str(&" ".repeat(max - w));
    }
    result
}

/// Split `s` into lines where each line's display width ≤ `max` columns.
/// Breaks on whitespace where possible; falls back to character-breaking
/// only when a single word is wider than `max`.
pub fn wrap_field(s: &str, max: usize) -> Vec<String> {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    if max == 0 || s.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in s.split_whitespace() {
        let word_width = word.width();

        // Case 1: Word exceeds max width - must be split by characters
        if word_width > max {
            // Flush existing line before handling the giant word
            if !current_line.is_empty() {
                lines.push(std::mem::take(&mut current_line));
                current_width = 0;
            }

            for ch in word.chars() {
                let cw = ch.width().unwrap_or(1);
                if current_width + cw > max && !current_line.is_empty() {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
                current_line.push(ch);
                current_width += cw;
            }
            continue;
        }

        // Case 2: Regular word wrapping
        let space_needed = if current_line.is_empty() { 0 } else { 1 };

        if current_width + space_needed + word_width <= max {
            if space_needed == 1 {
                current_line.push(' ');
            }
            current_line.push_str(word);
            current_width += space_needed + word_width;
        } else {
            // Push current line and start a new one with the current word
            lines.push(std::mem::take(&mut current_line));
            current_line.push_str(word);
            current_width = word_width;
        }
    }

    if !current_line.is_empty() || lines.is_empty() {
        lines.push(current_line);
    }

    lines
}