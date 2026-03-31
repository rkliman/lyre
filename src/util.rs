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