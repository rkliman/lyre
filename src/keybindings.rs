use crossterm::event::{KeyCode, KeyModifiers};
use crate::types::Panel;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit, ToggleHelp, CycleForward, CycleBackward,
    JumpToSidebar, JumpToQueue, JumpToTracks,
    ToggleArtWindow, ToggleLyrics,

    PlayPause, Next, Previous, Stop,
    VolumeUp, VolumeDown, SeekForward, SeekBackward,
    ToggleShuffle, ToggleLoop,

    MoveUp, MoveDown, PageUp, PageDown,
    GoToTop, GoToBottom, MoveLeft, MoveRight, Enter,
    ExtendSelectionUp, ExtendSelectionDown,

    CycleSort, ToggleSortOrder, EnterSearch,

    AddToQueue, AddAllToQueue, RemoveFromQueue, ClearQueue,

    NewPlaylist, AddToPlaylist, RemoveFromPlaylist,
    MoveTrackUp, MoveTrackDown,

    SearchExit, SearchConfirm, SearchBackspace, SearchChar(char),
    HelpScroll(i32), HelpClose,
    LyricsReload,
    OverlayConfirm, OverlayCancel, OverlayChar(char),
    OverlayBackspace, OverlayNavigate(i32),
}

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub keys: Vec<KeyCode>,
    pub action: Action,
    pub context: Context,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Global,
    Panel(Panel),
    Search,
    Help,
    Overlay,
}

pub struct Keybindings {
    pub all: Vec<Keybinding>,
}

macro_rules! kb {
    ($keys:expr, $action:expr, $ctx:expr) => {
        Keybinding {
            keys: $keys,
            action: $action,
            context: $ctx,
            modifiers: KeyModifiers::NONE,
        }
    };
    ($keys:expr, $action:expr, $ctx:expr, $mods:expr) => {
        Keybinding {
            keys: $keys,
            action: $action,
            context: $ctx,
            modifiers: $mods,
        }
    };
}

use KeyCode::*;
use Context::*;

impl Keybindings {
    pub fn new() -> Self {
        let all = vec![
            // Navigation
            kb!(vec![Tab], Action::CycleForward, Global),
            kb!(vec![BackTab], Action::CycleBackward, Global),
            kb!(vec![Char('1')], Action::JumpToSidebar, Global),
            kb!(vec![Char('2')], Action::JumpToTracks, Global),
            kb!(vec![Char('3')], Action::JumpToQueue, Global),
            kb!(vec![Char('4')], Action::ToggleArtWindow, Global),
            kb!(vec![Char('L')], Action::ToggleLyrics, Global),
            kb!(vec![Char('j'), Down], Action::MoveDown, Global),
            kb!(vec![Char('k'), Up], Action::MoveUp, Global),
            kb!(vec![Up], Action::ExtendSelectionUp, Panel(Panel::TrackList), KeyModifiers::SHIFT),
            kb!(vec![Down], Action::ExtendSelectionDown, Panel(Panel::TrackList), KeyModifiers::SHIFT),
            kb!(vec![Char('g'), Home], Action::GoToTop, Global),
            kb!(vec![Char('G'), End], Action::GoToBottom, Global),
            kb!(vec![Char('u'), PageUp], Action::PageUp, Global),
            kb!(vec![Char('d'), PageDown], Action::PageDown, Global),
            kb!(vec![Char('h'), Left], Action::MoveLeft, Global),
            kb!(vec![Char('l'), Right], Action::MoveRight, Global),

            // Playback
            kb!(vec![Enter], Action::Enter, Global),
            kb!(vec![Char(' ')], Action::PlayPause, Global),
            kb!(vec![Char('n')], Action::Next, Global),
            kb!(vec![Char('p')], Action::Previous, Global),
            kb!(vec![Char('s')], Action::Stop, Global),
            kb!(vec![Char('+'), Char('=')], Action::VolumeUp, Global),
            kb!(vec![Char('-'), Char('_')], Action::VolumeDown, Global),
            kb!(vec![Char('.')], Action::SeekForward, Global),
            kb!(vec![Char(',')], Action::SeekBackward, Global),
            kb!(vec![Char('z')], Action::ToggleShuffle, Global),
            kb!(vec![Char('o')], Action::ToggleLoop, Global),

            // Queue
            kb!(vec![Char('a')], Action::AddToQueue, Panel(Panel::TrackList)),
            kb!(vec![Char('A')], Action::AddAllToQueue, Panel(Panel::TrackList)),
            kb!(vec![Char('x'), Delete], Action::RemoveFromQueue, Panel(Panel::Queue)),
            kb!(vec![Char('c')], Action::ClearQueue, Panel(Panel::Queue)),

            // Library
            kb!(vec![Char('S')], Action::CycleSort, Global),
            kb!(vec![Char('R')], Action::ToggleSortOrder, Global),
            kb!(vec![Char('/')], Action::EnterSearch, Global),

            // Playlists
            kb!(vec![Char('N')], Action::NewPlaylist, Panel(Panel::Sidebar)),
            kb!(vec![Char('P')], Action::AddToPlaylist, Global),
            kb!(vec![Char('x'), Delete], Action::RemoveFromPlaylist, Panel(Panel::TrackList)),
            kb!(vec![Char('K')], Action::MoveTrackUp, Panel(Panel::TrackList)),
            kb!(vec![Char('J')], Action::MoveTrackDown, Panel(Panel::TrackList)),

            // Other
            kb!(vec![Char('?')], Action::ToggleHelp, Global),
            kb!(vec![Char('q')], Action::Quit, Global),
            kb!(vec![Char('r')], Action::LyricsReload, Panel(Panel::Lyrics)),

            // Search mode
            kb!(vec![Esc], Action::SearchExit, Search),
            kb!(vec![Enter], Action::SearchConfirm, Search),
            kb!(vec![Backspace], Action::SearchBackspace, Search),

            // Help overlay
            kb!(vec![Esc, Char('?')], Action::HelpClose, Help),
            kb!(vec![Up, Char('k')], Action::HelpScroll(-1), Help),
            kb!(vec![Down, Char('j')], Action::HelpScroll(1), Help),
            kb!(vec![PageUp, Char('u')], Action::HelpScroll(-10), Help),
            kb!(vec![PageDown, Char('d')], Action::HelpScroll(10), Help),
            kb!(vec![Char('g')], Action::GoToTop, Help),
            kb!(vec![Char('G')], Action::GoToBottom, Help),

            // Overlay mode
            kb!(vec![Esc], Action::OverlayCancel, Overlay),
            kb!(vec![Enter], Action::OverlayConfirm, Overlay),
            kb!(vec![Backspace], Action::OverlayBackspace, Overlay),
            kb!(vec![Up, Char('k')], Action::OverlayNavigate(-1), Overlay),
            kb!(vec![Down, Char('j')], Action::OverlayNavigate(1), Overlay),
        ];

        Self { all }
    }

