use crossterm::event::KeyCode;

#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let prev = self.prev_char_boundary();
                    self.text.drain(prev..self.cursor);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    let next = self.next_char_boundary();
                    self.text.drain(self.cursor..next);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor = self.prev_char_boundary();
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.text.len() {
                    self.cursor = self.next_char_boundary();
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.text.len();
                true
            }
            _ => false,
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns (before_cursor, char_at_cursor, after_cursor).
    /// `char_at_cursor` is empty when the cursor is at the end.
    pub fn split_at_cursor(&self) -> (&str, &str, &str) {
        let before = &self.text[..self.cursor];
        if self.cursor >= self.text.len() {
            (before, "", "")
        } else {
            let rest = &self.text[self.cursor..];
            let char_end = rest
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            (&self.text[..self.cursor], &rest[..char_end], &rest[char_end..])
        }
    }

    fn prev_char_boundary(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_char_boundary(&self) -> usize {
        self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.text.len())
    }
}
