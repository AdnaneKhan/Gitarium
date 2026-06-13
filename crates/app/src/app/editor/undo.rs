//! Primitive edits and the involutive undo/redo stack: applying an Op
//! mutates the buffer and returns its inverse.

use super::{Editor, Pos};

pub(super) enum Op {
    /// Undo of an insert: delete this range.
    DeleteRange(Pos, Pos),
    /// Undo of a delete: re-insert text at pos.
    InsertAt(Pos, String),
    /// One atomic undo step (e.g. replace-selection); applied last-to-first.
    Group(Vec<Op>),
}

impl Editor {
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

    pub(super) fn do_insert(&mut self, text: &str) {
        let mut replaced = None;
        if let Some((a, b)) = self.sel_range() {
            let removed = self.raw_delete(a, b);
            replaced = Some(Op::InsertAt(a, removed));
            self.cursor = a;
            self.anchor = None;
        }
        let at = self.cursor;
        let end = self.raw_insert(at, text);
        match replaced {
            // Replacing a selection is one atomic undo step: delete + insert.
            Some(restore) => self.undo.push(Op::Group(vec![restore, Op::DeleteRange(at, end)])),
            None => {
                let single = text.chars().count() == 1 && !text.contains('\n');
                self.record_insert(at, end, single);
            }
        }
        self.cursor = end;
        self.modified = true;
        self.redo.clear();
    }

    pub(super) fn do_delete(&mut self, a: Pos, b: Pos) {
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
            Op::Group(ops) => {
                let inv: Vec<Op> = ops.into_iter().rev().map(|op| self.apply(op)).collect();
                Op::Group(inv)
            }
        }
    }
}
