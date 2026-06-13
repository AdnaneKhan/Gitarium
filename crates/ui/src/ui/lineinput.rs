//! Single-line text input model used for token entry, filters, commit
//! messages. Rendering happens in the px view; this is just the editing
//! logic.

use super::input::{Key, Mods};

#[derive(Default)]
pub struct LineInput {
    pub text: String,
    pub cursor: usize, // char index
    pub masked: bool,
}

impl LineInput {
    pub fn new(masked: bool) -> Self {
        LineInput { text: String::new(), cursor: 0, masked }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Copy for renderers that need the value while the app stays borrowed.
    pub fn clone_shallow(&self) -> LineInput {
        LineInput { text: self.text.clone(), cursor: self.cursor, masked: self.masked }
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.text.len())
    }

    pub fn insert(&mut self, s: &str) {
        let b = self.byte_at(self.cursor);
        self.text.insert_str(b, s);
        self.cursor += s.chars().count();
    }

    /// Returns true if the key was consumed.
    pub fn handle_key(&mut self, key: &Key, mods: Mods) -> bool {
        match key {
            Key::Char(c) if !mods.ctrl && !mods.alt => {
                self.insert(&c.to_string());
                true
            }
            Key::Char('u') if mods.ctrl => {
                self.clear();
                true
            }
            Key::Backspace => {
                if self.cursor > 0 {
                    let b0 = self.byte_at(self.cursor - 1);
                    let b1 = self.byte_at(self.cursor);
                    self.text.replace_range(b0..b1, "");
                    self.cursor -= 1;
                }
                true
            }
            Key::Delete => {
                let count = self.text.chars().count();
                if self.cursor < count {
                    let b0 = self.byte_at(self.cursor);
                    let b1 = self.byte_at(self.cursor + 1);
                    self.text.replace_range(b0..b1, "");
                }
                true
            }
            Key::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            Key::Right => {
                self.cursor = (self.cursor + 1).min(self.text.chars().count());
                true
            }
            Key::Home => {
                self.cursor = 0;
                true
            }
            Key::End => {
                self.cursor = self.text.chars().count();
                true
            }
            _ => false,
        }
    }
}
