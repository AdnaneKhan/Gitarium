//! The cyberpunk HUD: draws every screen in pixel space over the shared App
//! state machine. Owns smooth scrolling, hover animation, hit regions, and
//! the editor's pixel geometry; the App owns all actual state.

use std::collections::HashMap;

use crate::app::run_icon;
use crate::app::{AgentItem, App, Click, Loadable, Overlay, RepoFocus, RepoSource, Route, Scroll, Tab};
use crate::highlight::{self, LineState};
use crate::ui::grid::Rect as CellRect;
use crate::ui::lineinput::LineInput;

use super::anim::{ease_out, Smooth};
use super::atlas::{Atlas, MONO, UI, UI_BOLD};
use super::draw::{DrawList, RectF};
use super::theme::*;

#[derive(Clone, Copy)]
struct EditorGeom {
    rect: RectF,
    line_h: f32,
    adv: f32,
    scroll_px: f32,
    hscroll_px: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    /// Extending the agent-transcript selection.
    Agent,
    /// Extending the editor selection.
    Editor,
}

pub struct View {
    pub scale: f32,
    pub mouse: (f32, f32),
    pub needs_frame: bool,
    time: f64,
    dt: f32,
    started: bool,
    hot: (f32, f32),
    hover: HashMap<u64, Smooth>,
    scrolls: HashMap<u8, Smooth>,
    last_sel: HashMap<u8, usize>,
    last_editor_scroll: usize,
    last_agent_rev: u64,
    overlay_t: Smooth,
    toast_t: Smooth,
    route_t: Smooth,
    last_route: u8,
    tab_x: Smooth,
    tab_w: Smooth,
    clicks: Vec<(RectF, Click)>,
    wheels: Vec<(RectF, Scroll, f32)>, // rect, target, row_h
    editor_geom: Option<EditorGeom>,
    /// Agent transcript layout from the last frame: inner rect, row height,
    /// mono advance, scroll offset — plus the wrapped text for hit-testing
    /// and clipboard copy.
    agent_geom: Option<(RectF, f32, f32, f32)>,
    agent_lines: Vec<String>,
    /// Transcript selection: (anchor, head) as (line, col), unnormalized.
    agent_sel: Option<((usize, usize), (usize, usize))>,
    drag: Drag,
    pub cursor_pointer: bool,
    pub cursor_text: bool,
    active: bool,
}

fn skey(s: Scroll) -> u8 {
    match s {
        Scroll::Repos => 0,
        Scroll::Tree => 1,
        Scroll::Content => 2,
        Scroll::Runs => 3,
        Scroll::Jobs => 4,
        Scroll::Overlay => 5,
        Scroll::Agent => 6,
    }
}

fn wid(zone: u8, i: usize) -> u64 {
    ((zone as u64) << 48) ^ i as u64
}

const Z_REPO: u8 = 1;
const Z_TREE: u8 = 2;
const Z_TAB: u8 = 3;
const Z_CHIP: u8 = 4;
const Z_OVER: u8 = 5;
const Z_RUN: u8 = 6;

impl View {
    pub fn new(scale: f32) -> Self {
        View {
            scale,
            mouse: (-1e6, -1e6),
            needs_frame: true,
            time: 0.0,
            dt: 0.016,
            started: false,
            hot: (-1e6, -1e6),
            hover: HashMap::new(),
            scrolls: HashMap::new(),
            last_sel: HashMap::new(),
            last_editor_scroll: 0,
            last_agent_rev: 0,
            overlay_t: Smooth::new(0.0),
            toast_t: Smooth::new(0.0),
            route_t: Smooth::new(1.0),
            last_route: 255,
            tab_x: Smooth::new(0.0),
            tab_w: Smooth::new(0.0),
            clicks: Vec::new(),
            wheels: Vec::new(),
            editor_geom: None,
            agent_geom: None,
            agent_lines: Vec::new(),
            agent_sel: None,
            drag: Drag::None,
            cursor_pointer: false,
            cursor_text: false,
            active: false,
        }
    }

