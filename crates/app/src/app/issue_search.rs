//! In-page text search for the issue/PR detail view, mirroring the job-log
//! search: `/` opens a box, typing filters, `‹ › / Enter` step through
//! matches, Esc closes. Matching is over the rendered rows, so the view fills
//! `search.matches` (row indices) each frame; this owns the query + stepping.

use crate::ui::input::{Key, Mods};
use crate::ui::lineinput::LineInput;

use super::{App, LogSearch};

impl App {
    pub(super) fn open_detail_search(&mut self) {
        if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            if d.search.is_none() {
                d.search = Some(LogSearch { query: LineInput::new(false), matches: Vec::new(), idx: 0 });
            }
        }
    }

    pub(super) fn close_detail_search(&mut self) {
        if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            d.search = None;
        }
    }

    /// Step to the next (`+1`) / previous (`-1`) match and scroll to it. The
    /// match row indices are filled by the view; this just advances `idx`.
    pub(super) fn detail_search_step(&mut self, dir: i32) {
        let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) else { return };
        let line = {
            let Some(s) = d.search.as_mut() else { return };
            if s.matches.is_empty() {
                return;
            }
            let n = s.matches.len() as i32;
            s.idx = (((s.idx as i32 + dir) % n + n) % n) as usize;
            s.matches[s.idx]
        };
        d.scroll = line.saturating_sub(2);
    }

    /// Keys while the detail search box is open.
    pub(super) fn detail_search_key(&mut self, key: Key, mods: Mods) -> bool {
        match key {
            Key::Esc => self.close_detail_search(),
            Key::Enter => self.detail_search_step(if mods.shift { -1 } else { 1 }),
            Key::Up => self.detail_search_step(-1),
            Key::Down => self.detail_search_step(1),
            k => {
                // Re-matching (and jump-to-first on change) happens in the view,
                // which has the rendered rows; here we only edit the query.
                if let Some(s) = self
                    .rv
                    .as_mut()
                    .and_then(|rv| rv.detail.as_mut())
                    .and_then(|d| d.search.as_mut())
                {
                    s.query.handle_key(&k, mods);
                }
            }
        }
        true
    }
}
