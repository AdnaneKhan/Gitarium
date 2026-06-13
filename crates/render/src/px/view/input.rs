//! Host-facing input: hit-testing, transcript selection, mouse, wheel.

use super::*;

impl View {
    pub fn click_at(&self, x: f32, y: f32) -> Option<Click> {
        // Hyperlinks sit on top of everything, including the editor's
        // cursor-placement region — clicking one opens it, not moves the caret.
        if let Some(&(_, c)) = self
            .clicks
            .iter()
            .rev()
            .find(|(r, c)| matches!(c, Click::OpenUrl(_)) && r.contains(x, y))
        {
            return Some(c);
        }
        if let Some(g) = self.editor_geom {
            if g.rect.contains(x, y) {
                let row = ((y - g.rect.y + g.scroll_px) / g.line_h).floor().max(0.0) as usize;
                let cell_x = ((x - g.rect.x + g.hscroll_px) / g.adv + 0.5).max(0.0) as usize;
                return Some(Click::EditorPos { row, cell_x });
            }
        }
        self.clicks.iter().rev().find(|(r, _)| r.contains(x, y)).map(|(_, c)| *c)
    }

    /// Pen x offset of char boundary `c` on wrapped transcript line `i`:
    /// measured boundaries when the line has them, uniform cells otherwise.
    /// Boundaries past the end continue in mono cells (the newline cell).
    pub(super) fn agent_col_x(&self, i: usize, c: usize, adv: f32) -> f32 {
        match self.agent_xs.get(i).and_then(|t| t.as_ref()) {
            Some(xs) => {
                let last = xs.len() - 1;
                if c <= last {
                    xs[c]
                } else {
                    xs[last] + (c - last) as f32 * adv
                }
            }
            None => c as f32 * adv,
        }
    }

    /// (line, col) under a pixel position in the agent transcript. With
    /// `clamp`, positions outside the pane snap to the nearest text — used
    /// while dragging.
    pub(super) fn agent_pos_at(&self, x: f32, y: f32, clamp: bool) -> Option<(usize, usize)> {
        let (inner, row, adv, offset) = self.agent_geom?;
        if self.agent_lines.is_empty() || (!clamp && !inner.contains(x, y)) {
            return None;
        }
        let line = ((((y - inner.y + offset) / row).floor()).max(0.0) as usize)
            .min(self.agent_lines.len() - 1);
        let len = self.agent_lines[line].chars().count();
        let xrel = x - inner.x;
        let col = match self.agent_xs.get(line).and_then(|t| t.as_ref()) {
            // Nearest measured boundary — same advances the line was drawn with.
            Some(xs) => {
                let mut best = (0usize, f32::MAX);
                for (c, &bx) in xs.iter().enumerate() {
                    let d = (bx - xrel).abs();
                    if d <= best.1 {
                        best = (c, d);
                    }
                }
                best.0
            }
            None => (((xrel / adv) + 0.5).max(0.0) as usize).min(len),
        };
        Some((line, col))
    }

    /// The transcript selection as text, for the system clipboard. Wrapped
    /// segments of one source line join without injected newlines; label
    /// and separator decoration lines are skipped. None when the transcript
    /// pane wasn't drawn last frame or the selection resolves to nothing.
    pub fn agent_selection_text(&self) -> Option<String> {
        self.agent_geom?;
        let (a, b) = self.agent_sel?;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        if a == b {
            return None;
        }
        let mut out = String::new();
        let mut prev_src: Option<u32> = None;
        for i in a.0..=b.0 {
            let chars: Vec<char> = self.agent_lines.get(i)?.chars().collect();
            let Some(src) = *self.agent_src.get(i)? else { continue };
            let c0 = if i == a.0 { a.1.min(chars.len()) } else { 0 };
            let c1 = if i == b.0 { b.1.min(chars.len()) } else { chars.len() };
            if prev_src.is_some() && prev_src != Some(src) {
                out.push('\n');
            }
            out.extend(&chars[c0..c1]);
            prev_src = Some(src);
        }
        if out.is_empty() {
            return None;
        }
        Some(out)
    }

    pub fn on_mouse_down(&mut self, app: &mut App, x: f32, y: f32) {
        self.mouse = (x, y);
        self.needs_frame = true;
        // An open context menu swallows the click: hit an item, else dismiss.
        if app.context_menu.is_some() {
            match self.menu_rects.iter().find(|(r, _)| r.contains(x, y)).map(|(_, i)| *i) {
                Some(i) => app.menu_action_at(i),
                None => app.context_menu = None,
            }
            return;
        }
        // A release outside the window never reaches on_mouse_up; a fresh
        // press must not resume that stale drag.
        self.drag = Drag::None;
        if let Some(click) = self.click_at(x, y) {
            // Hyperlinks open a browser tab here rather than through
            // perform_click — the app crate has no DOM access. Opening
            // synchronously inside this gesture keeps popup blockers happy.
            if let Click::OpenUrl(i) = click {
                if let Some(url) = self.link_urls.get(i) {
                    super::links::open_url(url);
                }
                return;
            }
            if matches!(click, Click::EditorPos { .. }) {
                self.drag = Drag::Editor;
            }
            self.agent_sel = None;
            app.perform_click(click);
            return;
        }
        if app.route == Route::Agent {
            if let Some(pos) = self.agent_pos_at(x, y, false) {
                self.agent_sel = Some((pos, pos));
                self.drag = Drag::Agent;
            } else {
                self.agent_sel = None;
            }
        }
    }

    pub fn on_mouse_move(&mut self, app: &mut App, x: f32, y: f32) {
        self.mouse = (x, y);
        self.needs_frame = true;
        match self.drag {
            Drag::Agent => {
                if let Some(pos) = self.agent_pos_at(x, y, true) {
                    if let Some(sel) = &mut self.agent_sel {
                        sel.1 = pos;
                    }
                }
            }
            Drag::Editor => {
                if let Some(g) = self.editor_geom {
                    let row = ((y - g.rect.y + g.scroll_px) / g.line_h).floor().max(0.0) as usize;
                    let cell_x = ((x - g.rect.x + g.hscroll_px) / g.adv + 0.5).max(0.0) as usize;
                    app.editor_drag(row, cell_x);
                }
            }
            Drag::None => {}
        }
    }

    pub fn on_mouse_up(&mut self, _app: &mut App, _x: f32, _y: f32) {
        if self.drag == Drag::Agent {
            if let Some((a, b)) = self.agent_sel {
                if a == b {
                    self.agent_sel = None; // plain click, no drag
                }
            }
        }
        self.drag = Drag::None;
    }
}