    fn f(&self, v: f32) -> f32 {
        v * self.scale
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn hover_amt(&mut self, id: u64, inside: bool) -> f32 {
        let s = self.hover.entry(id).or_insert_with(|| Smooth::new(0.0));
        s.target = if inside { 1.0 } else { 0.0 };
        if s.tick_n(self.dt, 16.0) {
            self.active = true;
        }
        s.v
    }

    fn sel_changed(&mut self, zone: u8, sel: usize) -> bool {
        let prev = self.last_sel.insert(zone, sel);
        prev != Some(sel)
    }

    /// Smooth scroll for a row list; keeps the selection visible when it
    /// moves. Returns the pixel offset.
    fn list_scroll(&mut self, target: Scroll, zone: u8, sel: usize, count: usize, row_h: f32, view_h: f32) -> f32 {
        let changed = self.sel_changed(zone, sel);
        let s = self.scrolls.entry(skey(target)).or_insert_with(|| Smooth::new(0.0));
        let max = (count as f32 * row_h - view_h).max(0.0);
        if changed {
            let sy = sel as f32 * row_h;
            if sy < s.target {
                s.target = sy;
            }
            if sy + row_h > s.target + view_h {
                s.target = sy + row_h - view_h;
            }
        }
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        s.v.clamp(0.0, max)
    }

    // -----------------------------------------------------------------------
    // Host-facing input
    // -----------------------------------------------------------------------

    pub fn click_at(&self, x: f32, y: f32) -> Option<Click> {
        if let Some(g) = self.editor_geom {
            if g.rect.contains(x, y) {
                let row = ((y - g.rect.y + g.scroll_px) / g.line_h).floor().max(0.0) as usize;
                let cell_x = ((x - g.rect.x + g.hscroll_px) / g.adv + 0.35).max(0.0) as usize;
                return Some(Click::EditorPos { row, cell_x });
            }
        }
        self.clicks.iter().rev().find(|(r, _)| r.contains(x, y)).map(|(_, c)| *c)
    }

    /// (line, col) under a pixel position in the agent transcript. With
    /// `clamp`, positions outside the pane snap to the nearest text — used
    /// while dragging.
    fn agent_pos_at(&self, x: f32, y: f32, clamp: bool) -> Option<(usize, usize)> {
        let (inner, row, adv, offset) = self.agent_geom?;
        if self.agent_lines.is_empty() || (!clamp && !inner.contains(x, y)) {
            return None;
        }
        let line = ((((y - inner.y + offset) / row).floor()).max(0.0) as usize)
            .min(self.agent_lines.len() - 1);
        let len = self.agent_lines[line].chars().count();
        let col = ((((x - inner.x) / adv) + 0.5).max(0.0) as usize).min(len);
        Some((line, col))
    }

    /// The transcript selection as text, for the system clipboard.
    pub fn agent_selection_text(&self) -> Option<String> {
        let (a, b) = self.agent_sel?;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        if a == b {
            return None;
        }
        let mut out = String::new();
        for i in a.0..=b.0 {
            let chars: Vec<char> = self.agent_lines.get(i)?.chars().collect();
            let c0 = if i == a.0 { a.1.min(chars.len()) } else { 0 };
            let c1 = if i == b.0 { b.1.min(chars.len()) } else { chars.len() };
            out.extend(&chars[c0..c1]);
            if i != b.0 {
                out.push('\n');
            }
        }
        Some(out)
    }

    pub fn on_mouse_down(&mut self, app: &mut App, x: f32, y: f32) {
        self.mouse = (x, y);
        self.needs_frame = true;
        if let Some(click) = self.click_at(x, y) {
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
                    let cell_x = ((x - g.rect.x + g.hscroll_px) / g.adv + 0.35).max(0.0) as usize;
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

    pub fn wheel(&mut self, app: &mut App, x: f32, y: f32, dy_px: f32) {
        let hit = self.wheels.iter().rev().find(|(r, _, _)| r.contains(x, y)).map(|(_, t, rh)| (*t, *rh));
        let Some((target, row_h)) = hit else { return };
        let s = self.scrolls.entry(skey(target)).or_insert_with(|| Smooth::new(0.0));
        s.target += dy_px;
        // Row write-back keeps keyboard navigation coherent in App state.
        let rows = (s.target.max(0.0) / row_h) as usize;
        match target {
            Scroll::Content => {
                if let Some(rv) = &mut app.rv {
                    if let Some(f) = &mut rv.file {
                        f.editor.scroll = rows.min(f.editor.line_count().saturating_sub(1));
                        self.last_editor_scroll = f.editor.scroll;
                    }
                }
            }
            Scroll::Repos => app.repo_scroll = rows,
            Scroll::Tree => {
                if let Some(rv) = &mut app.rv {
                    rv.tree_scroll = rows;
                }
            }
            Scroll::Runs => {
                if let Some(rv) = &mut app.rv {
                    rv.runs_scroll = rows;
                }
            }
            Scroll::Jobs => {
                if let Some(rv) = &mut app.rv {
                    rv.jobs_scroll = rows;
                }
            }
            Scroll::Overlay => {
                if let Some(Overlay::BranchPick { scroll, .. }) = &mut app.overlay {
                    *scroll = rows;
                }
            }
            // Scroll state for the agent transcript lives in the Smooth only;
            // there is no keyboard row cursor to keep coherent.
            Scroll::Agent => {}
        }
        self.needs_frame = true;
    }

    // -----------------------------------------------------------------------
    // Frame
    // -----------------------------------------------------------------------

    pub fn frame(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, t_ms: f64) {
        self.dt = if self.started {
            (((t_ms - self.time) / 1000.0) as f32).clamp(0.001, 0.05)
        } else {
            0.016
        };
        self.time = t_ms;
        self.started = true;
        self.active = false;
        self.clicks.clear();
        self.wheels.clear();
        self.editor_geom = None;
        self.agent_geom = None;
        if app.route != Route::Agent {
            self.agent_sel = None;
        }
        self.hot = if app.overlay.is_some() { (-1e6, -1e6) } else { self.mouse };

        dl.begin(w, h);
        self.background(dl, w, h);

        // Route entrance.
        let tag = match app.route {
            Route::Auth => 0,
            Route::Repos => 1,
            Route::Repo => 2,
            Route::Agent => 3,
        };
        if tag != self.last_route {
            self.last_route = tag;
            self.route_t.snap(0.0);
            self.route_t.target = 1.0;
        }
        if self.route_t.tick_n(self.dt, 9.0) {
            self.active = true;
        }
        let yoff = (1.0 - ease_out(self.route_t.v)) * self.f(16.0);

        match app.route {
            Route::Auth => self.auth_screen(app, dl, atlas, w, h, yoff),
            Route::Repos => self.repos_screen(app, dl, atlas, w, h, yoff),
            Route::Repo => self.repo_screen(app, dl, atlas, w, h, yoff),
            Route::Agent => self.agent_screen(app, dl, atlas, w, h, yoff),
        }

        self.status_bar(app, dl, atlas, w, h);
        self.overlay(app, dl, atlas, w, h);
        self.toast(app, dl, atlas, w, h);

        // Busy sweep along the very top.
        if busy(app) {
            let p = ((self.time * 0.00045) % 1.3) as f32 - 0.15;
            let bw = w * 0.22;
            let r = RectF::new(p * w, 0.0, bw, self.f(2.0));
            dl.glow(r, 1.0, with_a(CYAN, 0.25), self.f(8.0));
            dl.solid(r, with_a(CYAN, 0.9));
            self.active = true;
        }

        dl.scanlines(w, h, 0.05);

        self.cursor_pointer = {
            let (mx, my) = self.mouse;
            self.clicks.iter().any(|(r, _)| r.contains(mx, my))
        };
        self.cursor_text = {
            let (mx, my) = self.mouse;
            !self.cursor_pointer
                && self.agent_geom.map(|(r, ..)| r.contains(mx, my)).unwrap_or(false)
        };
        self.hover.retain(|_, s| s.v > 0.002 || s.target > 0.0);
    }

    fn background(&self, dl: &mut DrawList, w: f32, h: f32) {
        let step = self.f(72.0);
        let c = with_a(CYAN, 0.022);
        let mut x = step;
        while x < w {
            dl.solid(RectF::new(x, 0.0, 1.0, h), c);
            x += step;
        }
        let mut y = step;
        while y < h {
            dl.solid(RectF::new(0.0, y, w, 1.0), c);
            y += step;
        }
    }

    fn brackets(&self, dl: &mut DrawList, r: RectF, len: f32, color: Color) {
        let t = self.f(2.0);
        for (cx, cy, dx, dy) in [
            (r.x, r.y, 1.0, 1.0),
            (r.right(), r.y, -1.0, 1.0),
            (r.x, r.bottom(), 1.0, -1.0),
            (r.right(), r.bottom(), -1.0, -1.0),
        ] {
            let x0 = if dx > 0.0 { cx } else { cx - len };
            let y0 = if dy > 0.0 { cy } else { cy - t };
            dl.solid(RectF::new(x0, y0, len, t), color);
            let x1 = if dx > 0.0 { cx } else { cx - t };
            let y1 = if dy > 0.0 { cy } else { cy - len };
            dl.solid(RectF::new(x1, y1, t, len), color);
        }
    }

    fn panel(&self, dl: &mut DrawList, r: RectF) {
        dl.rrect(r, self.f(4.0), BG1, 1.0);
        dl.border(r, self.f(4.0), 1.0, BORDER_BRIGHT);
        self.brackets(dl, r, self.f(12.0), with_a(CYAN, 0.55));
    }

    fn input_field(&mut self, dl: &mut DrawList, atlas: &mut Atlas, input: &LineInput, r: RectF, focus: bool) {
        dl.rrect(r, self.f(3.0), BG2, 1.0);
        let line = RectF::new(r.x, r.bottom() - self.f(2.0), r.w, self.f(2.0));
        if focus {
            dl.glow(line, 1.0, with_a(CYAN, 0.3), self.f(7.0));
            dl.solid(line, with_a(CYAN, 0.9));
        } else {
            dl.solid(line, BORDER_BRIGHT);
        }
        let px = self.f(14.0);
        let (ascent, lh) = atlas.metrics(MONO, px);
        let adv = atlas.advance(MONO, px, 'M');
        let pad = self.f(12.0);
        let shown: String = if input.masked {
            "•".repeat(input.text.chars().count())
        } else {
            input.text.clone()
        };
        let visible = (((r.w - pad * 2.0) / adv) as usize).max(4);
        let cur = input.cursor;
        let off = (cur + 1).saturating_sub(visible);
        let slice: String = shown.chars().skip(off).take(visible).collect();
        let baseline = r.y + (r.h - lh) / 2.0 + ascent;
        dl.text(atlas, MONO, px, r.x + pad, baseline, &slice, TEXT, 0.0);
        if focus {
            self.active = true; // caret blink
            if ((self.time / 530.0) as i64) % 2 == 0 {
                let cx = r.x + pad + (cur - off) as f32 * adv;
                let cr = RectF::new(cx, baseline - ascent, self.f(2.0), lh);
                dl.glow(cr, 1.0, with_a(CYAN, 0.4), self.f(4.0));
                dl.solid(cr, CYAN);
            }
        }
    }

    /// Right-aligned action chip; returns the new right edge for stacking.
    #[allow(clippy::too_many_arguments)]
    fn chip(&mut self, dl: &mut DrawList, atlas: &mut Atlas, label: &str, right: f32, cy: f32, color: Color, click: Click, id: u64) -> f32 {
        let px = self.f(12.0);
        let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(1.5));
        let r = RectF::new(right - tw - self.f(20.0), cy - self.f(11.0), tw + self.f(20.0), self.f(22.0));
        let hv = self.hover_amt(id, r.contains(self.hot.0, self.hot.1));
        if hv > 0.01 {
            dl.glow(r, self.f(3.0), with_a(color, 0.20 * hv), self.f(9.0));
        }
        dl.rrect(r, self.f(3.0), with_a(color, 0.07 + 0.10 * hv), 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.75));
        let (ascent, lh) = atlas.metrics(UI_BOLD, px);
        dl.text(atlas, UI_BOLD, px, r.x + self.f(10.0), r.y + (r.h - lh) / 2.0 + ascent, label, color, self.f(1.5));
        self.clicks.push((r, click));
        r.x - self.f(8.0)
    }

    /// Left-aligned toolbar chip; returns the next x.
    #[allow(clippy::too_many_arguments)]
    fn tool_chip(&mut self, dl: &mut DrawList, atlas: &mut Atlas, label: &str, x: f32, y: f32, color: Color, click: Click, id: u64) -> f32 {
        let px = self.f(11.0);
        let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(1.2));
        let r = RectF::new(x, y, tw + self.f(18.0), self.f(24.0));
        let hv = self.hover_amt(id, r.contains(self.hot.0, self.hot.1));
        if hv > 0.01 {
            dl.glow(r, self.f(3.0), with_a(color, 0.18 * hv), self.f(8.0));
        }
        dl.rrect(r, self.f(3.0), with_a(color, 0.05 + 0.08 * hv), 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.55 + 0.35 * hv));
        let (asc, lh) = atlas.metrics(UI_BOLD, px);
        dl.text(atlas, UI_BOLD, px, r.x + self.f(9.0), r.y + (r.h - lh) / 2.0 + asc, label, color, self.f(1.2));
        self.clicks.push((r, click));
        r.right() + self.f(8.0)
    }

    fn sweep_note(&mut self, dl: &mut DrawList, atlas: &mut Atlas, x: f32, y: f32, w: f32, label: &str) {
        let px = self.f(13.0);
        dl.text(atlas, UI, px, x, y, label, DIM, self.f(2.0));
        let bar = RectF::new(x, y + self.f(10.0), w.min(self.f(220.0)), self.f(2.0));
        let p = ((self.time * 0.0009) % 1.0) as f32;
        dl.solid(bar, with_a(CYAN, 0.10));
        let pw = bar.w * 0.3;
        dl.push_clip(bar);
        dl.solid(RectF::new(bar.x + p * bar.w - pw, bar.y, pw, bar.h), with_a(CYAN, 0.8));
        dl.pop_clip();
        self.active = true;
    }

    // -----------------------------------------------------------------------
    // Screens
    // -----------------------------------------------------------------------

    fn auth_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let pw = self.f(560.0).min(w - self.f(32.0));
        let ph = self.f(264.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 - self.f(30.0) + yoff, pw, ph);
        dl.glow(r, self.f(4.0), with_a(CYAN, 0.05), self.f(36.0));
        self.panel(dl, r);

        let x = r.x + self.f(30.0);
        let title_px = self.f(34.0);
        let end = dl.text(atlas, UI_BOLD, title_px, x, r.y + self.f(56.0), "RUSTVM", CYAN, self.f(7.0));
        dl.text(atlas, UI, self.f(14.0), end + self.f(6.0), r.y + self.f(56.0), "// GITHUB INTERFACE", MAGENTA, self.f(2.5));

        dl.text(
            atlas,
            UI,
            self.f(13.0),
            x,
            r.y + self.f(96.0),
            "ACCESS TOKEN — ENTER ON EMPTY FOR ANONYMOUS",
            DIM,
            self.f(1.5),
        );
        let field = RectF::new(x, r.y + self.f(112.0), r.w - self.f(60.0), self.f(42.0));
        let input = app.token_input.clone_shallow();
        self.input_field(dl, atlas, &input, field, !app.auth_busy);

        let msg_y = r.y + self.f(190.0);
        if app.auth_busy {
            let a = 0.55 + 0.45 * ((self.time * 0.006).sin() as f32);
            dl.text(atlas, UI, self.f(14.0), x, msg_y, "VALIDATING TOKEN…", with_a(YELLOW, a), self.f(1.5));
            self.active = true;
        } else if let Some(err) = &app.auth_error {
            let msg = dl.fit(atlas, UI, self.f(14.0), err, r.w - self.f(60.0));
            dl.text(atlas, UI, self.f(14.0), x, msg_y, &msg, RED, 0.0);
        } else {
            dl.text(
                atlas,
                UI,
                self.f(13.0),
                x,
                msg_y,
                "paste with Cmd/Ctrl+V · fine-grained PAT recommended",
                DIM,
                0.0,
            );
        }
    }

    fn repos_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let title = match &app.repo_source {
            RepoSource::Mine => "RUSTVM::GITHUB".to_string(),
            RepoSource::Org(n) => format!("ORG::{}", n.to_uppercase()),
        };
        let hh = self.header(app, dl, atlas, w, &title, None);
        let mut top = hh + self.f(6.0);

        // Toolbar: sort + visibility toggles (only once a list is loaded).
        if app.repos.ready().is_some() {
            let ty = top + self.f(4.0);
            let mut x = self.f(16.0);
            let (sort_label, dir, hide_forks, hide_archived) =
                (app.repo_sort.label(), app.sort_asc, app.hide_forks, app.hide_archived);
            x = self.tool_chip(dl, atlas, &format!("SORT: {}", sort_label), x, ty, CYAN, Click::SortCycle, wid(Z_CHIP, 20));
            x = self.tool_chip(dl, atlas, if dir { "↑" } else { "↓" }, x, ty, CYAN, Click::SortDir, wid(Z_CHIP, 21));
            x += self.f(8.0);
            let fc = if hide_forks { MAGENTA } else { with_a(TEXT, 0.55) };
            x = self.tool_chip(
                dl,
                atlas,
                if hide_forks { "FORKS: HIDDEN" } else { "FORKS: SHOWN" },
                x,
                ty,
                fc,
                Click::ToggleForks,
                wid(Z_CHIP, 22),
            );
            let ac = if hide_archived { MAGENTA } else { with_a(TEXT, 0.55) };
            x = self.tool_chip(
                dl,
                atlas,
                if hide_archived { "ARCHIVED: HIDDEN" } else { "ARCHIVED: SHOWN" },
                x,
                ty,
                ac,
                Click::ToggleArchived,
                wid(Z_CHIP, 23),
            );
            let _ = x;
            let shown = app.filtered_repos().len();
            let total = app.repos.ready().map(|r| r.len()).unwrap_or(0);
            let count = format!("{}/{}", shown, total);
            let cw = dl.text_width(atlas, MONO, self.f(12.0), &count, 0.0);
            dl.text(atlas, MONO, self.f(12.0), w - cw - self.f(20.0), ty + self.f(16.0), &count, DIM, 0.0);
            top += self.f(34.0);
        }

        if app.filter_active || !app.filter.text.is_empty() {
            let bar = RectF::new(self.f(16.0), top, w - self.f(32.0), self.f(36.0));
            dl.text(atlas, UI_BOLD, self.f(16.0), bar.x + self.f(2.0), bar.y + self.f(24.0), "/", CYAN, 0.0);
            let field = RectF::new(bar.x + self.f(18.0), bar.y, bar.w - self.f(18.0), bar.h);
            let input = app.filter.clone_shallow();
            self.input_field(dl, atlas, &input, field, app.filter_active);
            top += self.f(44.0);
        }

        let list = RectF::new(self.f(16.0), top + yoff, w - self.f(32.0), h - top - self.f(34.0) - yoff);

        match &app.repos {
            Loadable::Loading => {
                self.sweep_note(dl, atlas, list.x + self.f(8.0), list.y + self.f(24.0), list.w, "SCANNING REPOSITORIES…")
            }
            Loadable::Failed(e) => {
                let msg = dl.fit(atlas, UI, self.f(14.0), e, list.w - self.f(16.0));
                dl.text(atlas, UI, self.f(14.0), list.x + self.f(8.0), list.y + self.f(24.0), &msg, RED, 0.0);
            }
            Loadable::Idle => {
                dl.text(
                    atlas,
                    UI,
                    self.f(15.0),
                    list.x + self.f(8.0),
                    list.y + self.f(28.0),
                    "ANONYMOUS MODE — NO REPOSITORY LIST",
                    DIM,
                    self.f(2.0),
                );
                dl.text(
                    atlas,
                    UI_BOLD,
                    self.f(15.0),
                    list.x + self.f(8.0),
                    list.y + self.f(54.0),
                    "[O] OPEN owner/repo OR AN ORGANIZATION",
                    CYAN,
                    self.f(1.5),
                );
            }
            Loadable::Ready(_) => {
                let filtered = app.filtered_repos();
                if filtered.is_empty() {
                    dl.text(atlas, UI, self.f(14.0), list.x + self.f(8.0), list.y + self.f(24.0), "no matches", DIM, 0.0);
                }
                if app.repo_sel >= filtered.len() && !filtered.is_empty() {
                    app.repo_sel = filtered.len() - 1;
                }
                // One full-width card per row.
                let gap = self.f(10.0);
                let card_h = self.f(88.0);
                let row_h = card_h + gap;
                app.layout.repos_cols = 1;
                app.layout.repos_h = ((list.h / row_h) as usize).max(1);
                let offset = self.list_scroll(Scroll::Repos, Z_REPO, app.repo_sel, filtered.len(), row_h, list.h);
                let repos = app.repos.ready().unwrap();
                dl.push_clip(list);
                let first = (offset / row_h) as usize;
                for vis in 0..(list.h / row_h) as usize + 2 {
                    let fi = first + vis;
                    if fi >= filtered.len() {
                        break;
                    }
                    let repo = &repos[filtered[fi]];
                    let card = RectF::new(list.x, list.y + fi as f32 * row_h - offset, list.w - self.f(8.0), card_h);
                    let selected = fi == app.repo_sel;
                    let hv = self.hover_amt(wid(Z_REPO, fi), card.contains(self.hot.0, self.hot.1));

                    if selected {
                        dl.glow(card, self.f(4.0), with_a(CYAN, 0.10), self.f(14.0));
                    }
                    dl.rrect(card, self.f(4.0), BG1, 1.0);
                    if hv > 0.01 {
                        dl.rrect(card, self.f(4.0), with_a(CYAN, 0.045 * hv), 1.0);
                    }
                    let border_c = if selected {
                        with_a(CYAN, 0.85)
                    } else if hv > 0.01 {
                        with_a(CYAN, 0.2 + 0.3 * hv)
                    } else {
                        BORDER_BRIGHT
                    };
                    dl.border(card, self.f(4.0), 1.0, border_c);
                    if selected {
                        self.brackets(dl, card, self.f(9.0), with_a(CYAN, 0.8));
                    }

                    let pad = self.f(16.0);
                    let inner_w = card.w - pad * 2.0;

                    // Line 1: name + badges (left), pushed age (right).
                    let l1 = card.y + self.f(24.0);
                    let name = dl.fit(atlas, UI_BOLD, self.f(15.5), &repo.full_name, inner_w * 0.5);
                    let mut x = dl.text(
                        atlas,
                        UI_BOLD,
                        self.f(15.5),
                        card.x + pad,
                        l1,
                        &name,
                        if selected { CYAN } else { TEXT },
                        0.0,
                    );
                    let badges: [(bool, &str, Color); 3] = [
                        (repo.private, "PRIVATE", MAGENTA),
                        (repo.fork, "FORK", with_a(TEXT, 0.55)),
                        (repo.archived, "ARCHIVED", YELLOW),
                    ];
                    for (on, label, color) in badges {
                        if !on {
                            continue;
                        }
                        let bpx = self.f(9.5);
                        let tw = dl.text_width(atlas, UI, bpx, label, self.f(1.2));
                        let br = RectF::new(x + self.f(10.0), card.y + self.f(11.0), tw + self.f(10.0), self.f(16.0));
                        dl.border(br, self.f(2.0), 1.0, with_a(color, 0.8));
                        dl.text(atlas, UI, bpx, br.x + self.f(5.0), card.y + self.f(23.0), label, color, self.f(1.2));
                        x = br.right();
                    }
                    if let Some(p) = &repo.pushed_at {
                        let age = format!("pushed {}", crate::app::fmt_age(p));
                        let tw = dl.text_width(atlas, UI, self.f(11.5), &age, 0.0);
                        dl.text(atlas, UI, self.f(11.5), card.right() - pad - tw, l1, &age, FAINT, 0.0);
                    }

                    // Line 2: description, one line across the full width.
                    if let Some(d) = &repo.description {
                        let msg = dl.fit(atlas, UI, self.f(12.5), d, inner_w);
                        dl.text(atlas, UI, self.f(12.5), card.x + pad, card.y + self.f(46.0), &msg, DIM, 0.0);
                    }

                    // Line 3: indicators (left), branch chip (right).
                    let iy = card.y + self.f(70.0);
                    let mut ix = card.x + pad;
                    if let Some(lang) = &repo.language {
                        let dot = RectF::new(ix, iy - self.f(7.0), self.f(8.0), self.f(8.0));
                        dl.rrect(dot, self.f(4.0), lang_color(lang), 1.0);
                        ix = dl.text(atlas, UI, self.f(12.0), ix + self.f(13.0), iy, lang, with_a(TEXT, 0.75), 0.0)
                            + self.f(16.0);
                    }
                    if repo.stargazers_count > 0 {
                        ix = dl.text(atlas, MONO, self.f(11.5), ix, iy, &format!("*{}", repo.stargazers_count), DIM, 0.0)
                            + self.f(16.0);
                    }
                    if repo.forks_count > 0 {
                        ix = dl.text(atlas, UI, self.f(12.0), ix, iy, &format!("{} forks", repo.forks_count), DIM, 0.0)
                            + self.f(16.0);
                    }
                    if repo.open_issues_count > 0 {
                        ix = dl.text(atlas, UI, self.f(12.0), ix, iy, &format!("{} issues", repo.open_issues_count), DIM, 0.0)
                            + self.f(16.0);
                    }
                    if let Some(l) = repo.license.as_ref().and_then(|l| l.spdx_id.as_deref()) {
                        if l != "NOASSERTION" && ix < card.right() - pad - self.f(120.0) {
                            dl.text(atlas, UI, self.f(12.0), ix, iy, l, FAINT, 0.0);
                        }
                    }
                    let chip = dl.fit(atlas, MONO, self.f(11.0), &format!("[{}]", repo.default_branch), inner_w * 0.3);
                    let cw = dl.text_width(atlas, MONO, self.f(11.0), &chip, 0.0);
                    dl.text(atlas, MONO, self.f(11.0), card.right() - pad - cw, iy, &chip, with_a(CYAN, 0.6), 0.0);

                    self.clicks.push((card, Click::Repo(fi)));
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, filtered.len() as f32 * row_h, offset);
                self.wheels.push((list, Scroll::Repos, row_h));
            }
        }
    }

    /// Top strip; returns its height.
    fn header(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, title: &str, repo: Option<String>) -> f32 {
        let hh = self.f(52.0);
        dl.solid(RectF::new(0.0, 0.0, w, hh), BG1);
        dl.solid(RectF::new(0.0, hh - 1.0, w, 1.0), BORDER_BRIGHT);

        // Pulsing status dot.
        let pulse = 0.55 + 0.45 * ((self.time * 0.003).sin() as f32);
        let dot = RectF::new(self.f(18.0), hh / 2.0 - self.f(4.0), self.f(8.0), self.f(8.0));
        dl.glow(dot, self.f(4.0), with_a(CYAN, 0.35 * pulse), self.f(8.0));
        dl.rrect(dot, self.f(4.0), with_a(CYAN, pulse), 1.0);
        self.active = true;

        let mut x = dl.text(atlas, UI_BOLD, self.f(19.0), self.f(38.0), self.f(33.0), title, TEXT, self.f(3.5));

        if let Some(branch) = repo {
            // Branch chip.
            let label = format!("{} ▾", branch);
            let px = self.f(13.0);
            let tw = dl.text_width(atlas, MONO, px, &label, 0.0);
            let chip = RectF::new(x + self.f(16.0), hh / 2.0 - self.f(13.0), tw + self.f(22.0), self.f(26.0));
            let hv = self.hover_amt(wid(Z_CHIP, 99), chip.contains(self.hot.0, self.hot.1));
            if hv > 0.01 {
                dl.glow(chip, self.f(3.0), with_a(CYAN, 0.2 * hv), self.f(8.0));
            }
            dl.rrect(chip, self.f(3.0), with_a(CYAN, 0.06 + 0.08 * hv), 1.0);
            dl.border(chip, self.f(3.0), 1.0, with_a(CYAN, 0.7));
            let (asc, lh) = atlas.metrics(MONO, px);
            dl.text(atlas, MONO, px, chip.x + self.f(11.0), chip.y + (chip.h - lh) / 2.0 + asc, &label, CYAN, 0.0);
            self.clicks.push((chip, Click::BranchBtn));
            x = chip.right();

            // Tabs.
            let tabs = [(Tab::Code, "CODE"), (Tab::Actions, "ACTIONS")];
            let sel_tab = app.rv.as_ref().map(|rv| rv.tab).unwrap_or(Tab::Code);
            let mut tx = x + self.f(30.0);
            for (i, (tab, label)) in tabs.iter().enumerate() {
                let px = self.f(14.0);
                let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(3.0));
                let region = RectF::new(tx - self.f(8.0), 0.0, tw + self.f(16.0), hh);
                let hv = self.hover_amt(wid(Z_TAB, i), region.contains(self.hot.0, self.hot.1));
                let on = *tab == sel_tab;
                let c = if on {
                    CYAN
                } else {
                    with_a(TEXT, 0.45 + 0.4 * hv)
                };
                dl.text(atlas, UI_BOLD, px, tx, self.f(32.0), label, c, self.f(3.0));
                if on {
                    self.tab_x.target = tx;
                    self.tab_w.target = tw;
                    if self.tab_x.v == 0.0 {
                        self.tab_x.snap(tx);
                        self.tab_w.snap(tw);
                    }
                }
                self.clicks.push((region, Click::Tab(*tab)));
                tx += tw + self.f(26.0);
            }
            if self.tab_x.tick(self.dt, 18.0) | self.tab_w.tick(self.dt, 18.0) {
                self.active = true;
            }
            let underline = RectF::new(self.tab_x.v, hh - self.f(3.0), self.tab_w.v, self.f(2.0));
            dl.glow(underline, 1.0, with_a(CYAN, 0.3), self.f(6.0));
            dl.solid(underline, CYAN);
        }

        // Right side: identity.
        let who = app.login.clone().unwrap_or_else(|| "ANONYMOUS".into());
        let tw = dl.text_width(atlas, UI, self.f(13.0), &who, self.f(1.5));
        dl.text(atlas, UI, self.f(13.0), w - tw - self.f(20.0), self.f(32.0), &who, DIM, self.f(1.5));
        hh
    }

    fn repo_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let (title, branch, tab) = match &app.rv {
            Some(rv) => (rv.repo.full_name.clone(), rv.branch.clone(), rv.tab),
            None => return,
        };
        let hh = self.header(app, dl, atlas, w, &title, Some(branch));
        let top = hh + self.f(10.0) + yoff;
        let bottom = h - self.f(34.0);
        match tab {
            Tab::Code => self.code_tab(app, dl, atlas, w, top, bottom),
            Tab::Actions => self.actions_tab(app, dl, atlas, w, top, bottom),
        }
    }

    fn agent_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let hh = self.header(app, dl, atlas, w, "AGENT::CLAUDE", None);

        // No API key yet: key/endpoint entry panel, mirroring the auth screen.
        if app.anthropic_key.is_none() {
            let pw = self.f(560.0).min(w - self.f(32.0));
            let ph = self.f(330.0);
            let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 - self.f(30.0) + yoff, pw, ph);
            dl.glow(r, self.f(4.0), with_a(MAGENTA, 0.05), self.f(36.0));
            self.panel(dl, r);
            let x = r.x + self.f(30.0);
            let end = dl.text(atlas, UI_BOLD, self.f(28.0), x, r.y + self.f(52.0), "AGENT", MAGENTA, self.f(6.0));
            dl.text(atlas, UI, self.f(14.0), end + self.f(6.0), r.y + self.f(52.0), "// GITHUB AUTOPILOT", CYAN, self.f(2.5));

            let url_focused = app.agent.url_focused;
            dl.text(atlas, UI, self.f(13.0), x, r.y + self.f(92.0), "ANTHROPIC API KEY", DIM, self.f(1.5));
            let key_field = RectF::new(x, r.y + self.f(104.0), r.w - self.f(60.0), self.f(42.0));
            let key_input = app.agent.key_input.clone_shallow();
            self.input_field(dl, atlas, &key_input, key_field, !url_focused);

            dl.text(
                atlas,
                UI,
                self.f(13.0),
                x,
                r.y + self.f(184.0),
                "API ENDPOINT — OPTIONAL, DEFAULTS TO api.anthropic.com",
                DIM,
                self.f(1.5),
            );
            let url_field = RectF::new(x, r.y + self.f(196.0), r.w - self.f(60.0), self.f(42.0));
            let url_input = app.agent.url_input.clone_shallow();
            self.input_field(dl, atlas, &url_input, url_field, url_focused);

            dl.text(
                atlas,
                UI,
                self.f(12.5),
                x,
                r.y + self.f(274.0),
                "stored in localStorage · [TAB] switch field · [ESC] back",
                DIM,
                0.0,
            );
            return;
        }

        let top = hh + self.f(8.0) + yoff;
        let bottom = h - self.f(34.0);

        // Toolbar: model id + busy pulse left, action chips right.
        let cy = top + self.f(14.0);
        let mut right = w - self.f(16.0);
        right = self.chip(dl, atlas, "CLEAR", right, cy, CYAN, Click::AgentClear, wid(Z_CHIP, 40));
        let _ = self.chip(dl, atlas, "KEY", right, cy, MAGENTA, Click::AgentResetKey, wid(Z_CHIP, 41));
        let model = crate::agent::MODEL.to_uppercase();
        let mut mx = dl.text(atlas, MONO, self.f(11.5), self.f(18.0), cy + self.f(4.0), &model, DIM, self.f(1.0));
        if let Some(u) = &app.anthropic_url {
            let host = u.trim_start_matches("https://").trim_start_matches("http://");
            mx = dl.text(
                atlas,
                MONO,
                self.f(11.5),
                mx + self.f(10.0),
                cy + self.f(4.0),
                &format!("· {}", host),
                FAINT,
                0.0,
            );
        }
        if app.agent.busy {
            let a = 0.55 + 0.45 * ((self.time * 0.006).sin() as f32);
            dl.text(atlas, UI, self.f(12.0), mx + self.f(18.0), cy + self.f(4.0), "WORKING…", with_a(YELLOW, a), self.f(2.0));
            self.active = true;
        }

        let pane = RectF::new(self.f(16.0), top + self.f(30.0), w - self.f(32.0), bottom - top - self.f(80.0));
        self.panel(dl, pane);

        // ---- transcript, wrapped to mono columns ----
        let pad = self.f(14.0);
        let inner = RectF::new(pane.x + pad, pane.y + pad, pane.w - pad * 2.0, pane.h - pad * 2.0);
        let px = self.f(12.5);
        let (asc, lh) = atlas.metrics(MONO, px);
        let adv = atlas.advance(MONO, px, 'M').max(1.0);
        let cols = ((inner.w / adv) as usize).max(8);
        let row = lh * 1.4;

        struct TLine {
            text: String,
            color: Color,
            label: bool,
            code: bool,
            spans: Vec<highlight::Span>,
        }
        let plain = |text: String, color: Color| TLine {
            text,
            color,
            label: false,
            code: false,
            spans: Vec::new(),
        };
        let mut lines: Vec<TLine> = Vec::new();
        let push_wrapped = |lines: &mut Vec<TLine>, text: &str, color: Color| {
            let mut buf = Vec::new();
            wrap_chars(text, cols, &mut buf);
            for l in buf {
                lines.push(plain(l, color));
            }
        };
        // Assistant text: ``` fences become syntax-highlighted code blocks.
        let push_assistant = |lines: &mut Vec<TLine>, t: &str| {
            let mut in_code = false;
            let mut spec: Option<&'static highlight::LangSpec> = None;
            let mut state = LineState::Normal;
            for raw in t.split('\n') {
                let trimmed = raw.trim_start();
                if trimmed.starts_with("```") {
                    in_code = !in_code;
                    if in_code {
                        spec = lang_for_tag(trimmed[3..].trim());
                        state = LineState::Normal;
                    }
                    continue;
                }
                if !in_code {
                    push_wrapped(lines, raw, with_a(TEXT, 0.92));
                    continue;
                }
                let expanded = raw.replace('\t', "    ");
                let (spans, next) = match spec {
                    Some(sp) => highlight::highlight(sp, &expanded, state),
                    None => (Vec::new(), state),
                };
                state = next;
                let chars: Vec<char> = expanded.chars().collect();
                let mut s0 = 0;
                loop {
                    let s1 = (s0 + cols).min(chars.len());
                    let seg_spans = spans
                        .iter()
                        .filter(|(a, b, _)| *b > s0 && *a < s1)
                        .map(|(a, b, c)| (a.saturating_sub(s0), (b - s0).min(s1 - s0), *c))
                        .collect();
                    lines.push(TLine {
                        text: chars[s0..s1].iter().collect(),
                        color: with_a(TEXT, 0.9),
                        label: false,
                        code: true,
                        spans: seg_spans,
                    });
                    s0 = s1;
                    if s0 >= chars.len() {
                        break;
                    }
                }
            }
        };
        for item in &app.agent.transcript {
            match item {
                AgentItem::User(t) => {
                    lines.push(TLine {
                        text: "YOU".into(),
                        color: CYAN,
                        label: true,
                        code: false,
                        spans: Vec::new(),
                    });
                    push_wrapped(&mut lines, t, TEXT);
                }
                AgentItem::Text(t) => {
                    lines.push(TLine {
                        text: "CLAUDE".into(),
                        color: MAGENTA,
                        label: true,
                        code: false,
                        spans: Vec::new(),
                    });
                    push_assistant(&mut lines, t);
                }
                AgentItem::Tool { label, done } => {
                    let (icon, color) = match done {
                        None => ('●', YELLOW),
                        Some(true) => ('✓', GREEN),
                        Some(false) => ('✗', RED),
                    };
                    push_wrapped(&mut lines, &format!("{} {}", icon, label), with_a(color, 0.85));
                }
                AgentItem::Error(e) => push_wrapped(&mut lines, &format!("✗ {}", e), RED),
            }
            lines.push(plain(String::new(), TEXT));
        }
        if lines.is_empty() {
            dl.text(
                atlas,
                UI,
                self.f(14.5),
                inner.x,
                inner.y + self.f(20.0),
                "DESCRIBE A GITHUB TASK — THE AGENT PLANS AND RUNS THE API CALLS",
                DIM,
                self.f(1.5),
            );
            dl.text(
                atlas,
                UI,
                self.f(13.0),
                inner.x,
                inner.y + self.f(46.0),
                "try: list my open PRs · open an issue here · what changed in the last release?",
                FAINT,
                0.0,
            );
        }

        let content_h = lines.len() as f32 * row;
        let max = (content_h - inner.h).max(0.0);
        let stick = app.agent.rev != self.last_agent_rev;
        self.last_agent_rev = app.agent.rev;
        let s = self.scrolls.entry(skey(Scroll::Agent)).or_insert_with(|| Smooth::new(0.0));
        if stick {
            s.target = max;
        }
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);

        // Expose layout + text to the input layer (drag selection, copy).
        self.agent_lines = lines.iter().map(|l| l.text.clone()).collect();
        self.agent_geom = Some((inner, row, adv, offset));
        let sel = self.agent_sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });

        dl.push_clip(inner);
        let first = (offset / row) as usize;
        for vis in 0..(inner.h / row) as usize + 2 {
            let i = first + vis;
            let Some(l) = lines.get(i) else { break };
            let ytop = inner.y + i as f32 * row - offset;
            let y = ytop + asc;
            if l.code {
                dl.solid(RectF::new(inner.x - self.f(6.0), ytop, inner.w + self.f(12.0), row), BG2);
                dl.solid(RectF::new(inner.x - self.f(6.0), ytop, self.f(2.0), row), with_a(CYAN, 0.45));
            }
            // Selection band (under the text, over the code strip).
            if let Some((a, b)) = sel {
                if i >= a.0 && i <= b.0 && !(a == b) {
                    let len = l.text.chars().count();
                    let c0 = if i == a.0 { a.1.min(len) } else { 0 };
                    let c1 = if i == b.0 { b.1.min(len) } else { len + 1 };
                    if c1 > c0 {
                        dl.solid(
                            RectF::new(inner.x + c0 as f32 * adv, ytop, (c1 - c0) as f32 * adv, row),
                            with_a(CYAN, 0.2),
                        );
                    }
                }
            }
            if l.text.is_empty() {
                continue;
            }
            if l.label {
                dl.text(atlas, UI_BOLD, self.f(11.5), inner.x, y, &l.text, l.color, self.f(2.5));
            } else if !l.spans.is_empty() {
                // Syntax-colored runs (fence content; tabs pre-expanded).
                let mut span_i = 0;
                let mut run = String::new();
                let mut run_color = l.color;
                let mut run_start = 0usize;
                for (ci, ch) in l.text.chars().enumerate() {
                    while span_i < l.spans.len() && l.spans[span_i].1 <= ci {
                        span_i += 1;
                    }
                    let color = l.spans
                        .get(span_i)
                        .filter(|(s, e, _)| *s <= ci && ci < *e)
                        .map(|(_, _, c)| super::theme::c(*c, 1.0))
                        .unwrap_or(l.color);
                    if color != run_color && !run.is_empty() {
                        dl.text(atlas, MONO, px, inner.x + run_start as f32 * adv, y, &run, run_color, 0.0);
                        run.clear();
                    }
                    if run.is_empty() {
                        run_start = ci;
                        run_color = color;
                    }
                    run.push(ch);
                }
                if !run.is_empty() {
                    dl.text(atlas, MONO, px, inner.x + run_start as f32 * adv, y, &run, run_color, 0.0);
                }
            } else {
                dl.text(atlas, MONO, px, inner.x, y, &l.text, l.color, 0.0);
            }
        }
        dl.pop_clip();
        self.scrollbar(dl, &pane, content_h + pad * 2.0, offset);
        self.wheels.push((pane, Scroll::Agent, row));

        // ---- prompt input ----
        let bar = RectF::new(pane.x, pane.bottom() + self.f(10.0), pane.w, self.f(40.0));
        let input = app.agent.input.clone_shallow();
        self.input_field(dl, atlas, &input, bar, !app.agent.busy);
    }

    fn code_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let tree = RectF::new(self.f(16.0), top, self.f(300.0).min(w * 0.3), bottom - top);
        let content = RectF::new(tree.right() + self.f(12.0), top, w - tree.right() - self.f(28.0), bottom - top);
        self.panel(dl, tree);
        self.panel(dl, content);

        let row_h = self.f(27.0);
        let inner = tree.shrink(self.f(8.0));
        app.layout.tree_h = (inner.h / row_h).max(1.0) as usize;

        // ---- tree
        enum TreeState {
            Loading,
            Failed(String),
            Ready,
        }
        let state = match &app.rv.as_ref().unwrap().tree {
            Loadable::Loading | Loadable::Idle => TreeState::Loading,
            Loadable::Failed(e) => TreeState::Failed(e.clone()),
            Loadable::Ready(_) => TreeState::Ready,
        };
        match state {
            TreeState::Loading => {
                self.sweep_note(dl, atlas, inner.x + self.f(6.0), inner.y + self.f(22.0), inner.w, "SCANNING TREE…")
            }
            TreeState::Failed(e) => {
                let msg = dl.fit(atlas, UI, self.f(13.0), &e, inner.w - self.f(8.0));
                dl.text(atlas, UI, self.f(13.0), inner.x + self.f(6.0), inner.y + self.f(22.0), &msg, RED, 0.0);
            }
            TreeState::Ready => {
                let (sel, count, focus_tree, truncated) = {
                    let rv = app.rv.as_ref().unwrap();
                    (rv.tree_sel, rv.rows.len(), rv.focus == RepoFocus::Tree, rv.truncated)
                };
                let offset = self.list_scroll(Scroll::Tree, Z_TREE, sel, count, row_h, inner.h);
                dl.push_clip(inner);
                {
                    let rv = app.rv.as_ref().unwrap();
                    let first = (offset / row_h) as usize;
                    for vis in 0..(inner.h / row_h) as usize + 2 {
                        let i = first + vis;
                        if i >= rv.rows.len() {
                            break;
                        }
                        let row = &rv.rows[i];
                        let y = inner.y + i as f32 * row_h - offset;
                        let rr = RectF::new(inner.x, y, inner.w, row_h - 1.0);
                        let selected = i == sel;
                        let hv = self.hover_amt(wid(Z_TREE, i), rr.contains(self.hot.0, self.hot.1));
                        let a = if selected {
                            if focus_tree {
                                0.12
                            } else {
                                0.06
                            }
                        } else {
                            0.05 * hv
                        };
                        if a > 0.005 {
                            dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                        }
                        let ix = inner.x + self.f(8.0) + row.depth as f32 * self.f(15.0);
                        let (asc, _) = atlas.metrics(MONO, self.f(12.0));
                        let baseline = y + (row_h + asc) / 2.0 - self.f(2.0);
                        let (mark, mc) = if row.is_dir {
                            if rv.expanded.contains(&row.path) {
                                ("▾", CYAN)
                            } else {
                                ("▸", CYAN)
                            }
                        } else {
                            ("·", FAINT)
                        };
                        dl.text(atlas, MONO, self.f(12.0), ix, baseline, mark, mc, 0.0);
                        let nc = if row.is_dir {
                            with_a(CYAN, 0.9)
                        } else if selected {
                            TEXT
                        } else {
                            with_a(TEXT, 0.85)
                        };
                        let name = dl.fit(atlas, UI, self.f(14.0), &row.name, inner.right() - ix - self.f(22.0));
                        dl.text(atlas, UI, self.f(14.0), ix + self.f(16.0), baseline, &name, nc, 0.0);
                        self.clicks.push((rr, Click::TreeRow(i)));
                    }
                }
                dl.pop_clip();
                self.scrollbar(dl, &inner, count as f32 * row_h, offset);
                self.wheels.push((tree, Scroll::Tree, row_h));
                if truncated {
                    dl.text(atlas, UI, self.f(11.0), tree.x + self.f(10.0), tree.bottom() - self.f(8.0), "⚠ TREE TRUNCATED", YELLOW, self.f(1.0));
                }
            }
        }

        // ---- content
        let loading_path = app.rv.as_ref().unwrap().file_loading.clone();
        if let Some(p) = loading_path {
            self.sweep_note(dl, atlas, content.x + self.f(16.0), content.y + self.f(30.0), content.w - self.f(32.0), &format!("LOADING {}", p.to_uppercase()));
            return;
        }
        if app.rv.as_ref().unwrap().file.is_none() {
            let rv = app.rv.as_ref().unwrap();
            let x = content.x + self.f(24.0);
            dl.text(atlas, UI_BOLD, self.f(26.0), x, content.y + self.f(52.0), &rv.repo.full_name, with_a(CYAN, 0.35), self.f(3.0));
            let mut y = content.y + self.f(84.0);
            if let Some(d) = rv.repo.description.clone() {
                let msg = dl.fit(atlas, UI, self.f(14.0), &d, content.w - self.f(48.0));
                dl.text(atlas, UI, self.f(14.0), x, y, &msg, DIM, 0.0);
                y += self.f(30.0);
            }
            for line in [
                "↑↓ NAVIGATE · ENTER OPEN · TAB SWITCH PANE",
                "E EDIT · C COMMIT · B BRANCH · A ACTIONS",
                "? FULL KEYMAP",
            ] {
                dl.text(atlas, UI, self.f(12.5), x, y, line, FAINT, self.f(1.5));
                y += self.f(22.0);
            }
            return;
        }

        // Path bar.
        let bar_h = self.f(34.0);
        let bar = RectF::new(content.x + 1.0, content.y + 1.0, content.w - 2.0, bar_h);
        dl.solid(RectF::new(bar.x, bar.bottom(), bar.w, 1.0), BORDER);
        {
            let rv = app.rv.as_ref().unwrap();
            let file = rv.file.as_ref().unwrap();
            let mut x = content.x + self.f(14.0);
            let path = dl.fit(atlas, MONO, self.f(12.5), &file.path, content.w * 0.5);
            x = dl.text(atlas, MONO, self.f(12.5), x, bar.y + self.f(22.0), &path, with_a(TEXT, 0.8), 0.0);
            if file.editor.modified {
                let dot = RectF::new(x + self.f(8.0), bar.y + self.f(14.0), self.f(7.0), self.f(7.0));
                dl.glow(dot, self.f(3.5), with_a(YELLOW, 0.4), self.f(6.0));
                dl.rrect(dot, self.f(3.5), YELLOW, 1.0);
                x = dot.right();
            }
            if file.editing {
                x += self.f(12.0);
                let tag = "EDIT";
                let tw = dl.text_width(atlas, UI_BOLD, self.f(11.0), tag, self.f(2.0));
                let tr = RectF::new(x, bar.y + self.f(8.0), tw + self.f(12.0), self.f(18.0));
                dl.glow(tr, self.f(2.0), with_a(MAGENTA, 0.25), self.f(7.0));
                dl.border(tr, self.f(2.0), 1.0, MAGENTA);
                dl.text(atlas, UI_BOLD, self.f(11.0), tr.x + self.f(6.0), bar.y + self.f(21.5), tag, MAGENTA, self.f(2.0));
            }
            if file.committing {
                dl.text(atlas, UI, self.f(12.0), x + self.f(14.0), bar.y + self.f(22.0), "COMMITTING…", YELLOW, self.f(1.5));
                self.active = true;
            }
        }
        // Action chips (need &mut self, so read flags first).
        let (binary, modified, editing, committing) = {
            let f = app.rv.as_ref().unwrap().file.as_ref().unwrap();
            (f.binary, f.editor.modified, f.editing, f.committing)
        };
        let mut right = content.right() - self.f(12.0);
        if !binary && modified && !committing {
            right = self.chip(dl, atlas, "COMMIT", right, bar.y + bar_h / 2.0, GREEN, Click::CommitBtn, wid(Z_CHIP, 1));
        }
        if !binary && !editing {
            self.chip(dl, atlas, "EDIT", right, bar.y + bar_h / 2.0, CYAN, Click::EditBtn, wid(Z_CHIP, 2));
        }

        let body = RectF::new(
            content.x + self.f(6.0),
            bar.bottom() + self.f(6.0),
            content.w - self.f(12.0),
            content.bottom() - bar.bottom() - self.f(12.0),
        );
        if binary {
            let f = app.rv.as_ref().unwrap().file.as_ref().unwrap();
            dl.text(atlas, UI, self.f(14.0), body.x + self.f(12.0), body.y + self.f(28.0), &format!("BINARY FILE · {} BYTES", f.size), DIM, self.f(1.5));
            return;
        }
        self.editor_body(app, dl, atlas, body);
    }

    fn editor_body(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, body: RectF) {
        let code_px = self.f(13.5);
        let (ascent, lh) = atlas.metrics(MONO, code_px);
        let line_h = (lh * 1.08).ceil();
        let adv = atlas.advance(MONO, code_px, 'M');

        // Sync keyboard-driven row scrolling into the smooth pixel offset.
        {
            let rv = app.rv.as_mut().unwrap();
            let f = rv.file.as_mut().unwrap();
            if f.editor.scroll != self.last_editor_scroll {
                self.last_editor_scroll = f.editor.scroll;
                let s = self.scrolls.entry(skey(Scroll::Content)).or_insert_with(|| Smooth::new(0.0));
                s.target = f.editor.scroll as f32 * line_h;
            }
        }

        let rv = app.rv.as_ref().unwrap();
        let file = rv.file.as_ref().unwrap();
        let ed = &file.editor;
        let total = ed.line_count();
        let digits = total.to_string().len().max(3);
        let gutter = digits as f32 * adv + self.f(20.0);
        let text_rect = RectF::new(body.x + gutter, body.y, body.w - gutter - self.f(10.0), body.h);
        let editing = file.editing && rv.focus == RepoFocus::Content;

        let s = self.scrolls.entry(skey(Scroll::Content)).or_insert_with(|| Smooth::new(0.0));
        let max = (total as f32 * line_h - body.h).max(0.0);
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);
        let hscroll_px = ed.hscroll as f32 * adv;

        dl.push_clip(body);
        let first = (offset / line_h) as usize;
        let vis_rows = (body.h / line_h) as usize + 2;
        let sel = ed.sel_range();
        let caret_on = ((self.time / 530.0) as i64) % 2 == 0;
        for vis in 0..vis_rows {
            let row = first + vis;
            if row >= total {
                break;
            }
            let y = body.y + row as f32 * line_h - offset;
            let baseline = y + ascent;
            let cursor_row = row == ed.cursor.0;

            // Gutter.
            let num = format!("{:>w$}", row + 1, w = digits);
            let nc = if cursor_row && editing { with_a(CYAN, 0.9) } else { FAINT };
            dl.text(atlas, MONO, self.f(11.0), body.x, baseline, &num, nc, 0.0);

            let line = &ed.lines[row];
            // Selection band.
            if let Some((a, b)) = sel {
                if row >= a.0 && row <= b.0 {
                    let x0 = if row == a.0 { ed.col_to_x(row, a.1) as f32 * adv } else { 0.0 };
                    let x1 = if row == b.0 {
                        ed.col_to_x(row, b.1) as f32 * adv
                    } else {
                        (ed.col_to_x(row, line.chars().count()) + 1) as f32 * adv
                    };
                    dl.solid(
                        RectF::new(text_rect.x + x0 - hscroll_px, y, (x1 - x0).max(adv * 0.4), line_h),
                        with_a(CYAN, 0.15),
                    );
                }
            }

            // Syntax-colored runs.
            let state = file.line_states.get(row).copied().unwrap_or(LineState::Normal);
            let spans = match file.lang {
                Some(spec) => highlight::highlight(spec, line, state).0,
                None => Vec::new(),
            };
            let mut span_i = 0;
            let mut run = String::new();
            let mut run_color = TEXT;
            let mut run_start_cell = 0usize;
            let mut cell = 0usize;
            let flush = |dl: &mut DrawList, atlas: &mut Atlas, run: &mut String, start: usize, color: Color| {
                if !run.is_empty() {
                    let x = text_rect.x + start as f32 * adv - hscroll_px;
                    dl.text(atlas, MONO, code_px, x, baseline, run, color, 0.0);
                    run.clear();
                }
            };
            for (ci, ch) in line.chars().enumerate() {
                while span_i < spans.len() && spans[span_i].1 <= ci {
                    span_i += 1;
                }
                let color = spans
                    .get(span_i)
                    .filter(|(st, en, _)| *st <= ci && ci < *en)
                    .map(|(_, _, c)| super::theme::c(*c, 1.0))
                    .unwrap_or(TEXT);
                if ch == '\t' {
                    flush(dl, atlas, &mut run, run_start_cell, run_color);
                    cell += crate::app::editor::TAB_W;
                    run_start_cell = cell;
                    continue;
                }
                if color != run_color && !run.is_empty() {
                    flush(dl, atlas, &mut run, run_start_cell, run_color);
                    run_start_cell = cell;
                }
                if run.is_empty() {
                    run_start_cell = cell;
                    run_color = color;
                }
                run.push(ch);
                cell += 1;
            }
            flush(dl, atlas, &mut run, run_start_cell, run_color);

            // Caret.
            if editing && cursor_row {
                self.active = true;
                if caret_on {
                    let cx = text_rect.x + ed.col_to_x(row, ed.cursor.1) as f32 * adv - hscroll_px;
                    let cr = RectF::new(cx, y + 1.0, self.f(2.0), line_h - 2.0);
                    dl.glow(cr, 1.0, with_a(CYAN, 0.45), self.f(5.0));
                    dl.solid(cr, CYAN);
                }
            }
        }
        dl.pop_clip();
        self.scrollbar(dl, &body, total as f32 * line_h, offset);
        self.wheels.push((body, Scroll::Content, line_h));
        self.editor_geom = Some(EditorGeom {
            rect: text_rect,
            line_h,
            adv,
            scroll_px: offset,
            hscroll_px,
        });
        app.layout.content_text = CellRect::new(
            0,
            0,
            ((text_rect.w / adv) as i32).max(10),
            ((body.h / line_h) as i32).max(3),
        );
        app.layout.gutter = gutter as i32;
    }

    fn actions_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let left = RectF::new(self.f(16.0), top, (w - self.f(44.0)) * 0.46, bottom - top);
        let right = RectF::new(left.right() + self.f(12.0), top, w - left.right() - self.f(28.0), bottom - top);
        self.panel(dl, left);
        self.panel(dl, right);
        dl.text(atlas, UI, self.f(12.0), left.x + self.f(14.0), top + self.f(24.0), "WORKFLOW RUNS", DIM, self.f(2.5));
        dl.text(atlas, UI, self.f(12.0), right.x + self.f(14.0), top + self.f(24.0), "JOBS", DIM, self.f(2.5));

        let row_h = self.f(32.0);
        let list = RectF::new(left.x + self.f(8.0), top + self.f(36.0), left.w - self.f(16.0), left.h - self.f(46.0));
        app.layout.runs_h = (list.h / row_h).max(1.0) as usize;

        enum RState {
            Note(String, bool),
            Ready,
        }
        let rstate = match &app.rv.as_ref().unwrap().runs {
            Loadable::Loading | Loadable::Idle => RState::Note("FETCHING RUNS…".into(), false),
            Loadable::Failed(e) => RState::Note(e.clone(), true),
            Loadable::Ready(r) if r.is_empty() => RState::Note("NO WORKFLOW RUNS".into(), false),
            Loadable::Ready(_) => RState::Ready,
        };
        match rstate {
            RState::Note(msg, err) => {
                if err {
                    let m = dl.fit(atlas, UI, self.f(13.0), &msg, list.w);
                    dl.text(atlas, UI, self.f(13.0), list.x + self.f(6.0), list.y + self.f(20.0), &m, RED, 0.0);
                } else {
                    self.sweep_note(dl, atlas, list.x + self.f(6.0), list.y + self.f(20.0), list.w, &msg);
                }
            }
            RState::Ready => {
                let (sel, count) = {
                    let rv = app.rv.as_ref().unwrap();
                    (rv.runs_sel.min(rv.runs.ready().unwrap().len().saturating_sub(1)), rv.runs.ready().unwrap().len())
                };
                let offset = self.list_scroll(Scroll::Runs, Z_RUN, sel, count, row_h, list.h);
                dl.push_clip(list);
                {
                    let rv = app.rv.as_ref().unwrap();
                    let runs = rv.runs.ready().unwrap();
                    let first = (offset / row_h) as usize;
                    for vis in 0..(list.h / row_h) as usize + 2 {
                        let i = first + vis;
                        if i >= runs.len() {
                            break;
                        }
                        let run = &runs[i];
                        let y = list.y + i as f32 * row_h - offset;
                        let rr = RectF::new(list.x, y, list.w, row_h - 2.0);
                        let selected = i == sel;
                        let hv = self.hover_amt(wid(Z_RUN, i), rr.contains(self.hot.0, self.hot.1));
                        let a = if selected { 0.12 } else { 0.05 * hv };
                        if a > 0.005 {
                            dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                        }
                        let (icon, rgb) = run_icon(&run.status, run.conclusion.as_deref());
                        let mut ic = super::theme::c(rgb, 1.0);
                        if run.status == "in_progress" {
                            ic[3] = 0.5 + 0.5 * ((self.time * 0.005).sin() as f32);
                            self.active = true;
                        }
                        let baseline = y + self.f(21.0);
                        dl.text(atlas, MONO, self.f(13.0), rr.x + self.f(8.0), baseline, &icon.to_string(), ic, 0.0);
                        let title = run
                            .display_title
                            .clone()
                            .or_else(|| run.name.clone())
                            .unwrap_or_else(|| format!("run {}", run.id));
                        let label = format!("#{} {}", run.run_number, title);
                        let main_w = rr.w * 0.55;
                        let fitted = dl.fit(atlas, UI, self.f(13.5), &label, main_w);
                        let mut x = dl.text(atlas, UI, self.f(13.5), rr.x + self.f(26.0), baseline, &fitted, TEXT, 0.0);
                        if let Some(b) = &run.head_branch {
                            let bb = dl.fit(atlas, MONO, self.f(11.0), b, rr.w * 0.2);
                            x = dl.text(atlas, MONO, self.f(11.0), x + self.f(10.0), baseline, &bb, with_a(MAGENTA, 0.8), 0.0);
                        }
                        let meta = format!("{} {}", run.event, crate::app::fmt_age(&run.created_at));
                        let mw = dl.text_width(atlas, UI, self.f(11.0), &meta, 0.0);
                        let mx = (rr.right() - mw - self.f(8.0)).max(x + self.f(8.0));
                        dl.text(atlas, UI, self.f(11.0), mx, baseline, &meta, DIM, 0.0);
                        self.clicks.push((rr, Click::Run(i)));
                    }
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, count as f32 * row_h, offset);
                self.wheels.push((left, Scroll::Runs, row_h));
            }
        }

        // Jobs pane.
        let jlist = RectF::new(right.x + self.f(8.0), top + self.f(36.0), right.w - self.f(16.0), right.h - self.f(46.0));
        let jrow = self.f(26.0);
        app.layout.jobs_h = (jlist.h / jrow).max(1.0) as usize;
        let jstate = app.rv.as_ref().unwrap().jobs.as_ref().map(|(_, l)| match l {
            Loadable::Loading | Loadable::Idle => 0,
            Loadable::Failed(_) => 1,
            Loadable::Ready(_) => 2,
        });
        match jstate {
            None => {
                dl.text(atlas, UI, self.f(13.0), jlist.x + self.f(6.0), jlist.y + self.f(20.0), "PRESS ENTER ON A RUN TO LOAD ITS JOBS", FAINT, self.f(1.0));
            }
            Some(0) => self.sweep_note(dl, atlas, jlist.x + self.f(6.0), jlist.y + self.f(20.0), jlist.w, "FETCHING JOBS…"),
            Some(1) => {
                let e = match app.rv.as_ref().unwrap().jobs.as_ref() {
                    Some((_, Loadable::Failed(e))) => e.clone(),
                    _ => String::new(),
                };
                let m = dl.fit(atlas, UI, self.f(13.0), &e, jlist.w);
                dl.text(atlas, UI, self.f(13.0), jlist.x + self.f(6.0), jlist.y + self.f(20.0), &m, RED, 0.0);
            }
            _ => {
                let scroll_rows = app.rv.as_ref().unwrap().jobs_scroll;
                dl.push_clip(jlist);
                let rv = app.rv.as_ref().unwrap();
                if let Some((_, Loadable::Ready(jobs))) = &rv.jobs {
                    let mut lines: Vec<(f32, char, Color, String, bool)> = Vec::new();
                    for job in jobs {
                        let (icon, rgb) = run_icon(&job.status, job.conclusion.as_deref());
                        lines.push((0.0, icon, super::theme::c(rgb, 1.0), job.name.clone(), true));
                        for step in &job.steps {
                            let (si, srgb) = run_icon(&step.status, step.conclusion.as_deref());
                            lines.push((self.f(18.0), si, super::theme::c(srgb, 1.0), step.name.clone(), false));
                        }
                    }
                    for (vis, li) in (scroll_rows..lines.len()).enumerate() {
                        let y = jlist.y + vis as f32 * jrow;
                        if y > jlist.bottom() {
                            break;
                        }
                        let (indent, icon, ic, name, bold) = &lines[li];
                        let baseline = y + self.f(17.0);
                        dl.text(atlas, MONO, self.f(12.0), jlist.x + self.f(4.0) + indent, baseline, &icon.to_string(), *ic, 0.0);
                        let font = if *bold { UI_BOLD } else { UI };
                        let fitted = dl.fit(atlas, font, self.f(13.0), name, jlist.w - indent - self.f(30.0));
                        dl.text(atlas, font, self.f(13.0), jlist.x + self.f(22.0) + indent, baseline, &fitted, if *bold { TEXT } else { with_a(TEXT, 0.75) }, 0.0);
                    }
                    self.wheels.push((right, Scroll::Jobs, jrow));
                }
                dl.pop_clip();
            }
        }
    }

    fn scrollbar(&self, dl: &mut DrawList, area: &RectF, content_h: f32, offset: f32) {
        if content_h <= area.h {
            return;
        }
        let track = RectF::new(area.right() - self.f(4.0), area.y, self.f(3.0), area.h);
        dl.rrect(track, self.f(1.5), with_a(CYAN, 0.06), 1.0);
        let th = (area.h / content_h * area.h).max(self.f(24.0));
        let max = content_h - area.h;
        let ty = area.y + (offset / max) * (area.h - th);
        dl.rrect(RectF::new(track.x, ty, track.w, th), self.f(1.5), with_a(CYAN, 0.45), 1.0);
    }

    fn status_bar(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        let bh = self.f(28.0);
        let y = h - bh;
        dl.solid(RectF::new(0.0, y, w, bh), BG1);
        dl.solid(RectF::new(0.0, y, w, 1.0), BORDER_BRIGHT);
        let hints = match app.route {
            Route::Auth => "[ENTER] CONTINUE",
            Route::Repos => {
                if app.filter_active {
                    "[ENTER] APPLY · [ESC] CLEAR"
                } else if app.repo_source != RepoSource::Mine {
                    "[/] FILTER · [O] OPEN · [S] SORT · [F] FORKS · [X] ARCHIVED · [ESC] MY REPOS"
                } else {
                    "[/] FILTER · [O] OPEN REPO/ORG · [S] SORT · [F] FORKS · [X] ARCHIVED · [I] AGENT · [?] HELP"
                }
            }
            Route::Repo => {
                if app.in_editor() {
                    "[CTRL+S] COMMIT · [CTRL+Z] UNDO · [ESC] VIEW MODE"
                } else if app.rv.as_ref().map(|rv| rv.tab == Tab::Actions).unwrap_or(false) {
                    "[ENTER] JOBS · [R] REFRESH · [A/ESC] CODE"
                } else {
                    "[ENTER] OPEN · [/] FIND · [G] CODE SEARCH · [E] EDIT · [B] BRANCH · [A] ACTIONS · [I] AGENT · [ESC] BACK"
                }
            }
            Route::Agent => {
                if app.anthropic_key.is_none() {
                    "[ENTER] SAVE · [TAB] SWITCH FIELD · [ESC] BACK"
                } else if app.agent.busy {
                    "[ESC] CANCEL"
                } else {
                    "[ENTER] SEND · [ESC] BACK"
                }
            }
        };
        let baseline = y + self.f(19.0);
        dl.text(atlas, UI, self.f(12.0), self.f(16.0), baseline, hints, with_a(DIM, 0.9), self.f(1.0));
        let rate = crate::fetch::RATE_LIMIT
            .with(|c| c.get())
            .map(|(r, l)| format!("API {}/{}", r, l))
            .unwrap_or_default();
        let tw = dl.text_width(atlas, UI, self.f(12.0), &rate, self.f(1.0));
        dl.text(atlas, UI, self.f(12.0), w - tw - self.f(16.0), baseline, &rate, DIM, self.f(1.0));
    }

    fn overlay(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        self.overlay_t.target = if app.overlay.is_some() { 1.0 } else { 0.0 };
        if app.overlay.is_none() {
            self.overlay_t.snap(0.0);
            // Reset picker scroll state so the next open re-anchors on the
            // current selection instead of inheriting a stale offset.
            self.scrolls.remove(&skey(Scroll::Overlay));
            self.last_sel.remove(&Z_OVER);
            return;
        }
        if self.overlay_t.tick_n(self.dt, 14.0) {
            self.active = true;
        }
        let k = ease_out(self.overlay_t.v);
        // Overlay swallows all main-screen input.
        self.clicks.clear();
        self.wheels.clear();
        self.editor_geom = None;

        dl.solid(RectF::new(0.0, 0.0, w, h), [0.0, 0.0, 0.0, 0.55 * k]);
        let pw = self.f(560.0).min(w - self.f(40.0));
        let lift = (1.0 - k) * self.f(16.0);

        let title_of = |o: &Overlay| match o {
            Overlay::Commit(_) => "COMMIT",
            Overlay::OpenRepo(_) => "OPEN REPOSITORY",
            Overlay::BranchPick { .. } => "SWITCH BRANCH",
            Overlay::FileSearch { .. } => "FIND FILE",
            Overlay::CodeSearch { .. } => "CODE SEARCH",
            Overlay::Confirm { .. } => "CONFIRM",
            Overlay::Help => "KEYMAP",
        };
        let title = app.overlay.as_ref().map(title_of).unwrap_or("").to_string();

        match app.overlay.as_ref().unwrap() {
            Overlay::Commit(input) => {
                let input = input.clone_shallow();
                let branch = app.rv.as_ref().map(|rv| rv.branch.clone()).unwrap_or_default();
                let path = app
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.file.as_ref())
                    .map(|f| f.path.clone())
                    .unwrap_or_default();
                let ph = self.f(196.0);
                let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
                self.overlay_panel(dl, atlas, r, &format!("{} → {}", title, branch.to_uppercase()));
                dl.text(atlas, MONO, self.f(12.0), r.x + self.f(24.0), r.y + self.f(64.0), &path, DIM, 0.0);
                let field = RectF::new(r.x + self.f(24.0), r.y + self.f(80.0), r.w - self.f(48.0), self.f(40.0));
                self.input_field(dl, atlas, &input, field, true);
                dl.text(atlas, UI, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "[ENTER] COMMIT · [ESC] ABORT", FAINT, self.f(1.5));
            }
            Overlay::OpenRepo(input) => {
                let input = input.clone_shallow();
                let ph = self.f(190.0);
                let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
                self.overlay_panel(dl, atlas, r, &title);
                dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), r.y + self.f(64.0), "owner/repo or organization:", DIM, self.f(1.5));
                let field = RectF::new(r.x + self.f(24.0), r.y + self.f(76.0), r.w - self.f(48.0), self.f(40.0));
                self.input_field(dl, atlas, &input, field, true);
                dl.text(atlas, UI, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "[ENTER] OPEN · [ESC] ABORT", FAINT, self.f(1.5));
            }
            Overlay::BranchPick { sel, scroll } => {
                let (sel, scroll) = (*sel, *scroll);
                let branches = app
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.branches.ready())
                    .cloned()
                    .unwrap_or_default();
                let current = app.rv.as_ref().map(|rv| rv.branch.clone()).unwrap_or_default();
                let row_h = self.f(30.0);
                let list_h = (branches.len() as f32 * row_h).min(h * 0.5);
                let ph = list_h + self.f(86.0);
                let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
                self.overlay_panel(dl, atlas, r, &title);
                let list = RectF::new(r.x + self.f(16.0), r.y + self.f(54.0), r.w - self.f(32.0), list_h);
                app.layout.overlay_h = (list.h / row_h).max(1.0) as usize;
                let _ = scroll;
                // Same rule as the main lists: re-anchor on the selection
                // only when it moves, so wheel scrolling is free.
                let offset = self.list_scroll(Scroll::Overlay, Z_OVER, sel, branches.len(), row_h, list.h);
                dl.push_clip(list);
                let first = (offset / row_h) as usize;
                for vis in 0..(list.h / row_h) as usize + 2 {
                    let i = first + vis;
                    if i >= branches.len() {
                        break;
                    }
                    let y = list.y + i as f32 * row_h - offset;
                    let rr = RectF::new(list.x, y, list.w, row_h - 2.0);
                    let hv = self.hover_amt(wid(Z_OVER, i), rr.contains(self.mouse.0, self.mouse.1));
                    let a = if i == sel { 0.13 } else { 0.06 * hv };
                    if a > 0.005 {
                        dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                    }
                    let baseline = y + self.f(20.0);
                    if branches[i].name == current {
                        dl.text(atlas, MONO, self.f(12.0), rr.x + self.f(8.0), baseline, "●", GREEN, 0.0);
                    }
                    let name = dl.fit(atlas, MONO, self.f(13.0), &branches[i].name, rr.w - self.f(40.0));
                    dl.text(atlas, MONO, self.f(13.0), rr.x + self.f(26.0), baseline, &name, TEXT, 0.0);
                    self.clicks.push((rr, Click::OverlayItem(i)));
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, branches.len() as f32 * row_h, offset);
                self.wheels.push((list, Scroll::Overlay, row_h));
            }
            Overlay::FileSearch { input, sel } => {
                let input = input.clone_shallow();
                let sel = *sel;
                let results: Vec<String> = app
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.tree.ready())
                    .map(|t| {
                        crate::app::search_tree(t, &input.text)
                            .into_iter()
                            .map(|i| t[i].path.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let row_h = self.f(26.0);
                let visible = results.len().min(12);
                let list_h = visible.max(1) as f32 * row_h;
                let pw2 = self.f(680.0).min(w - self.f(40.0));
                let ph = list_h + self.f(118.0);
                // Anchored near the top like a command palette.
                let r = RectF::new((w - pw2) / 2.0, h * 0.14 + lift, pw2, ph);
                self.overlay_panel(dl, atlas, r, &title);
                let count = format!("{} MATCHES", results.len());
                let cw = dl.text_width(atlas, UI, self.f(11.0), &count, self.f(1.5));
                dl.text(atlas, UI, self.f(11.0), r.right() - cw - self.f(24.0), r.y + self.f(34.0), &count, FAINT, self.f(1.5));
                let field = RectF::new(r.x + self.f(24.0), r.y + self.f(48.0), r.w - self.f(48.0), self.f(38.0));
                self.input_field(dl, atlas, &input, field, true);

                let sel = sel.min(results.len().saturating_sub(1));
                let first = if sel >= visible && visible > 0 { sel + 1 - visible } else { 0 };
                let y0 = field.bottom() + self.f(12.0);
                for vis in 0..visible {
                    let i = first + vis;
                    let y = y0 + vis as f32 * row_h;
                    let rr = RectF::new(r.x + self.f(16.0), y, r.w - self.f(32.0), row_h - 2.0);
                    let hv = self.hover_amt(wid(Z_OVER, 1000 + i), rr.contains(self.mouse.0, self.mouse.1));
                    let a = if i == sel { 0.13 } else { 0.06 * hv };
                    if a > 0.005 {
                        dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                    }
                    let baseline = y + self.f(18.0);
                    let path = dl.fit(atlas, MONO, self.f(12.0), &results[i], rr.w - self.f(20.0));
                    // Directory part dim, filename bright.
                    let (dir, name) = match path.rsplit_once('/') {
                        Some((d, n)) => (format!("{}/", d), n.to_string()),
                        None => (String::new(), path.clone()),
                    };
                    let mut x = rr.x + self.f(10.0);
                    if !dir.is_empty() {
                        x = dl.text(atlas, MONO, self.f(12.0), x, baseline, &dir, FAINT, 0.0);
                    }
                    let nc = if i == sel { CYAN } else { with_a(TEXT, 0.9) };
                    dl.text(atlas, MONO, self.f(12.0), x, baseline, &name, nc, 0.0);
                    self.clicks.push((rr, Click::OverlayItem(i)));
                }
                if results.is_empty() {
                    dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "no matching files", FAINT, self.f(1.0));
                }
            }
            Overlay::CodeSearch { input, sel, searched, results } => {
                let input = input.clone_shallow();
                let sel = *sel;
                let armed = input.text.trim() != searched.as_str() || searched.is_empty();
                enum RState {
                    Idle,
                    Loading,
                    Failed(String),
                    Hits(Vec<(String, String, Option<(usize, usize)>)>),
                }
                let state = match results {
                    Loadable::Idle => RState::Idle,
                    Loadable::Loading => RState::Loading,
                    Loadable::Failed(e) => RState::Failed(e.clone()),
                    Loadable::Ready(h) => RState::Hits(
                        h.iter().map(|c| (c.path.clone(), c.line.clone(), c.range)).collect(),
                    ),
                };
                let hits_len = match &state {
                    RState::Hits(h) => h.len(),
                    _ => 0,
                };
                let row_h = self.f(40.0);
                let visible = hits_len.min(8);
                let list_h = visible.max(1) as f32 * row_h;
                let pw2 = self.f(720.0).min(w - self.f(40.0));
                let ph = list_h + self.f(140.0);
                let r = RectF::new((w - pw2) / 2.0, h * 0.12 + lift, pw2, ph);
                self.overlay_panel(dl, atlas, r, &title);
                if hits_len > 0 {
                    let count = format!("{} RESULTS", hits_len);
                    let cw = dl.text_width(atlas, UI, self.f(11.0), &count, self.f(1.5));
                    dl.text(atlas, UI, self.f(11.0), r.right() - cw - self.f(24.0), r.y + self.f(34.0), &count, FAINT, self.f(1.5));
                }
                let field = RectF::new(r.x + self.f(24.0), r.y + self.f(48.0), r.w - self.f(48.0), self.f(38.0));
                self.input_field(dl, atlas, &input, field, true);
                let hint = if armed {
                    "[ENTER] SEARCH · GITHUB CODE SEARCH · DEFAULT BRANCH ONLY"
                } else {
                    "[ENTER] OPEN · ↑↓ SELECT · EDIT QUERY TO SEARCH AGAIN"
                };
                dl.text(atlas, UI, self.f(11.0), r.x + self.f(24.0), r.y + ph - self.f(18.0), hint, FAINT, self.f(1.5));

                let y0 = field.bottom() + self.f(12.0);
                match state {
                    RState::Idle => {
                        dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "type a query, Enter to search", FAINT, self.f(1.0));
                    }
                    RState::Loading => {
                        self.sweep_note(dl, atlas, r.x + self.f(24.0), y0 + self.f(16.0), r.w - self.f(48.0), "SEARCHING…");
                    }
                    RState::Failed(e) => {
                        let m = dl.fit(atlas, UI, self.f(13.0), &e, r.w - self.f(48.0));
                        dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), &m, RED, 0.0);
                    }
                    RState::Hits(hits) => {
                        if hits.is_empty() {
                            dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "no results", FAINT, self.f(1.0));
                        }
                        let sel = sel.min(hits.len().saturating_sub(1));
                        let first = if sel >= visible && visible > 0 { sel + 1 - visible } else { 0 };
                        for vis in 0..visible {
                            let i = first + vis;
                            let (path, line, range) = &hits[i];
                            let y = y0 + vis as f32 * row_h;
                            let rr = RectF::new(r.x + self.f(16.0), y, r.w - self.f(32.0), row_h - 4.0);
                            let hv = self.hover_amt(wid(Z_OVER, 2000 + i), rr.contains(self.mouse.0, self.mouse.1));
                            let a = if i == sel { 0.13 } else { 0.06 * hv };
                            if a > 0.005 {
                                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                            }
                            dl.push_clip(rr);
                            // Line 1: path (dir dim, filename bright).
                            let b1 = y + self.f(15.0);
                            let (dir, name) = match path.rsplit_once('/') {
                                Some((d, n)) => (format!("{}/", d), n.to_string()),
                                None => (String::new(), path.clone()),
                            };
                            let mut x = rr.x + self.f(10.0);
                            if !dir.is_empty() {
                                x = dl.text(atlas, MONO, self.f(11.5), x, b1, &dir, FAINT, 0.0);
                            }
                            dl.text(atlas, MONO, self.f(11.5), x, b1, &name, if i == sel { CYAN } else { with_a(TEXT, 0.9) }, 0.0);
                            // Line 2: matched line with the hit highlighted.
                            let b2 = y + self.f(31.0);
                            let mut x = rr.x + self.f(10.0);
                            match range {
                                Some((cs, ce)) if *ce > *cs => {
                                    let pre: String = line.chars().take(*cs).collect();
                                    let hit: String = line.chars().skip(*cs).take(ce - cs).collect();
                                    let post: String = line.chars().skip(*ce).collect();
                                    x = dl.text(atlas, MONO, self.f(11.5), x, b2, &pre, DIM, 0.0);
                                    let hw = dl.text_width(atlas, MONO, self.f(11.5), &hit, 0.0);
                                    dl.solid(RectF::new(x, b2 - self.f(11.0), hw, self.f(15.0)), with_a(MAGENTA, 0.18));
                                    x = dl.text(atlas, MONO, self.f(11.5), x, b2, &hit, MAGENTA, 0.0);
                                    dl.text(atlas, MONO, self.f(11.5), x, b2, &post, DIM, 0.0);
                                }
                                _ => {
                                    dl.text(atlas, MONO, self.f(11.5), x, b2, line, DIM, 0.0);
                                }
                            }
                            dl.pop_clip();
                            self.clicks.push((rr, Click::OverlayItem(i)));
                        }
                    }
                }
            }
            Overlay::Confirm { msg, .. } => {
                let msg = msg.clone();
                let ph = self.f(150.0);
                let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
                self.overlay_panel(dl, atlas, r, &title);
                let m = dl.fit(atlas, UI, self.f(14.5), &msg, r.w - self.f(48.0));
                dl.text(atlas, UI, self.f(14.5), r.x + self.f(24.0), r.y + self.f(72.0), &m, TEXT, 0.0);
                dl.text(atlas, UI, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "[ENTER/Y] CONFIRM · [ESC/N] ABORT", FAINT, self.f(1.5));
            }
            Overlay::Help => {
                let lines: [(&str, &str); 18] = [
                    ("GLOBAL", ""),
                    ("?", "this help · esc closes"),
                    ("REPOSITORIES", ""),
                    ("/ O R ENTER", "filter · open repo or org · reload · open"),
                    ("S ⇧S F X", "cycle sort · flip order · toggle forks/archived"),
                    ("CODE", ""),
                    ("↑↓ ←→ ENTER", "navigate tree · expand/collapse · open"),
                    ("/", "find file across the whole tree"),
                    ("G", "code search via GitHub API (needs token)"),
                    ("TAB", "switch tree/content pane"),
                    ("E C B A", "edit · commit · branch · actions"),
                    ("EDITOR", ""),
                    ("CTRL+S", "commit · ctrl+z undo · ctrl+y redo"),
                    ("SHIFT+ARROWS", "select · esc back to view mode"),
                    ("ACTIONS", ""),
                    ("ENTER R", "load jobs · refresh"),
                    ("AGENT", ""),
                    ("I", "AI agent window · drives the GitHub API for you"),
                ];
                let row = self.f(24.0);
                let ph = lines.len() as f32 * row + self.f(86.0);
                let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
                self.overlay_panel(dl, atlas, r, &title);
                for (i, (k, v)) in lines.iter().enumerate() {
                    let y = r.y + self.f(64.0) + i as f32 * row;
                    if v.is_empty() {
                        dl.text(atlas, UI_BOLD, self.f(12.0), r.x + self.f(24.0), y, k, CYAN, self.f(2.5));
                    } else {
                        dl.text(atlas, MONO, self.f(12.0), r.x + self.f(36.0), y, k, with_a(MAGENTA, 0.9), 0.0);
                        dl.text(atlas, UI, self.f(13.0), r.x + self.f(190.0), y, v, with_a(TEXT, 0.8), 0.0);
                    }
                }
            }
        }
    }

    fn overlay_panel(&mut self, dl: &mut DrawList, atlas: &mut Atlas, r: RectF, title: &str) {
        dl.glow(r, self.f(4.0), with_a(CYAN, 0.07), self.f(30.0));
        dl.rrect(r, self.f(4.0), BG1, 1.0);
        dl.border(r, self.f(4.0), 1.0, BORDER_BRIGHT);
        self.brackets(dl, r, self.f(12.0), with_a(CYAN, 0.7));
        dl.text(atlas, UI_BOLD, self.f(16.0), r.x + self.f(24.0), r.y + self.f(34.0), title, CYAN, self.f(3.0));
    }

    fn toast(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        self.toast_t.target = if app.toast.is_some() { 1.0 } else { 0.0 };
        if app.toast.is_none() {
            self.toast_t.snap(0.0);
            return;
        }
        if self.toast_t.tick_n(self.dt, 13.0) {
            self.active = true;
        }
        let k = ease_out(self.toast_t.v);
        let (msg, is_err) = app.toast.clone().unwrap();
        let color = if is_err { RED } else { GREEN };
        let px = self.f(13.0);
        let tw = dl.text_width(atlas, UI, px, &msg, self.f(0.5));
        let r = RectF::new(
            w - tw - self.f(44.0) + (1.0 - k) * self.f(60.0),
            h - self.f(64.0),
            tw + self.f(28.0),
            self.f(30.0),
        );
        dl.glow(r, self.f(3.0), with_a(color, 0.25 * k), self.f(10.0));
        dl.rrect(r, self.f(3.0), BG2, 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.9 * k));
        let (asc, lh) = atlas.metrics(UI, px);
        dl.text(atlas, UI, px, r.x + self.f(14.0), r.y + (r.h - lh) / 2.0 + asc, &msg, with_a(TEXT, k), self.f(0.5));
    }
}

