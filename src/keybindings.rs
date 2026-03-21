use crossterm::event::KeyCode;
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
    pub description: &'static str,
    pub action: Action,
    pub context: Context,
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
    ($keys:expr, $desc:expr, $action:expr, $ctx:expr) => {
        Keybinding {
            keys: $keys,
            description: $desc,
            action: $action,
            context: $ctx,
        }
    };
}

use KeyCode::*;
use Context::*;

impl Keybindings {
    pub fn new() -> Self {
        let all = vec![
            // Navigation
            kb!(vec![Tab], "Cycle panels", Action::CycleForward, Global),
            kb!(vec![BackTab], "Cycle panels", Action::CycleBackward, Global),
            kb!(vec![Char('1')], "Jump to sidebar / queue / tracks", Action::JumpToSidebar, Global),
            kb!(vec![Char('2')], "Jump to sidebar / queue / tracks", Action::JumpToQueue, Global),
            kb!(vec![Char('3')], "Jump to sidebar / queue / tracks", Action::JumpToTracks, Global),
            kb!(vec![Char('4'), Char('A')], "Toggle album art window in sidebar", Action::ToggleArtWindow, Global),
            kb!(vec![Char('5'), Char('L')], "Toggle lyrics panel", Action::ToggleLyrics, Global),
            kb!(vec![Char('j'), Down], "Move up / down", Action::MoveDown, Global),
            kb!(vec![Char('k'), Up], "Move up / down", Action::MoveUp, Global),
            kb!(vec![Char('g'), Home], "Go to top / bottom", Action::GoToTop, Global),
            kb!(vec![Char('G'), End], "Go to top / bottom", Action::GoToBottom, Global),
            kb!(vec![Char('u'), PageUp], "Page up / down (all panels)", Action::PageUp, Global),
            kb!(vec![Char('d'), PageDown], "Page up / down (all panels)", Action::PageDown, Global),
            kb!(vec![Char('h'), Left], "Switch panels", Action::MoveLeft, Global),
            kb!(vec![Char('l'), Right], "Switch panels", Action::MoveRight, Global),

            // Playback
            kb!(vec![Enter, Char(' ')], "Play selected / toggle pause", Action::PlayPause, Global),
            kb!(vec![Char('n')], "Next track in queue", Action::Next, Global),
            kb!(vec![Char('p')], "Previous track in queue", Action::Previous, Global),
            kb!(vec![Char('s')], "Stop", Action::Stop, Global),
            kb!(vec![Char('+'), Char('=')], "Volume up / down", Action::VolumeUp, Global),
            kb!(vec![Char('-')], "Volume up / down", Action::VolumeDown, Global),
            kb!(vec![Char('.')], "Seek forward / backward (5 seconds)", Action::SeekForward, Global),
            kb!(vec![Char(',')], "Seek forward / backward (5 seconds)", Action::SeekBackward, Global),
            kb!(vec![Char('b')], "Toggle shuffle", Action::ToggleShuffle, Global),
            kb!(vec![Char('o')], "Toggle loop (off → all → one)", Action::ToggleLoop, Global),

            // Queue
            kb!(vec![Char('a')], "Add selected track to queue", Action::AddToQueue, Panel(Panel::TrackList)),
            kb!(vec![Char('A')], "Add all visible tracks to queue", Action::AddAllToQueue, Panel(Panel::TrackList)),
            kb!(vec![Char('x'), Delete], "Remove selected from queue", Action::RemoveFromQueue, Panel(Panel::Queue)),
            kb!(vec![Char('c')], "Clear entire queue", Action::ClearQueue, Panel(Panel::Queue)),

            // Library
            kb!(vec![Char('S')], "Cycle sort field (title→artist→album→year→genre→dur)", Action::CycleSort, Global),
            kb!(vec![Char('R')], "Toggle sort order (asc / desc)", Action::ToggleSortOrder, Global),
            kb!(vec![Char('/')], "Activate search bar (always visible above track list)", Action::EnterSearch, Global),

            // Playlists
            kb!(vec![Char('N')], "Create a new empty playlist", Action::NewPlaylist, Panel(Panel::Sidebar)),
            kb!(vec![Char('P')], "Add track to an existing playlist", Action::AddToPlaylist, Global),
            kb!(vec![Char('x'), Delete], "Remove track from playlist (saves immediately)", Action::RemoveFromPlaylist, Panel(Panel::TrackList)),
            kb!(vec![Char('K')], "Move selected track up", Action::MoveTrackUp, Panel(Panel::TrackList)),
            kb!(vec![Char('J')], "Move selected track down", Action::MoveTrackDown, Panel(Panel::TrackList)),

            // Other
            kb!(vec![Char('?')], "Toggle this help overlay", Action::ToggleHelp, Global),
            kb!(vec![Char('q'), Char('Q')], "Quit", Action::Quit, Global),
            kb!(vec![Char('r')], "Reload lyrics", Action::LyricsReload, Panel(Panel::Lyrics)),

            // Search mode
            kb!(vec![Esc], "Exit search", Action::SearchExit, Search),
            kb!(vec![Enter], "Confirm search", Action::SearchConfirm, Search),
            kb!(vec![Backspace], "Delete character", Action::SearchBackspace, Search),

            // Help overlay
            kb!(vec![Esc, Char('?')], "Close help", Action::HelpClose, Help),
            kb!(vec![Up, Char('k')], "Scroll up", Action::HelpScroll(-1), Help),
            kb!(vec![Down, Char('j')], "Scroll down", Action::HelpScroll(1), Help),
            kb!(vec![PageUp, Char('u')], "Scroll up (page)", Action::HelpScroll(-10), Help),
            kb!(vec![PageDown, Char('d')], "Scroll down (page)", Action::HelpScroll(10), Help),
            kb!(vec![Char('g')], "Go to top", Action::GoToTop, Help),
            kb!(vec![Char('G')], "Go to bottom", Action::GoToBottom, Help),

            // Overlay mode
            kb!(vec![Esc], "Cancel overlay", Action::OverlayCancel, Overlay),
            kb!(vec![Enter], "Confirm overlay", Action::OverlayConfirm, Overlay),
            kb!(vec![Backspace], "Delete character", Action::OverlayBackspace, Overlay),
            kb!(vec![Up, Char('k')], "Navigate up", Action::OverlayNavigate(-1), Overlay),
            kb!(vec![Down, Char('j')], "Navigate down", Action::OverlayNavigate(1), Overlay),
        ];

        Self { all }
    }

    /// Look up an action for a given key in a specific context
    pub fn lookup(&self, key: KeyCode, panel: Panel, in_search: bool, in_help: bool, in_overlay: bool) -> Option<Action> {
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

        // Try panel-specific first
        if let Some(action) = self.all.iter()
            .find(|kb| kb.context == Context::Panel(panel) && kb.keys.contains(&key))
            .map(|kb| kb.action.clone())
        {
            return Some(action);
        }

        // Fall back to global
        self.all.iter()
            .find(|kb| kb.context == Context::Global && kb.keys.contains(&key))
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
                (ks(&[JumpToSidebar, JumpToQueue, JumpToTracks]), "Jump to sidebar / queue / tracks"),
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
                (k(AddToQueue), "Add selected track to queue"),
                (k(AddAllToQueue), "Add all visible tracks to queue"),
                (k(RemoveFromQueue), "Remove selected from queue"),
                (k(ClearQueue), "Clear entire queue"),
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
            Char(' ') => "⎵".into(),
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
