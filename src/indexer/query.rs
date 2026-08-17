use std::collections::HashMap;

use super::util::{open_db, query_triples, Colorize};

fn search_by_column(
    conn: &rusqlite::Connection,
    order_column: &str,
    filter_column: &str,
    pattern: Option<&str>,
) -> Vec<(String, String, String)> {
    match pattern {
        Some(p) => {
            let sql = format!(
                "SELECT artist, album, title FROM tracks \
                 WHERE {} LIKE ?1 ORDER BY {}, artist, album, title",
                filter_column, order_column
            );
            query_triples(conn, &sql, &[&format!("%{}%", p)])
        }
        None => {
            let sql = format!(
                "SELECT artist, album, title FROM tracks ORDER BY {}, artist, album, title",
                order_column
            );
            query_triples(conn, &sql, &[])
        }
    }
}

pub(super) fn search_tracks(db_path: &str, query: Option<String>) {
    let conn = open_db(db_path);
    let q = query.as_deref();

    let sections: [(&str, &str, usize); 3] = [
        ("Tracks", "title", 2),
        ("Albums", "album", 1),
        ("Artists", "artist", 0),
    ];

    for (i, (heading, column, idx)) in sections.iter().enumerate() {
        if i == 0 {
            println!("{}", "Tracks (Track - Album - Artist)".bold().underline());
        } else {
            println!("\n{}", heading.bold().underline());
        }

        let results = search_by_column(&conn, column, column, q);
        if results.is_empty() {
            println!("{}", format!("No {} found.", heading.to_lowercase()).yellow());
            continue;
        }

        if *idx == 2 {
            for (artist, album, title) in results {
                println!("{} - {} - {}", title, album, artist);
            }
        } else {
            let mut unique: Vec<String> = results
                .into_iter()
                .map(|t| if *idx == 1 { t.1 } else { t.0 })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            unique.sort();
            for v in unique {
                println!("{}", v);
            }
        }
    }
}

fn print_grouped_tracks(results: Vec<(String, String, String)>) {
    if results.is_empty() {
        println!("{}", "No tracks found.".yellow());
        return;
    }
    let mut last_artist = String::new();
    let mut last_album = String::new();
    for (artist, album, title) in results {
        if artist != last_artist {
            println!("\n{}:", artist.bold());
            last_artist = artist;
            last_album.clear();
        }
        if album != last_album {
            println!("  {}:", album.cyan());
            last_album = album;
        }
        println!("    {}", title);
    }
}

pub(super) fn list_tracks(db_path: &str, query: Option<String>, genre: Option<String>) {
    let conn = open_db(db_path);
    if let Some(ref g) = genre {
        println!("{} {}", "Genre:".bold(), g.cyan());
    }

    let mut clauses: Vec<&str> = Vec::new();
    let genre_pattern = genre.as_ref().map(|g| format!("%{}%", g));
    let query_pattern = query.as_ref().map(|q| format!("%{}%", q));

    if genre_pattern.is_some() {
        clauses.push("genre LIKE ?1");
    }
    if query_pattern.is_some() {
        clauses.push(if genre_pattern.is_some() {
            "(album LIKE ?2 OR artist LIKE ?2 OR title LIKE ?2)"
        } else {
            "(album LIKE ?1 OR artist LIKE ?1 OR title LIKE ?1)"
        });
    }

    let sql = if clauses.is_empty() {
        "SELECT artist, album, title FROM tracks ORDER BY artist, album, title".to_string()
    } else {
        format!(
            "SELECT artist, album, title FROM tracks WHERE {} ORDER BY artist, album, title",
            clauses.join(" AND ")
        )
    };

    let results = match (&genre_pattern, &query_pattern) {
        (Some(g), Some(q)) => query_triples(&conn, &sql, &[g, q]),
        (Some(g), None) => query_triples(&conn, &sql, &[g]),
        (None, Some(q)) => query_triples(&conn, &sql, &[q]),
        (None, None) => query_triples(&conn, &sql, &[]),
    };

    print_grouped_tracks(results);
}

pub(super) fn list_genres(db_path: &str) {
    let conn = open_db(db_path);
    let mut stmt = conn
        .prepare("SELECT genre FROM tracks WHERE genre != ''")
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut counts: HashMap<String, usize> = HashMap::new();
    while let Some(row) = rows.next().expect("row") {
        let genre_str: String = row.get(0).unwrap_or_default();
        for g in genre_str.split(',') {
            let g = g.trim().to_string();
            if !g.is_empty() {
                *counts.entry(g).or_insert(0) += 1;
            }
        }
    }
    if counts.is_empty() {
        println!("{}", "No genres found.".yellow());
        return;
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (genre, count) in sorted {
        println!("{:<30} {}", genre.bold(), format!("({} tracks)", count).yellow());
    }
}
