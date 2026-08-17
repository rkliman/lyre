use lofty::file::{AudioFile, TaggedFileExt};
use std::path::Path;

use super::util::{abs_track_path, open_db, Colorize};

pub(super) fn get_info(db_path: &str, music_dir: &str, query: &str) {
    let conn = open_db(db_path);
    let pattern = format!("%{}%", query);
    let mut stmt = conn
        .prepare(
            "SELECT artist, album, title, path FROM tracks \
             WHERE artist LIKE ?1 OR album LIKE ?1 OR title LIKE ?1 \
             ORDER BY artist, album, title LIMIT 25",
        )
        .expect("prepare");
    let mut rows = stmt.query([&pattern]).expect("query");
    let mut results = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        results.push((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
        ));
    }

    if results.is_empty() {
        println!("{}", "No matching tracks found.".yellow());
        return;
    }

    let options: Vec<String> = results
        .iter()
        .map(|(artist, album, title, _)| format!("{} - {}  [{}]", artist, title, album))
        .collect();

    let selection = inquire::Select::new("Select a track to view info:", options.clone()).prompt();
    let selected_idx = match selection {
        Ok(sel) => options.iter().position(|o| o == &sel),
        Err(_) => None,
    };
    let Some(idx) = selected_idx else {
        println!("{}", "No track selected.".yellow());
        return;
    };

    let abs = abs_track_path(music_dir, &results[idx].3);
    print_track_info(&abs);
}

fn print_track_info(path_str: &str) {
    let path = Path::new(path_str);
    println!("\n{}", path_str.bold().underline());
    if !path.exists() {
        println!("{}", "File does not exist on disk.".red());
        return;
    }
    let tagged_file = match lofty::read_from_path(path) {
        Ok(f) => f,
        Err(e) => {
            println!("{}", format!("Failed to read file tags: {}", e).red());
            return;
        }
    };
    println!("{} {:?}", "File type:".bold(), tagged_file.file_type());
    let props = tagged_file.properties();
    println!("\n{}", "Audio properties:".bold().underline());
    println!("  {:<20} {:?}", "Duration:", props.duration());
    if let Some(br) = props.overall_bitrate() {
        println!("  {:<20} {} kbps", "Overall bitrate:", br);
    }
    if let Some(br) = props.audio_bitrate() {
        println!("  {:<20} {} kbps", "Audio bitrate:", br);
    }
    if let Some(sr) = props.sample_rate() {
        println!("  {:<20} {} Hz", "Sample rate:", sr);
    }
    if let Some(bd) = props.bit_depth() {
        println!("  {:<20} {}", "Bit depth:", bd);
    }
    if let Some(ch) = props.channels() {
        println!("  {:<20} {}", "Channels:", ch);
    }

    match tagged_file.primary_tag() {
        Some(tag) => {
            println!("\n{}", "Tags:".bold().underline());
            for item in tag.items() {
                let key = format!("{:?}", item.key());
                let val = match item.value() {
                    lofty::tag::ItemValue::Text(s) => s.clone(),
                    lofty::tag::ItemValue::Locator(s) => s.clone(),
                    lofty::tag::ItemValue::Binary(b) => {
                        format!("<binary, {} bytes>", b.len())
                    }
                };
                println!("  {:<25} {}", key.cyan(), val);
            }
            let pics = tag.pictures();
            if !pics.is_empty() {
                println!("\n{}", "Artwork:".bold().underline());
                for pic in pics {
                    println!(
                        "  {:?}  {:?}  ({} bytes)",
                        pic.pic_type(),
                        pic.mime_type(),
                        pic.data().len()
                    );
                }
            }
        }
        None => println!("\n{}", "No tags found on this file.".yellow()),
    }
}
