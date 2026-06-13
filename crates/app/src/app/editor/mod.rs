//! Line-based text editor buffer: cursor, selection, undo/redo,
//! tab-aware column↔cell mapping. Rendering happens in the px view.

mod keys;
#[cfg(test)]
mod tests;
mod undo;

use undo::Op;

pub const TAB_W: usize = 4;

pub(super) type Pos = (usize, usize); // (row, col) — col in chars

pub struct Editor {
    pub lines: Vec<String>,
    pub cursor: Pos,
    pub anchor: Option<Pos>,
    pub scroll: usize,
    pub hscroll: usize,
    pub modified: bool,
    pub read_only: bool,
    trailing_newline: bool,
    uses_tabs: bool,
    undo: Vec<Op>,
    redo: Vec<Op>,
}

impl Editor {
    pub fn from_text(text: &str) -> Self {
        let trailing_newline = text.ends_with('\n');
        let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        if trailing_newline {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let uses_tabs = lines.iter().take(200).any(|l| l.starts_with('\t'));
        Editor {
            lines,
            cursor: (0, 0),
            anchor: None,
            scroll: 0,
            hscroll: 0,
            modified: false,
            read_only: true,
            trailing_newline,
            uses_tabs,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut s = self.lines.join("\n");
        if self.trailing_newline {
            s.push('\n');
        }
        s
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    // -- coordinate helpers ------------------------------------------------

    fn line_len(&self, row: usize) -> usize {
        self.lines[row].chars().count()
    }

    fn clamp(&self, pos: Pos) -> Pos {
        let row = pos.0.min(self.lines.len() - 1);
        (row, pos.1.min(self.line_len(row)))
    }

    /// Visual cell x for a char column (tabs expand to TAB_W).
    pub fn col_to_x(&self, row: usize, col: usize) -> usize {
        self.lines[row]
            .chars()
            .take(col)
            .map(|c| if c == '\t' { TAB_W } else { 1 })
            .sum()
    }

    /// Char column for a visual cell x.
    pub fn x_to_col(&self, row: usize, x: usize) -> usize {
        let mut cx = 0;
        for (i, c) in self.lines[row].chars().enumerate() {
            let w = if c == '\t' { TAB_W } else { 1 };
            if cx + w > x {
                return i;
            }
            cx += w;
        }
        self.line_len(row)
    }

    pub fn sel_range(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?;
        if a == self.cursor {
            return None;
        }
        Some(if a < self.cursor { (a, self.cursor) } else { (self.cursor, a) })
    }

    /// The selected text, for the system clipboard.
    pub fn selection_text(&self) -> Option<String> {
        let (a, b) = self.sel_range()?;
        let mut out = String::new();
        for row in a.0..=b.0 {
            // Defensive: a stale selection past EOF degrades to no-copy, not a panic.
            let chars: Vec<char> = self.lines.get(row)?.chars().collect();
            let c0 = if row == a.0 { a.1.min(chars.len()) } else { 0 };
            let c1 = if row == b.0 { b.1.min(chars.len()) } else { chars.len() };
            out.extend(&chars[c0..c1]);
            if row != b.0 {
                out.push('\n');
            }
        }
        Some(out)
    }

    pub fn move_to(&mut self, pos: Pos, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = self.clamp(pos);
    }

    /// Adjust scroll so the cursor is visible in a view_h × view_w window.
    pub fn ensure_visible(&mut self, view_h: usize, view_w: usize) {
        if self.cursor.0 < self.scroll {
            self.scroll = self.cursor.0;
        }
        if view_h > 0 && self.cursor.0 >= self.scroll + view_h {
            self.scroll = self.cursor.0 - view_h + 1;
        }
        let x = self.col_to_x(self.cursor.0, self.cursor.1);
        if x < self.hscroll {
            self.hscroll = x;
        }
        if view_w > 0 && x >= self.hscroll + view_w {
            self.hscroll = x - view_w + 1;
        }
    }

    pub fn scroll_by(&mut self, dy: i32, view_h: usize) {
        let max = self.lines.len().saturating_sub(view_h.max(1));
        self.scroll = (self.scroll as i64 + dy as i64).clamp(0, max as i64) as usize;
    }
}
