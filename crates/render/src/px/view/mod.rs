//! The cyberpunk HUD: draws every screen in pixel space over the shared App
//! state machine. Owns smooth scrolling, hover animation, hit regions, and
//! the editor's pixel geometry; the App owns all actual state.

use std::collections::HashMap;

use crate::app::run_icon;
use crate::app::{
    AgentItem, App, Click, CommitForm, CommitTarget, Detail, Loadable, Overlay, RepoFocus,
    RepoSource, Route, Scroll, SearchScope, Staged, Tab,
};
use crate::highlight::{self, LineState};
use crate::ui::grid::Rect as CellRect;
use crate::ui::lineinput::LineInput;

use super::anim::{ease_out, Smooth};
use super::atlas::{Atlas, MONO, UI, UI_BOLD};
use super::draw::{DrawList, RectF};
use super::theme::*;

mod actions_pane;
mod agent_draw;
mod agent_pane;
mod agent_text;
mod actions_log;
mod chrome;
mod context_menu;
mod editor_pane;
mod frame;
mod hints;
mod input;
mod issue_detail_body;
mod issue_detail_pane;
mod issues_pane;
mod links;
mod md;
mod overlay_commit;
mod overlay_grep;
mod overlay_pick;
mod overlays;
mod repo_card;
mod repo_pane;
mod repos_pane;
mod scroll;
#[cfg(test)]
mod tests;
mod text;
mod tree_pane; mod widgets;

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
    /// Resizing the Actions runs/jobs split.
    ActionsSplit,
    /// Extending the job-log text selection.
    JobLog,
    /// Dragging the job-log scrollbar thumb.
    LogScroll,
}

/// Job-log view geometry from the last frame, for hit-testing selection, the
/// scrollbar drag, and search jumps.
#[derive(Clone, Copy)]
struct LogGeom {
    area: RectF,
    lh: f32,
    adv: f32,
    scroll: usize,
    lines: usize,
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
    /// Agent scroll extent on the previous frame — "was the user at the
    /// bottom?" must be judged against the extent before a content change.
    last_agent_max: f32,
    overlay_t: Smooth,
    toast_t: Smooth,
    route_t: Smooth,
    last_route: u8,
    tab_x: Smooth,
    tab_w: Smooth,
    clicks: Vec<(RectF, Click)>,
    wheels: Vec<(RectF, Scroll, f32, f32)>, // rect, target, row_h, max scroll px
    /// Context-menu item hit-regions from the last frame: (rect, item index).
    menu_rects: Vec<(RectF, usize)>,
    /// Tree-pane rect from the last frame, for right-click empty-space hits.
    tree_rect: Option<RectF>,
    editor_geom: Option<EditorGeom>,
    /// Agent transcript layout from the last frame: inner rect, row height,
    /// mono advance, scroll offset — plus the wrapped text for hit-testing
    /// and clipboard copy.
    agent_geom: Option<(RectF, f32, f32, f32)>,
    agent_lines: Vec<String>,
    /// Per wrapped line: logical source-line id; wrapped segments of one
    /// source line share it. None marks label/separator decoration lines,
    /// which are excluded from copies.
    agent_src: Vec<Option<u32>>,
    /// Per wrapped line: measured char-boundary x offsets, only for lines
    /// whose drawn advances aren't uniform mono cells (labels, non-ASCII).
    agent_xs: Vec<Option<Vec<f32>>>,
    /// Transcript selection: (anchor, head) as (line, col), unnormalized.
    agent_sel: Option<((usize, usize), (usize, usize))>,
    /// Distinct hyperlink targets for the transcript drawn this frame;
    /// `Click::OpenUrl` carries an index into this table.
    link_urls: Vec<String>,
    /// Actions runs/jobs split ratio (left pane fraction), drag-adjustable.
    actions_split: f32,
    /// The draggable splitter handle rect, and the (x0, total_w) span used to
    /// convert a drag x into a ratio — from the last Actions frame.
    actions_split_hit: Option<(RectF, f32, f32)>,
    /// Job-log selection (anchor, head) as (line, col), unnormalized.
    log_sel: Option<((usize, usize), (usize, usize))>,
    /// Job-log layout from the last frame (selection / scrollbar / search).
    log_geom: Option<LogGeom>,
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
        Scroll::Issues => 7, Scroll::Detail => 8,
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
// File/code search rows get their own zones: sharing Z_OVER with index
// offsets collides once the branch list outgrows the offset.
const Z_FILE: u8 = 7;
const Z_GREP: u8 = 8;
const Z_MENU: u8 = 9;
const Z_ISSUE: u8 = 10; const Z_DETAIL: u8 = 11;

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
            last_agent_max: 0.0,
            overlay_t: Smooth::new(0.0),
            toast_t: Smooth::new(0.0),
            route_t: Smooth::new(1.0),
            last_route: 255,
            tab_x: Smooth::new(0.0),
            tab_w: Smooth::new(0.0),
            clicks: Vec::new(),
            wheels: Vec::new(),
            menu_rects: Vec::new(),
            tree_rect: None,
            editor_geom: None,
            agent_geom: None,
            agent_lines: Vec::new(),
            agent_src: Vec::new(),
            agent_xs: Vec::new(),
            agent_sel: None,
            link_urls: Vec::new(),
            actions_split: 0.46,
            actions_split_hit: None,
            log_sel: None,
            log_geom: None,
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
}
