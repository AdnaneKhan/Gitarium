//! Host-facing input: hit-testing, transcript selection, mouse, wheel.

use super::*;

/// The `n`th raw line of the currently-open job log (Ready only).
fn log_line(app: &App, n: usize) -> Option<&str> {
    match app.rv.as_ref()?.job_logs.as_ref()? {
        (_, Loadable::Ready(text)) => text.lines().nth(n),
        _ => None,
    }
}

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
        // Actions runs/jobs splitter (lives in the gap between the panes).
        if let Some((hit, _, _)) = self.actions_split_hit {
            if hit.contains(x, y) {
                self.drag = Drag::ActionsSplit;
                return;
            }
        }
        // Job-log scrollbar thumb (not a Click region — handled directly).
        if let Some(g) = self.log_geom {
            let track = RectF::new(g.area.right() - self.f(9.0), g.area.y, self.f(9.0), g.area.h);
            if (g.lines as f32) * g.lh > g.area.h && track.contains(x, y) {
                self.drag = Drag::LogScroll;
                self.log_scroll_to(app, y);
                return;
            }
        }
        if let Some(click) = self.click_at(x, y) {
            // Hyperlinks open a browser tab here rather than through
            // perform_click — the app crate has no DOM access. Opening
            // synchronously inside this gesture keeps popup blockers happy.
            if let Click::OpenUrl(i) = click {
                if let Some(url) = self.link_urls.get(i) {
                    super::dom::open_url(url);
                }
                return;
            }
            // Download saves the cached log to a file (DOM access only here).
            if let Click::DownloadLog = click {
                if let Some(rv) = app.rv.as_ref() {
                    if let Some((job_id, Loadable::Ready(text))) = &rv.job_logs {
                        let name = format!("{}_{}.txt", rv.repo.full_name.replace('/', "_"), job_id);
                        super::dom::download_text(&name, text);
                    }
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
        // Job-log text selection (after click_at so the log's chips win).
        if let Some(pos) = self.log_pos_at(app, x, y, false) {
            self.log_sel = Some((pos, pos));
            self.drag = Drag::JobLog;
            return;
        }
        // A selectable text surface (agent transcript or issue/PR detail) is
        // present when its geometry was recorded last frame.
        if self.agent_geom.is_some() {
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
            Drag::ActionsSplit => {
                if let Some((_, x0, total)) = self.actions_split_hit {
                    if total > 0.0 {
                        self.actions_split = ((x - x0) / total).clamp(0.15, 0.85);
                    }
                }
            }
            Drag::LogScroll => self.log_scroll_to(app, y),
            Drag::JobLog => {
                if let Some(pos) = self.log_pos_at(app, x, y, true) {
                    if let Some(sel) = &mut self.log_sel {
                        sel.1 = pos;
                    }
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
        if self.drag == Drag::JobLog {
            if let Some((a, b)) = self.log_sel {
                if a == b {
                    self.log_sel = None; // plain click, no drag
                }
            }
        }
        self.drag = Drag::None;
    }

    /// (line, col) under a pixel position in the job-log view, or None when
    /// outside it. With `clamp`, positions outside snap to the nearest line
    /// (used while dragging a selection).
    fn log_pos_at(&self, app: &App, x: f32, y: f32, clamp: bool) -> Option<(usize, usize)> {
        let g = self.log_geom?;
        if g.lines == 0 || (!clamp && !g.area.contains(x, y)) {
            return None;
        }
        let rel = ((y - g.area.y - self.f(2.0)) / g.lh).max(0.0) as usize;
        let line = (g.scroll + rel).min(g.lines - 1);
        let col = (((x - g.area.x - self.f(4.0)) / g.adv) + 0.5).max(0.0) as usize;
        let len = log_line(app, line).map(|l| l.chars().count()).unwrap_or(0);
        Some((line, col.min(len)))
    }

    /// Scroll the log so the row under `y` (as a fraction of the track) is at
    /// the top — for the scrollbar drag.
    fn log_scroll_to(&mut self, app: &mut App, y: f32) {
        let Some(g) = self.log_geom else { return };
        let visible = (g.area.h / g.lh) as usize;
        let max = g.lines.saturating_sub(visible);
        if max == 0 {
            return;
        }
        let frac = ((y - g.area.y) / g.area.h).clamp(0.0, 1.0);
        if let Some(rv) = &mut app.rv {
            rv.jobs_scroll = ((frac * max as f32).round() as usize).min(max);
        }
    }

    /// The job-log selection as text (raw lines joined), for the clipboard.
    pub fn job_log_text(&self, app: &App) -> Option<String> {
        let (a, b) = self.log_sel?;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        if a == b {
            return None;
        }
        let mut out = String::new();
        for line in a.0..=b.0 {
            let chars: Vec<char> = log_line(app, line)?.chars().collect();
            let c0 = if line == a.0 { a.1.min(chars.len()) } else { 0 };
            let c1 = if line == b.0 { b.1.min(chars.len()) } else { chars.len() };
            if line > a.0 {
                out.push('\n');
            }
            out.extend(&chars[c0..c1]);
        }
        (!out.is_empty()).then_some(out)
    }
}