    /// Look up an action for a given key in a specific context
    pub fn lookup(&self, key: KeyCode, modifiers: KeyModifiers, panel: Panel, in_search: bool, in_help: bool, in_overlay: bool) -> Option<Action> {
        // Priority order: overlay > help > search > panel-specific > global

        if in_overlay {
            return self.lookup_overlay(key);
        }

        if in_help {
            return self.lookup_help(key);
        }

        if in_search {
            return self.lookup_search(key);
        }

        // Normalize modifiers: for character keys, ignore SHIFT since capital letters include it
        let normalized_modifiers = match key {
            KeyCode::Char(_) => modifiers & !KeyModifiers::SHIFT,
            _ => modifiers,
        };

        // Try panel-specific first (with modifier matching)
        if let Some(action) = self.all.iter()
            .find(|kb| {
                kb.context == Context::Panel(panel)
                && kb.keys.contains(&key)
                && kb.modifiers == normalized_modifiers
            })
            .map(|kb| kb.action.clone())
        {
            return Some(action);
        }

        // Fall back to global (only if no modifiers or modifiers match)
        self.all.iter()
            .find(|kb| {
                kb.context == Context::Global
                && kb.keys.contains(&key)
                && kb.modifiers == normalized_modifiers
            })
            .map(|kb| kb.action.clone())
    }

    fn lookup_search(&self, key: KeyCode) -> Option<Action> {
        // Check defined keybindings first
        if let Some(action) = self.all.iter()
            .find(|kb| kb.context == Context::Search && kb.keys.contains(&key))
            .map(|kb| kb.action.clone())
        {
            return Some(action);
        }
        // Fallback: any character is a SearchChar
        match key {
            Char(c) => Some(Action::SearchChar(c)),
            _ => None,
        }
    }

    fn lookup_help(&self, key: KeyCode) -> Option<Action> {
        self.all.iter()
            .find(|kb| kb.context == Context::Help && kb.keys.contains(&key))
            .map(|kb| kb.action.clone())
    }

    fn lookup_overlay(&self, key: KeyCode) -> Option<Action> {
        // Check defined keybindings first
        if let Some(action) = self.all.iter()
            .find(|kb| kb.context == Context::Overlay && kb.keys.contains(&key))
            .map(|kb| kb.action.clone())
        {
            return Some(action);
        }
        // Fallback: any character is an OverlayChar
        match key {
            Char(c) => Some(Action::OverlayChar(c)),
            _ => None,
        }
    }

