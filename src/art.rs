/// Album art extraction and rendering.
///
/// Supports multiple rendering backends:
/// - Terminal graphics protocols (Kitty, iTerm2, Sixel) for high-quality images
/// - Unicode block-art fallback for terminals without graphics support
///
/// Block-art uses the upper-half-block character `▀` to pack two vertical pixels per cell:
///   - foreground colour  = top pixel
///   - background colour  = bottom pixel
///
/// So an image resized to W × (H*2) pixels renders as W×H character cells.
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui_image::picker::Picker;

/// A rendered block-art image: rows of styled Spans ready for ratatui.
#[derive(Clone)]
pub struct BlockArt {
    pub rows: Vec<Line<'static>>,
}

impl BlockArt {
    /// Returns a placeholder block of the given size filled with dim `░` chars.
    pub fn placeholder(char_w: u16, char_h: u16, fg: Color, bg: Color) -> Self {
        let row_str = "░".repeat(char_w as usize);
        let rows = (0..char_h)
            .map(|_| {
                Line::from(Span::styled(
                    row_str.clone(),
                    Style::default().fg(fg).bg(bg),
                ))
            })
            .collect();
        Self { rows }
    }
}

/// Extract the first embedded cover image from the file at `path`.
/// Returns raw image bytes on success.
pub fn extract_cover_bytes(path: &str) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;

    // Prefer CoverFront, fall back to any picture
    let pic = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;

    Some(pic.data().to_vec())
}

/// Extract duration (in seconds) from the file at `path`.
pub fn extract_duration(path: &str) -> Option<i64> {
    let tagged = lofty::read_from_path(path).ok()?;
    let properties = tagged.properties();
    Some(properties.duration().as_secs() as i64)
}

/// Extract cover art from `path` and write it to a temp file.
/// Returns a `file://` URI suitable for `mpris:artUrl`, or `None` if
/// the track has no embedded art.
pub fn extract_cover_to_temp_file(track_path: &str) -> Option<String> {
    let bytes = extract_cover_bytes(track_path)?;
    write_bytes_to_temp_file(&bytes)
}

/// Write album art bytes to a temp file.
/// Returns a `file://` URI suitable for `mpris:artUrl`.
/// This is more efficient than extract_cover_to_temp_file when you already have the bytes cached.
pub fn write_bytes_to_temp_file(bytes: &[u8]) -> Option<String> {
    // Detect format from magic bytes to pick the right extension
    let ext = if bytes.starts_with(b"\xff\xd8\xff") {
        "jpg"
    } else if bytes.starts_with(b"\x89PNG") {
        "png"
    } else {
        "jpg" // safe fallback
    };

    let tmp_path = format!("/tmp/lyre-cover.{}", ext);
    std::fs::write(&tmp_path, bytes).ok()?;
    Some(format!("file://{}", tmp_path))
}
/// Load cover art from bytes into a DynamicImage for rendering with terminal graphics.
pub fn load_cover_image(bytes: &[u8]) -> Option<DynamicImage> {
    image::load_from_memory(bytes).ok()
}

/// Render album art as Unicode block art (fallback for terminals without graphics support).
pub fn render_block_art(bytes: &[u8], char_w: u16, char_h: u16) -> Option<BlockArt> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(image_to_block_art(&img, char_w, char_h))
}

/// Render album art from a DynamicImage as Unicode block art.
/// Use this when you already have the image loaded and want to generate block art at a specific size.
pub fn render_block_art_from_image(img: &DynamicImage, char_w: u16, char_h: u16) -> BlockArt {
    image_to_block_art(img, char_w, char_h)
}

/// Create a Picker for detecting terminal graphics capabilities.
/// This should be called once at application startup.
pub fn create_picker() -> Option<Picker> {
    Picker::from_query_stdio().ok()
}

fn image_to_block_art(img: &DynamicImage, char_w: u16, char_h: u16) -> BlockArt {
    // Each char cell is 2 pixel rows tall
    let px_w = char_w as u32;
    let px_h = char_h as u32 * 2;

    let resized = img.resize_exact(px_w, px_h, FilterType::Lanczos3);

    let mut rows: Vec<Line<'static>> = Vec::with_capacity(char_h as usize);

    for cell_row in 0..char_h as u32 {
        let top_y = cell_row * 2;
        let bottom_y = cell_row * 2 + 1;

        let mut spans: Vec<Span<'static>> = Vec::with_capacity(char_w as usize);

        for x in 0..char_w as u32 {
            let [r1, g1, b1, _] = resized.get_pixel(x, top_y).0;
            let [r2, g2, b2, _] = resized.get_pixel(x, bottom_y).0;

            let style = Style::default()
                .fg(Color::Rgb(r1, g1, b1)) // top pixel  → foreground
                .bg(Color::Rgb(r2, g2, b2)); // bottom pixel → background

            spans.push(Span::styled("▀", style));
        }

        rows.push(Line::from(spans));
    }

    BlockArt { rows }
}
