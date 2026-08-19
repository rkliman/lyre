use std::collections::HashSet;
use crossterm::event::KeyCode;

/// A list with a cursor index, viewport offset, and optional multi-selection state.
/// Used by the sidebar, track list, and global search panels.
pub struct NavigableList<T> {
    pub items: Vec<T>,
    pub index: usize,
    pub offset: usize,
    pub selected: HashSet<usize>,
    pub anchor: Option<usize>,
}

impl<T> Default for NavigableList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
            offset: 0,
            selected: HashSet::new(),
            anchor: None,
        }
    }
}

impl<T> NavigableList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn current(&self) -> Option<&T> {
        self.items.get(self.index)
    }

    /// Clamp index to valid range without changing offset or selection.
    /// Used when items are replaced but we want to preserve cursor position.
    pub fn clamp_index(&mut self) {
        if !self.items.is_empty() && self.index >= self.items.len() {
            self.index = self.items.len() - 1;
        }
    }

    /// Replace items, preserving cursor position (clamped).
    pub fn replace_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.clamp_index();
    }

    /// Replace items and reset cursor to 0. Clears selection.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.index = 0;
        self.offset = 0;
        self.selected.clear();
        self.anchor = None;
    }

    /// Wrapping navigation. Returns true if the key was consumed.
    /// Handles both arrow-style keys and vim-style character keys (j/k/u/d/g/G).
    pub fn navigate(&mut self, key: KeyCode) -> bool {
        self.navigate_with(key, true)
    }

    /// Like [`navigate`], but ignores vim-style character keys.
    /// Use this for lists that live next to a live text input, so that letters
    /// like `j` / `k` / `g` reach the input instead of being swallowed as navigation.
    pub fn navigate_arrows_only(&mut self, key: KeyCode) -> bool {
        self.navigate_with(key, false)
    }

    fn move_up(&mut self) {
        let len = self.items.len();
        if len > 0 {
            self.index = if self.index == 0 { len - 1 } else { self.index - 1 };
        }
    }

    fn move_down(&mut self) {
        let len = self.items.len();
        if len > 0 {
            self.index = if self.index + 1 >= len { 0 } else { self.index + 1 };
        }
    }

    fn page_up(&mut self) {
        self.index = self.index.saturating_sub(10);
    }

    fn page_down(&mut self) {
        self.index = (self.index + 10).min(self.items.len().saturating_sub(1));
    }

    fn navigate_with(&mut self, key: KeyCode, allow_vim_chars: bool) -> bool {
        match key {
            KeyCode::Up => { self.move_up(); true }
            KeyCode::Down => { self.move_down(); true }
            KeyCode::PageUp => { self.page_up(); true }
            KeyCode::PageDown => { self.page_down(); true }
            KeyCode::Home => { self.index = 0; true }
            KeyCode::End => { self.index = self.items.len().saturating_sub(1); true }
            KeyCode::Char('k') if allow_vim_chars => { self.move_up(); true }
            KeyCode::Char('j') if allow_vim_chars => { self.move_down(); true }
            KeyCode::Char('u') if allow_vim_chars => { self.page_up(); true }
            KeyCode::Char('d') if allow_vim_chars => { self.page_down(); true }
            KeyCode::Char('g') if allow_vim_chars => { self.index = 0; true }
            KeyCode::Char('G') if allow_vim_chars => { self.index = self.items.len().saturating_sub(1); true }
            _ => false,
        }
    }

    /// Update `offset` so `index` is inside a window of `visible_height` rows.
    /// Mirrors the "cursor near top / near bottom" pattern used across panels.
    pub fn ensure_visible(&mut self, visible_height: usize) {
        if visible_height == 0 || self.items.is_empty() {
            self.offset = 0;
            return;
        }
        if self.index < self.offset {
            self.offset = self.index;
        } else if self.index >= self.offset + visible_height {
            self.offset = self.index + 1 - visible_height;
        }
        let max_offset = self.items.len().saturating_sub(visible_height);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    /// Extend the selection upward by `step` rows.
    pub fn extend_up_by(&mut self, step: usize) {
        extend_range_selection(
            &mut self.index,
            self.items.len(),
            true,
            step,
            &mut self.selected,
            &mut self.anchor,
        );
    }

    /// Extend the selection downward by `step` rows.
    pub fn extend_down_by(&mut self, step: usize) {
        extend_range_selection(
            &mut self.index,
            self.items.len(),
            false,
            step,
            &mut self.selected,
            &mut self.anchor,
        );
    }

    pub fn selected_ascending(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.selected.iter().copied().collect();
        v.sort_unstable();
        v
    }

    pub fn selected_descending(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.selected.iter().copied().collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v
    }
}

/// Standalone wrapping navigation for indices not owned by a NavigableList (e.g. the queue).
pub fn navigate_index(key: KeyCode, index: &mut usize, len: usize) -> bool {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if len > 0 {
                *index = if *index == 0 { len - 1 } else { *index - 1 };
            }
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if len > 0 {
                *index = if *index + 1 >= len { 0 } else { *index + 1 };
            }
            true
        }
        KeyCode::PageUp | KeyCode::Char('u') => {
            *index = index.saturating_sub(10);
            true
        }
        KeyCode::PageDown | KeyCode::Char('d') => {
            *index = (*index + 10).min(len.saturating_sub(1));
            true
        }
        KeyCode::Home | KeyCode::Char('g') => {
            *index = 0;
            true
        }
        KeyCode::End | KeyCode::Char('G') => {
            *index = len.saturating_sub(1);
            true
        }
        _ => false,
    }
}

/// Shared anchor-based range-selection logic.
/// Used by NavigableList::extend_up/down and the queue (whose items live in Player).
pub fn extend_range_selection(
    index: &mut usize,
    len: usize,
    up: bool,
    step: usize,
    selected: &mut HashSet<usize>,
    anchor: &mut Option<usize>,
) {
    if len == 0 || step == 0 {
        return;
    }
    if anchor.is_none() {
        *anchor = Some(*index);
        selected.insert(*index);
    }
    if up {
        *index = index.saturating_sub(step);
    } else {
        *index = (*index + step).min(len - 1);
    }
    if let Some(a) = *anchor {
        selected.clear();
        let start = a.min(*index);
        let end = a.max(*index);
        for i in start..=end {
            selected.insert(i);
        }
    }
}
