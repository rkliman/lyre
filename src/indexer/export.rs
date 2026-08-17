use crate::util::expand_tilde;
use std::path::Path;

use super::util::{open_db, query_triples, write_csv_row};

pub(super) fn export_tracks(db_path: &str) {
    let conn = open_db(db_path);
    let expanded = expand_tilde(db_path);
    let results = query_triples(&conn, "SELECT artist, album, title FROM tracks", &[]);

    let folder = Path::new(&expanded)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let csv_path = folder.join("tracks_export.csv");
    let mut file = std::fs::File::create(&csv_path).expect("Failed to create CSV file");
    write_csv_row(&mut file, &["Artist", "Album", "Title"]).expect("write header");
    for (artist, album, title) in results {
        write_csv_row(&mut file, &[&artist, &album, &title]).expect("write row");
    }
    println!("Exported to {}", csv_path.display());
}