    pub fn help_sections(&self) -> Vec<(&'static str, Vec<(String, &'static str)>)> {
        use Action::*;

        let k = |a: Action| self.keys_for_action(a);
        let ks = |actions: &[Action]| self.keys_for_actions(actions);
        let kctx = |a: Action, ctx: &str| format!("{}  {}", self.keys_for_action(a), ctx);

        vec![
            ("Navigation", vec![
                (ks(&[CycleForward, CycleBackward]), "Cycle panels"),
                (ks(&[JumpToSidebar, JumpToTracks, JumpToQueue]), "Jump to sidebar / tracks / queue"),
                (k(ToggleArtWindow), "Toggle album art window in sidebar"),
                (k(ToggleLyrics), "Toggle lyrics panel"),
                (ks(&[MoveUp, MoveDown]), "Move up / down"),
                (ks(&[GoToTop, GoToBottom]), "Go to top / bottom"),
                (ks(&[PageUp, PageDown]), "Page up / down (all panels)"),
                (ks(&[MoveLeft, MoveRight]), "Switch panels"),
                (kctx(MoveRight, "on section header"), "Expand  (or focus tracks if open)"),
                (kctx(MoveLeft, "on section header"), "Collapse"),
            ]),
            ("Playback", vec![
                (k(PlayPause), "Play selected / toggle pause"),
                (k(Next), "Next track in queue"),
                (k(Previous), "Previous track in queue"),
                (k(Stop), "Stop"),
                (ks(&[VolumeUp, VolumeDown]), "Volume up / down"),
                (ks(&[SeekForward, SeekBackward]), "Seek forward / backward (5 seconds)"),
                (k(ToggleShuffle), "Toggle shuffle"),
                (k(ToggleLoop), "Toggle loop (off → all → one)"),
            ]),
            ("Queue", vec![
                (k(AddToQueue), "Add selected track(s) to queue"),
                (k(AddAllToQueue), "Add all visible tracks to queue"),
                (k(RemoveFromQueue), "Remove selected from queue"),
                (k(ClearQueue), "Clear entire queue"),
                ("Shift+↑ / Shift+↓".to_string(), "Multi-select tracks (in track list)"),
            ]),
            ("Library", vec![
                (k(CycleSort), "Cycle sort field (title→artist→album→year→genre→dur)"),
                (k(ToggleSortOrder), "Toggle sort order (asc / desc)"),
                (k(EnterSearch), "Activate search bar (always visible above track list)"),
                (k(SearchExit), "Deactivate search bar  (results stay if query non-empty)"),
            ]),
            ("Playlists", vec![
                (kctx(NewPlaylist, "on Playlists header"), "Create a new empty playlist"),
                (k(AddToPlaylist), "Add track to an existing playlist"),
                (kctx(RemoveFromPlaylist, "in playlist"), "Remove track from playlist (saves immediately)"),
                (kctx(MoveTrackUp, "in playlist"), "Move selected track up"),
                (kctx(MoveTrackDown, "in playlist"), "Move selected track down"),
            ]),
            ("Other", vec![
                (k(ToggleHelp), "Toggle this help overlay"),
                (k(Quit), "Quit"),
            ]),
        ]
    }

    pub fn keys_for_action(&self, action: Action) -> String {
        self.all.iter()
            .find(|kb| kb.action == action)
            .map(|kb| format_keys(&kb.keys))
            .unwrap_or_default()
    }

    fn keys_for_actions(&self, actions: &[Action]) -> String {
        actions.iter()
            .filter_map(|a| self.all.iter().find(|kb| kb.action == *a).map(|kb| format_keys(&kb.keys)))
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

fn format_keys(keys: &[KeyCode]) -> String {
    keys.iter()
        .map(|k| match k {
            Char(' ') => "space".into(),
            Char(c) => c.to_string(),
            Up => "↑".into(), 
            Down => "↓".into(),
            Left => "←".into(), 
            Right => "→".into(),
            Tab => "Tab".into(), 
            BackTab => "Shift-Tab".into(),
            Enter => "Enter".into(), 
            Esc => "Esc".into(),
            Delete => "Del".into(), 
            Home => "Home".into(),
            End => "End".into(), 
            PageUp => "PgUp".into(),
            PageDown => "PgDn".into(), 
            Backspace => "Backspace".into(),
            _ => format!("{:?}", k),
        })
        .collect::<Vec<_>>()
        .join("  or  ")
}

/// Helper function to convert an action back to a representative KeyCode
/// This is used for the panel-specific handlers that still use KeyCode matching
pub fn key_to_code(action: Action) -> KeyCode {
    match action {
        Action::MoveUp => KeyCode::Up,
        Action::MoveDown => KeyCode::Down,
        Action::MoveLeft => KeyCode::Left,
        Action::MoveRight => KeyCode::Right,
        Action::PageUp => KeyCode::PageUp,
        Action::PageDown => KeyCode::PageDown,
        Action::GoToTop => KeyCode::Home,
        Action::GoToBottom => KeyCode::End,
        _ => KeyCode::Null,
    }
}
