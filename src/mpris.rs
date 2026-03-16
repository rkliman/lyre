/// MPRIS2 D-Bus integration.
///
/// Registers `org.mpris.MediaPlayer2.lyre` on the session bus.
/// A background task watches for state changes and emits
/// PropertiesChanged signals so GNOME/KDE pick up updates immediately.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use zbus::{connection, interface, zvariant::Value, zvariant::OwnedValue, SignalContext};

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MprisPlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl MprisPlaybackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MprisPlaybackStatus::Playing => "Playing",
            MprisPlaybackStatus::Paused  => "Paused",
            MprisPlaybackStatus::Stopped => "Stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MprisState {
    pub status:   MprisPlaybackStatus,
    pub title:    String,
    pub artist:   String,
    pub album:    String,
    pub length:   i64,   // µs
    pub position: i64,   // µs
    pub volume:   f64,
    pub art_url:  Option<String>, // file:// URI for album art
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            status:   MprisPlaybackStatus::Stopped,
            title:    String::new(),
            artist:   String::new(),
            album:    String::new(),
            length:   0,
            position: 0,
            volume:   1.0,
            art_url:  None,
        }
    }
}

// ── Commands (desktop → lyre) ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
}

// ── MediaPlayer2 (app-level) ──────────────────────────────────────────────────

struct MediaPlayer2;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MediaPlayer2 {
    fn raise(&self) {}
    fn quit(&self)  {}

    #[zbus(property)] fn can_quit(&self)   -> bool { false }
    #[zbus(property)] fn can_raise(&self)  -> bool { false }
    #[zbus(property)] fn has_track_list(&self) -> bool { false }
    #[zbus(property)] fn identity(&self)   -> &str { "lyre" }
    #[zbus(property)] fn supported_uri_schemes(&self) -> Vec<String> { vec!["file".into()] }
    #[zbus(property)] fn supported_mime_types(&self)  -> Vec<String> {
        vec!["audio/mpeg".into(), "audio/flac".into(), "audio/x-wav".into()]
    }
}

// ── MediaPlayer2.Player ───────────────────────────────────────────────────────

struct MediaPlayer2Player {
    // Latest state, kept in sync by the notify task
    state: Arc<Mutex<MprisState>>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<MprisCommand>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MediaPlayer2Player {
    async fn play(&self)       { let _ = self.cmd_tx.send(MprisCommand::Play); }
    async fn pause(&self)      { let _ = self.cmd_tx.send(MprisCommand::Pause); }
    async fn play_pause(&self) { let _ = self.cmd_tx.send(MprisCommand::PlayPause); }
    async fn stop(&self)       { let _ = self.cmd_tx.send(MprisCommand::Stop); }
    async fn next(&self)       { let _ = self.cmd_tx.send(MprisCommand::Next); }
    async fn previous(&self)   { let _ = self.cmd_tx.send(MprisCommand::Previous); }
    async fn open_uri(&self, _uri: String) {}

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.state.lock().unwrap().status.as_str().to_string()
    }

    #[zbus(property)] fn loop_status(&self)   -> String { "None".into() }
    #[zbus(property)] fn rate(&self)          -> f64    { 1.0 }
    #[zbus(property)] fn minimum_rate(&self)  -> f64    { 1.0 }
    #[zbus(property)] fn maximum_rate(&self)  -> f64    { 1.0 }
    #[zbus(property)] fn shuffle(&self)       -> bool   { false }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        build_metadata(&self.state.lock().unwrap())
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.state.lock().unwrap().volume
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.state.lock().unwrap().position
    }

    #[zbus(property)] fn can_go_next(&self)     -> bool { true }
    #[zbus(property)] fn can_go_previous(&self) -> bool { true }
    #[zbus(property)] fn can_play(&self)        -> bool { true }
    #[zbus(property)] fn can_pause(&self)       -> bool { true }
    #[zbus(property)] fn can_seek(&self)        -> bool { false }
    #[zbus(property)] fn can_control(&self)     -> bool { true }
}

// ── Metadata builder ──────────────────────────────────────────────────────────

fn build_metadata(s: &MprisState) -> HashMap<String, OwnedValue> {
    let mut m: HashMap<String, OwnedValue> = HashMap::new();

    // trackid is mandatory
    m.insert(
        "mpris:trackid".into(),
        OwnedValue::try_from(Value::new(
            zbus::zvariant::ObjectPath::try_from("/org/lyre/track/1").unwrap()
        )).unwrap(),
    );

    if s.length > 0 {
        m.insert("mpris:length".into(),
            OwnedValue::try_from(Value::new(s.length)).unwrap());
    }
    if !s.title.is_empty() {
        m.insert("xesam:title".into(),
            OwnedValue::try_from(Value::new(s.title.as_str())).unwrap());
    }
    if !s.artist.is_empty() {
        // xesam:artist is an array of strings
        m.insert("xesam:artist".into(),
            OwnedValue::try_from(Value::new(vec![s.artist.as_str()])).unwrap());
    }
    if !s.album.is_empty() {
        m.insert("xesam:album".into(),
            OwnedValue::try_from(Value::new(s.album.as_str())).unwrap());
    }
    if let Some(ref url) = s.art_url {
        m.insert("mpris:artUrl".into(),
            OwnedValue::try_from(Value::new(url.as_str())).unwrap());
    }
    m
}

// ── Public spawn ──────────────────────────────────────────────────────────────

pub async fn spawn() -> anyhow::Result<(
    watch::Sender<MprisState>,
    tokio::sync::mpsc::UnboundedReceiver<MprisCommand>,
)> {
    let (state_tx, mut state_rx) = watch::channel(MprisState::default());
    let (cmd_tx, cmd_rx)         = tokio::sync::mpsc::unbounded_channel();

    // Shared arc that the interface reads from and the notify task writes to
    let shared_state: Arc<Mutex<MprisState>> = Arc::new(Mutex::new(MprisState::default()));

    let player_iface = MediaPlayer2Player {
        state:  Arc::clone(&shared_state),
        cmd_tx: cmd_tx.clone(),
    };

    let conn = connection::Builder::session()?
        .name("org.mpris.MediaPlayer2.lyre")?
        .serve_at("/org/mpris/MediaPlayer2", MediaPlayer2)?
        .serve_at("/org/mpris/MediaPlayer2", player_iface)?
        .build()
        .await?;

    // Background task: watch for state changes and emit PropertiesChanged
    let conn2 = conn.clone();
    tokio::spawn(async move {
        loop {
            // Block until the sender pushes a new value
            if state_rx.changed().await.is_err() {
                break; // sender dropped, shut down
            }

            let new_state = state_rx.borrow().clone();

            // Update the shared arc so property getters return fresh data
            *shared_state.lock().unwrap() = new_state.clone();

            // Emit PropertiesChanged for Metadata, PlaybackStatus, Volume
            // zbus provides per-property notify helpers generated by #[interface]
            if let Ok(iface_ref) = conn2
                .object_server()
                .interface::<_, MediaPlayer2Player>("/org/mpris/MediaPlayer2")
                .await
            {
                let ctxt = iface_ref.signal_context().clone();
                let iface = iface_ref.get().await;

                // Notify all three properties that clients care about
                let _ = iface.metadata_changed(&ctxt).await;
                let _ = iface.playback_status_changed(&ctxt).await;
                let _ = iface.volume_changed(&ctxt).await;
            }
        }
        drop(conn2);
    });

    // Keep the original connection alive
    tokio::spawn(async move {
        std::future::pending::<()>().await;
        drop(conn);
    });

    Ok((state_tx, cmd_rx))
}