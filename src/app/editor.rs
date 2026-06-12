//! Line-based text editor buffer: cursor, selection, undo/redo,
//! tab-aware column↔cell mapping. Rendering happens in view.rs.

use crate::ui::input::{Key, Mods};

pub const TAB_W: usize = 4;

type Pos = (usize, usize); // (row, col) — col in chars

enum Op {
    /// Undo of an insert: delete this range.
    DeleteRange(Pos, Pos),
    /// Undo of a delete: re-insert text at pos.
    InsertAt(Pos, String),
}

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
            let chars: Vec<char> = self.lines[row].chars().collect();
            let c0 = if row == a.0 { a.1.min(chars.len()) } else { 0 };
            let c1 = if row == b.0 { b.1.min(chars.len()) } else { chars.len() };
            out.extend(&chars[c0..c1]);
            if row != b.0 {
                out.push('\n');
            }
        }
        Some(out)
    }

    // -- primitive edits (undo-recorded) -----------------------------------

    fn byte_idx(line: &str, col: usize) -> usize {
        line.char_indices().nth(col).map(|(b, _)| b).unwrap_or(line.len())
    }

    fn raw_insert(&mut self, at: Pos, text: &str) -> Pos {
        let (row, col) = at;
        let b = Self::byte_idx(&self.lines[row], col);
        if !text.contains('\n') {
            self.lines[row].insert_str(b, text);
            return (row, col + text.chars().count());
        }
        let tail = self.lines[row].split_off(b);
        let mut segs = text.split('\n');
        let first = segs.next().unwrap_or("");
        self.lines[row].push_str(first);
        let mut insert_row = row + 1;
        let mut last_end = (row, col + first.chars().count());
        for seg in segs {
            self.lines.insert(insert_row, seg.to_string());
            last_end = (insert_row, seg.chars().count());
            insert_row += 1;
        }
        let (lr, lc) = last_end;
        self.lines[lr].push_str(&tail);
        (lr, lc)
    }

    fn raw_delete(&mut self, a: Pos, b: Pos) -> String {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        if a.0 == b.0 {
            let ba = Self::byte_idx(&self.lines[a.0], a.1);
            let bb = Self::byte_idx(&self.lines[a.0], b.1);
            let removed: String = self.lines[a.0][ba..bb].to_string();
            self.lines[a.0].replace_range(ba..bb, "");
            return removed;
        }
        let ba = Self::byte_idx(&self.lines[a.0], a.1);
        let bb = Self::byte_idx(&self.lines[b.0], b.1);
        let mut removed = self.lines[a.0][ba..].to_string();
        for row in a.0 + 1..b.0 {
            removed.push('\n');
            removed.push_str(&self.lines[row]);
        }
        removed.push('\n');
        removed.push_str(&self.lines[b.0][..bb]);
        let tail = self.lines[b.0][bb..].to_string();
        self.lines[a.0].truncate(ba);
        self.lines[a.0].push_str(&tail);
        self.lines.drain(a.0 + 1..=b.0);
        removed
    }

    fn record_insert(&mut self, at: Pos, end: Pos, coalesce: bool) {
        if coalesce {
            if let Some(Op::DeleteRange(_, b)) = self.undo.last_mut() {
                if *b == at && at.0 == end.0 {
                    *b = end;
                    return;
                }
            }
        }
        self.undo.push(Op::DeleteRange(at, end));
    }

    fn do_insert(&mut self, text: &str) {
        if let Some((a, b)) = self.sel_range() {
            let removed = self.raw_delete(a, b);
            self.undo.push(Op::InsertAt(a, removed));
            self.cursor = a;
            self.anchor = None;
        }
        let at = self.cursor;
        let end = self.raw_insert(at, text);
        let single = text.chars().count() == 1 && !text.contains('\n');
        self.record_insert(at, end, single);
        self.cursor = end;
        self.modified = true;
        self.redo.clear();
    }

    fn do_delete(&mut self, a: Pos, b: Pos) {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        if a == b {
            return;
        }
        let removed = self.raw_delete(a, b);
        self.undo.push(Op::InsertAt(a, removed));
        self.cursor = a;
        self.anchor = None;
        self.modified = true;
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some(op) = self.undo.pop() {
            let inverse = self.apply(op);
            self.redo.push(inverse);
            self.modified = true;
            self.anchor = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo.pop() {
            let inverse = self.apply(op);
            self.undo.push(inverse);
            self.modified = true;
            self.anchor = None;
        }
    }

    fn apply(&mut self, op: Op) -> Op {
        match op {
            Op::DeleteRange(a, b) => {
                let removed = self.raw_delete(a, b);
                self.cursor = a;
                Op::InsertAt(a, removed)
            }
            Op::InsertAt(at, text) => {
                let end = self.raw_insert(at, &text);
                self.cursor = end;
                Op::DeleteRange(at, end)
            }
        }
    }

    // -- key handling --------------------------------------------------------

    /// Returns true when the buffer or cursor changed.
    pub fn handle_key(&mut self, key: &Key, mods: Mods, view_h: usize) -> bool {
        let select = mods.shift;
        match key {
            Key::Char(c) if mods.ctrl => match c {
                'z' => {
                    if !self.read_only {
                        self.undo();
                    }
                    true
                }
                'y' => {
                    if !self.read_only {
                        self.redo();
                    }
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
