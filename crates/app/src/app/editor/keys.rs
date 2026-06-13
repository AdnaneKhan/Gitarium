//! Keyboard handling and paste insertion for the editor buffer.

use super::{Editor, Pos, TAB_W};
use crate::ui::input::{Key, Mods};

impl Editor {
    /// Returns true when the buffer or cursor changed.
    pub fn handle_key(&mut self, key: &Key, mods: Mods, view_h: usize) -> bool {
        let select = mods.shift;
        match key {
            Key::Char(c) if mods.ctrl => match c {
                'z' => {
                    if self.read_only {
                        return false;
                    }
                    self.undo();
                    true
                }
                'y' => {
                    if self.read_only {
                        return false;
                    }
                    self.redo();
                    true
                }
                _ => false,
            },
            Key::Char(c) if !mods.alt => {
                if self.read_only {
                    return false;
                }
                self.do_insert(&c.to_string());
                true
            }
            Key::Tab => {
                if self.read_only {
                    return false;
                }
                let indent = if self.uses_tabs { "\t".to_string() } else { " ".repeat(TAB_W) };
                self.do_insert(&indent);
                true
            }
            Key::Enter => {
                if self.read_only {
                    return false;
                }
                // Auto-indent: copy the current line's leading whitespace.
                let line = &self.lines[self.cursor.0];
                let ws: String = line
                    .chars()
                    .take(self.cursor.1)
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                self.do_insert(&format!("\n{}", ws));
                true
            }
            Key::Backspace => {
                if self.read_only {
                    return false;
                }
                if self.sel_range().is_some() {
                    let (a, b) = self.sel_range().unwrap();
                    self.do_delete(a, b);
                } else if self.cursor.1 > 0 {
                    self.do_delete((self.cursor.0, self.cursor.1 - 1), self.cursor);
                } else if self.cursor.0 > 0 {
                    let prev_len = self.line_len(self.cursor.0 - 1);
                    self.do_delete((self.cursor.0 - 1, prev_len), self.cursor);
                }
                true
            }
            Key::Delete => {
                if self.read_only {
                    return false;
                }
                if let Some((a, b)) = self.sel_range() {
                    self.do_delete(a, b);
                } else if self.cursor.1 < self.line_len(self.cursor.0) {
                    self.do_delete(self.cursor, (self.cursor.0, self.cursor.1 + 1));
                } else if self.cursor.0 + 1 < self.lines.len() {
                    self.do_delete(self.cursor, (self.cursor.0 + 1, 0));
                }
                true
            }
            Key::Left => {
                let pos = if mods.ctrl {
                    self.word_left()
                } else if self.cursor.1 > 0 {
                    (self.cursor.0, self.cursor.1 - 1)
                } else if self.cursor.0 > 0 {
                    (self.cursor.0 - 1, self.line_len(self.cursor.0 - 1))
                } else {
                    self.cursor
                };
                self.move_to(pos, select);
                true
            }
            Key::Right => {
                let pos = if mods.ctrl {
                    self.word_right()
                } else if self.cursor.1 < self.line_len(self.cursor.0) {
                    (self.cursor.0, self.cursor.1 + 1)
                } else if self.cursor.0 + 1 < self.lines.len() {
                    (self.cursor.0 + 1, 0)
                } else {
                    self.cursor
                };
                self.move_to(pos, select);
                true
            }
            Key::Up => {
                let pos = if self.cursor.0 > 0 { (self.cursor.0 - 1, self.cursor.1) } else { (0, 0) };
                self.move_to(pos, select);
                true
            }
            Key::Down => {
                let pos = if self.cursor.0 + 1 < self.lines.len() {
                    (self.cursor.0 + 1, self.cursor.1)
                } else {
                    (self.cursor.0, self.line_len(self.cursor.0))
                };
                self.move_to(pos, select);
                true
            }
            Key::Home => {
                let pos = if mods.ctrl { (0, 0) } else { (self.cursor.0, 0) };
                self.move_to(pos, select);
                true
            }
            Key::End => {
                let pos = if mods.ctrl {
                    (self.lines.len() - 1, self.line_len(self.lines.len() - 1))
                } else {
                    (self.cursor.0, self.line_len(self.cursor.0))
                };
                self.move_to(pos, select);
                true
            }
            Key::PageUp => {
                let row = self.cursor.0.saturating_sub(view_h.max(1));
                self.move_to((row, self.cursor.1), select);
                true
            }
            Key::PageDown => {
                let row = (self.cursor.0 + view_h.max(1)).min(self.lines.len() - 1);
                self.move_to((row, self.cursor.1), select);
                true
            }
            _ => false,
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        if self.read_only {
            return;
        }
        // Normalize newlines from pasted content.
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        self.do_insert(&text);
    }

    fn word_left(&self) -> Pos {
        let (row, mut col) = self.cursor;
        let chars: Vec<char> = self.lines[row].chars().collect();
        if col == 0 {
            return if row > 0 { (row - 1, self.line_len(row - 1)) } else { (0, 0) };
        }
        while col > 0 && !chars[col - 1].is_alphanumeric() {
            col -= 1;
        }
        while col > 0 && chars[col - 1].is_alphanumeric() {
            col -= 1;
        }
        (row, col)
    }

    fn word_right(&self) -> Pos {
        let (row, mut col) = self.cursor;
        let chars: Vec<char> = self.lines[row].chars().collect();
        let n = chars.len();
        if col >= n {
            return if row + 1 < self.lines.len() { (row + 1, 0) } else { (row, n) };
        }
        while col < n && !chars[col].is_alphanumeric() {
            col += 1;
        }
        while col < n && chars[col].is_alphanumeric() {
            col += 1;
        }
        (row, col)
    }
}