/// GitHub-style language dot colors, nudged lighter where the official
/// color would vanish on a near-black background.
fn lang_color(lang: &str) -> Color {
    match lang {
        "Rust" => rgba(0xde, 0xa5, 0x84, 1.0),
        "JavaScript" => rgba(0xf1, 0xe0, 0x5a, 1.0),
        "TypeScript" => rgba(0x31, 0x78, 0xc6, 1.0),
        "Python" => rgba(0x4b, 0x8b, 0xbe, 1.0),
        "Go" => rgba(0x00, 0xad, 0xd8, 1.0),
        "C" => rgba(0x9a, 0x9a, 0x9a, 1.0),
        "C++" => rgba(0xf3, 0x4b, 0x7d, 1.0),
        "C#" => rgba(0x2f, 0xa8, 0x35, 1.0),
        "Java" => rgba(0xb0, 0x72, 0x19, 1.0),
        "Kotlin" => rgba(0xa9, 0x7b, 0xff, 1.0),
        "Swift" => rgba(0xf0, 0x51, 0x38, 1.0),
        "Ruby" => rgba(0xcc, 0x34, 0x2d, 1.0),
        "PHP" => rgba(0x77, 0x7b, 0xb4, 1.0),
        "Shell" | "Bash" => rgba(0x89, 0xe0, 0x51, 1.0),
        "HTML" => rgba(0xe3, 0x4c, 0x26, 1.0),
        "CSS" => rgba(0x7e, 0x5c, 0xb8, 1.0),
        "Zig" => rgba(0xec, 0x91, 0x5c, 1.0),
        "Lua" => rgba(0x55, 0x66, 0xcc, 1.0),
        "Dockerfile" => rgba(0x5b, 0x7c, 0x88, 1.0),
        "Vue" => rgba(0x41, 0xb8, 0x83, 1.0),
        "Dart" => rgba(0x00, 0xb4, 0xab, 1.0),
        _ => DIM,
    }
}

