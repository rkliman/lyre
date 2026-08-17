use serde::Deserialize;

/// What we resolved for a single track after querying lrclib (with fallback).
/// Used by both the CLI (`lyre lyrics`) and the in-app fetch prompt so their
/// behavior stays in sync.
#[derive(Debug, Clone)]
pub enum LyricsResolution {
    Synced(String),
    Plain(String),
    Instrumental,
}

impl LyricsResolution {
    /// The exact string to write into the file's `LYRICS` tag.
    pub fn tag_text(self) -> String {
        match self {
            LyricsResolution::Synced(s) | LyricsResolution::Plain(s) => s,
            LyricsResolution::Instrumental => "[Instrumental]".to_string(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            LyricsResolution::Synced(_) => "synced",
            LyricsResolution::Plain(_) => "plain",
            LyricsResolution::Instrumental => "instrumental",
        }
    }
}

/// Resolve lyrics for a single track: query `/api/get`, then fall back to
/// `/api/search` if the primary hit didn't yield synced lyrics (and wasn't
/// flagged instrumental). Returns `None` only when no matching entry exists.
pub async fn resolve_lyrics(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
    album: Option<&str>,
    duration_secs: Option<u32>,
) -> Result<Option<LyricsResolution>, reqwest::Error> {
    let mut result = fetch_lyrics(client, artist, title, album, duration_secs).await?;

    let needs_fallback = match &result {
        None => true,
        Some(lrc) if lrc.instrumental => false,
        Some(lrc) => lrc.synced_lyrics.as_deref().is_none_or(|s| s.trim().is_empty()),
    };

    if needs_fallback {
        if let Some(found) = search_lyrics(client, artist, title).await? {
            result = Some(found);
        }
    }

    Ok(result.map(|lrc| {
        let synced = lrc.synced_lyrics.filter(|s| !s.trim().is_empty());
        let plain = lrc.plain_lyrics.filter(|s| !s.trim().is_empty());
        if let Some(s) = synced {
            LyricsResolution::Synced(s)
        } else if let Some(p) = plain {
            LyricsResolution::Plain(p)
        } else {
            LyricsResolution::Instrumental
        }
    }))
}

#[derive(Deserialize, Debug)]
pub struct LrcLibResult {
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(default)]
    pub instrumental: bool,
}

/// Retry an HTTP request up to `max_attempts` times when the server returns a
/// transient failure (429 / 5xx). Sleeps with exponential backoff between tries.
async fn send_with_retry(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let resp = client.get(url).query(query).send().await?;
        let status = resp.status();
        let transient = status.as_u16() == 429 || status.is_server_error();
        if !transient || attempt >= 5 {
            return Ok(resp);
        }
        let backoff = std::time::Duration::from_millis(500u64 << attempt.min(6));
        tokio::time::sleep(backoff).await;
    }
}

pub async fn fetch_lyrics(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
    album: Option<&str>,
    duration_secs: Option<u32>,
) -> Result<Option<LrcLibResult>, reqwest::Error> {
    let mut query = vec![
        ("artist_name".to_string(), artist.to_string()),
        ("track_name".to_string(), title.to_string()),
    ];
    if let Some(a) = album {
        query.push(("album_name".to_string(), a.to_string()));
    }
    if let Some(d) = duration_secs {
        query.push(("duration".to_string(), d.to_string()));
    }
    let resp = send_with_retry(client, "https://lrclib.net/api/get", &query).await?;
    if resp.status().is_success() {
        Ok(Some(resp.json::<LrcLibResult>().await?))
    } else {
        // 404 => genuinely not in lrclib; other statuses after retry => give up as None
        Ok(None)
    }
}

pub async fn search_lyrics(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Result<Option<LrcLibResult>, reqwest::Error> {
    let query = vec![
        ("artist_name".to_string(), artist.to_string()),
        ("track_name".to_string(), title.to_string()),
    ];
    let resp = send_with_retry(client, "https://lrclib.net/api/search", &query).await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let results: Vec<LrcLibResult> = resp.json().await?;
    if results.is_empty() {
        return Ok(None);
    }
    let best = results.into_iter().max_by(|a, b| {
        let a_synced = a.synced_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty());
        let b_synced = b.synced_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty());
        let score_a = strsim::jaro(&a.artist_name, artist) + strsim::jaro(&a.track_name, title);
        let score_b = strsim::jaro(&b.artist_name, artist) + strsim::jaro(&b.track_name, title);
        (a_synced, score_a)
            .partial_cmp(&(b_synced, score_b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(best)
}
