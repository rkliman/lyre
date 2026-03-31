/// Parse lyrics text into structured format with timestamps if present.
/// Supports LRC format: `[mm:ss.xx]lyric text`
pub fn parse_lyrics(lyrics_text: &str) -> crate::types::Lyrics {
    use crate::types::{LyricLine, Lyrics};

    let mut timed_lines = Vec::new();
    let mut has_timestamps = false;

    for line in lyrics_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try to parse LRC format: [mm:ss.xx]text or [mm:ss]text
        if let Some(parsed) = parse_lrc_line(trimmed) {
            has_timestamps = true;
            timed_lines.push(parsed);
        } else {
            // Plain text line without timestamp
            timed_lines.push(LyricLine {
                timestamp: None,
                text: trimmed.to_string(),
            });
        }
    }

    if has_timestamps {
        // Sort by timestamp (lines without timestamps go at the end)
        timed_lines.sort_by(|a, b| {
            match (a.timestamp, b.timestamp) {
                (Some(t1), Some(t2)) => t1.partial_cmp(&t2).unwrap(),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        Lyrics::Timed(timed_lines)
    } else {
        // No timestamps found, return as plain text
        Lyrics::Plain(lyrics_text.to_string())
    }
}

/// Parse a single LRC line: [mm:ss.xx]text or [mm:ss]text
fn parse_lrc_line(line: &str) -> Option<crate::types::LyricLine> {
    use crate::types::LyricLine;

    if !line.starts_with('[') {
        return None;
    }

    let close_bracket = line.find(']')?;
    let timestamp_str = &line[1..close_bracket];
    let text = line[close_bracket + 1..].trim().to_string();

    // Parse timestamp: mm:ss.xx or mm:ss
    let parts: Vec<&str> = timestamp_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let minutes: f64 = parts[0].parse().ok()?;
    let seconds: f64 = parts[1].parse().ok()?;

    let timestamp = minutes * 60.0 + seconds;

    Some(LyricLine {
        timestamp: Some(timestamp),
        text,
    })
}

/// Extract lyrics from the file at `path`.
/// Looks for unsynchronized lyrics in various tag formats.
pub fn extract_lyrics(path: &str) -> Option<String> {
    use lofty::prelude::*;

    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;

    // Try to get lyrics from common tag keys
    // Different formats use different keys: LYRICS, UNSYNCEDLYRICS, etc.
    tag.get_string(&ItemKey::Lyrics).map(|s| s.to_string())
}