/// Map a code-fence language tag to a highlighter spec via its usual file
/// extension.
fn lang_for_tag(tag: &str) -> Option<&'static highlight::LangSpec> {
    let ext = match tag.to_ascii_lowercase().as_str() {
        "" => return None,
        "rust" => "rs",
        "python" => "py",
        "javascript" | "node" => "js",
        "typescript" => "ts",
        "golang" => "go",
        "shell" | "bash" | "zsh" | "console" => "sh",
        "yaml" => "yml",
        "markdown" => "md",
        "c++" => "cpp",
        t => return highlight::lang_for_path(&format!("f.{}", t)),
    };
    highlight::lang_for_path(&format!("f.{}", ext))
}

/// Wrap text to a mono-column budget: split on newlines, then soft-wrap on
/// spaces, hard-breaking words longer than one line.
fn wrap_chars(text: &str, cols: usize, out: &mut Vec<String>) {
    for raw in text.split('\n') {
        if raw.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut count = 0usize;
        for word in raw.split(' ') {
            let wlen = word.chars().count();
            if count > 0 && count + 1 + wlen > cols {
                out.push(std::mem::take(&mut line));
                count = 0;
            }
            if count > 0 {
                line.push(' ');
                count += 1;
            }
            if wlen > cols {
                for ch in word.chars() {
                    if count >= cols {
                        out.push(std::mem::take(&mut line));
                        count = 0;
                    }
                    line.push(ch);
                    count += 1;
                }
            } else {
                line.push_str(word);
                count += wlen;
            }
        }
        out.push(line);
    }
}

fn busy(app: &App) -> bool {
    if app.auth_busy || app.agent.busy || matches!(app.repos, Loadable::Loading) {
        return true;
    }
    if let Some(rv) = &app.rv {
        if matches!(rv.branches, Loadable::Loading)
            || matches!(rv.tree, Loadable::Loading)
            || matches!(rv.runs, Loadable::Loading)
            || rv.file_loading.is_some()
            || matches!(rv.jobs, Some((_, Loadable::Loading)))
            || rv.file.as_ref().map(|f| f.committing).unwrap_or(false)
        {
            return true;
        }
    }
    false
}
