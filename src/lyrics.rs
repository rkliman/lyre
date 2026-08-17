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

/// Write lyrics to the primary tag of the file at `path`. Returns true on success.
pub fn write_lyrics_to_tag(path: &str, lyrics: &str) -> bool {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::prelude::*;
    use lofty::tag::TagExt;

    let Ok(mut tagged) = lofty::read_from_path(path) else {
        return false;
    };
    let Some(tag) = tagged.primary_tag_mut() else {
        return false;
    };
    tag.insert_text(ItemKey::Lyrics, lyrics.to_string());
    tag.save_to_path(path, WriteOptions::default()).is_ok()
}

/// Map an EditField to its corresponding lofty ItemKey.
fn edit_field_to_item_key(field: crate::types::EditField) -> lofty::prelude::ItemKey {
    use crate::types::EditField as F;
    use lofty::prelude::ItemKey as K;
    match field {
        F::Title => K::TrackTitle,
        F::Artist => K::TrackArtist,
        F::Album => K::AlbumTitle,
        F::AlbumArtist => K::AlbumArtist,
        F::Year => K::Year,
        F::Genre => K::Genre,
        F::Composer => K::Composer,
        F::Lyricist => K::Lyricist,
        F::Conductor => K::Conductor,
        F::Producer => K::Producer,
        F::Remixer => K::Remixer,
        F::TrackNumber => K::TrackNumber,
        F::TrackTotal => K::TrackTotal,
        F::DiscNumber => K::DiscNumber,
        F::DiscTotal => K::DiscTotal,
        F::Work => K::Work,
        F::Movement => K::Movement,
        F::MovementNumber => K::MovementNumber,
        F::MovementTotal => K::MovementTotal,
        F::Comment => K::Comment,
        F::Description => K::Description,
        F::ContentGroup => K::ContentGroup,
        F::Compilation => K::FlagCompilation,
        F::Mood => K::Mood,
        F::Language => K::Language,
        F::InitialKey => K::InitialKey,
        F::Bpm => K::Bpm,
        F::Publisher => K::Publisher,
        F::Label => K::Label,
        F::Copyright => K::CopyrightMessage,
        F::Isrc => K::Isrc,
        F::Barcode => K::Barcode,
        F::CatalogNumber => K::CatalogNumber,
        F::RecordingDate => K::RecordingDate,
        F::ReleaseDate => K::ReleaseDate,
        F::OriginalReleaseDate => K::OriginalReleaseDate,
        F::OriginalArtist => K::OriginalArtist,
        F::OriginalAlbum => K::OriginalAlbumTitle,
        F::SortTitle => K::TrackTitleSortOrder,
        F::SortArtist => K::TrackArtistSortOrder,
        F::SortAlbum => K::AlbumTitleSortOrder,
        F::SortAlbumArtist => K::AlbumArtistSortOrder,
        F::MusicBrainzTrackId => K::MusicBrainzTrackId,
        F::MusicBrainzRecordingId => K::MusicBrainzRecordingId,
        F::MusicBrainzReleaseId => K::MusicBrainzReleaseId,
        F::MusicBrainzArtistId => K::MusicBrainzArtistId,
        F::ReplayGainTrackGain => K::ReplayGainTrackGain,
        F::ReplayGainTrackPeak => K::ReplayGainTrackPeak,
        F::ReplayGainAlbumGain => K::ReplayGainAlbumGain,
        F::ReplayGainAlbumPeak => K::ReplayGainAlbumPeak,
        F::EncoderSoftware => K::EncoderSoftware,
    }
}

/// Normalize the boolean-ish Compilation flag to canonical "1"/"0" (or empty to clear).
fn normalize_compilation(input: &str) -> Option<&'static str> {
    let s = input.trim().to_ascii_lowercase();
    if s.is_empty() {
        None
    } else if matches!(s.as_str(), "1" | "true" | "yes" | "y" | "t") {
        Some("1")
    } else {
        Some("0")
    }
}

/// Format a Compilation flag for display: "yes" / "no" / "".
pub fn display_compilation(raw: &str) -> String {
    match normalize_compilation(raw) {
        Some("1") => "yes".to_string(),
        Some(_) => "no".to_string(),
        None => String::new(),
    }
}

/// Read all supported metadata fields from the primary tag of `path`.
/// Missing fields are omitted from the map (not inserted as empty strings).
pub fn read_metadata_from_tag(
    path: &str,
) -> std::collections::HashMap<crate::types::EditField, String> {
    use crate::types::EditField;
    use lofty::prelude::*;
    let mut out = std::collections::HashMap::new();
    let Ok(tagged) = lofty::read_from_path(path) else {
        return out;
    };
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return out;
    };
    for &field in EditField::DETAILED {
        let key = edit_field_to_item_key(field);
        if let Some(value) = tag.get_string(&key) {
            let value = if field == EditField::Compilation {
                display_compilation(value)
            } else {
                value.to_string()
            };
            if !value.is_empty() {
                out.insert(field, value);
            }
        }
    }
    out
}

/// Write a set of metadata fields to the primary tag of `path`. Empty values
/// clear the corresponding tag. Only fields present in `values` are touched.
/// Returns true on success.
pub fn write_metadata_to_tag(
    path: &str,
    values: &std::collections::HashMap<crate::types::EditField, String>,
) -> bool {
    use crate::types::EditField;
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::tag::TagExt;

    let Ok(mut tagged) = lofty::read_from_path(path) else {
        return false;
    };
    let tag = match tagged.primary_tag_mut() {
        Some(t) => t,
        None => match tagged.first_tag_mut() {
            Some(t) => t,
            None => return false,
        },
    };

    for (field, value) in values {
        let key = edit_field_to_item_key(*field);
        let value = value.trim();
        if *field == EditField::Compilation {
            match normalize_compilation(value) {
                Some(v) => {
                    tag.insert_text(key, v.to_string());
                }
                None => {
                    tag.remove_key(&key);
                }
            }
        } else if value.is_empty() {
            tag.remove_key(&key);
        } else {
            tag.insert_text(key, value.to_string());
        }
    }

    tag.save_to_path(path, WriteOptions::default()).is_ok()
}

/// Read technical (read-only) properties: format, codec, bitrate, sample rate,
/// channels, bit depth, file size. Returns (label, value) pairs in display order.
pub fn read_file_properties(path: &str) -> Vec<(String, String)> {
    use lofty::file::{AudioFile, TaggedFileExt};
    let mut out = Vec::new();

    if let Some(ext) = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
    {
        out.push(("Format".to_string(), ext.to_ascii_uppercase()));
    }

    if let Ok(size) = std::fs::metadata(path).map(|m| m.len()) {
        out.push(("File Size".to_string(), crate::util::format_bytes(size)));
    }

    let Ok(tagged) = lofty::read_from_path(path) else {
        return out;
    };
    let props = tagged.properties();
    out.push(("Codec".to_string(), format!("{:?}", tagged.file_type())));
    if let Some(br) = props.audio_bitrate() {
        out.push(("Bitrate".to_string(), format!("{} kbps", br)));
    }
    if let Some(sr) = props.sample_rate() {
        out.push(("Sample Rate".to_string(), format!("{} Hz", sr)));
    }
    if let Some(ch) = props.channels() {
        out.push(("Channels".to_string(), ch.to_string()));
    }
    if let Some(bd) = props.bit_depth() {
        out.push(("Bit Depth".to_string(), format!("{} bit", bd)));
    }
    out
}

