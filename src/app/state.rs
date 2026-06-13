//! Shared state types: routes, list sources, focus, overlays, hit-regions,
//! layout, and the open-file model.

use crate::github::Repo;
use crate::highlight::{LangSpec, LineState};
use crate::ui::grid::Rect;
use crate::ui::lineinput::LineInput;

use super::editor::Editor;

pub enum Loadable<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loadable<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Loadable::Ready(t) => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Auth,
    Repos,
    Repo,
    Agent,
}

/// Whose repositories the Repos screen is listing.
#[derive(Clone, PartialEq, Eq)]
pub enum RepoSource {
    Mine,
    Org(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepoSort {
    Pushed,
    Name,
    Stars,
    Forks,
}

impl RepoSort {
    pub fn label(self) -> &'static str {
        match self {
            RepoSort::Pushed => "PUSHED",
            RepoSort::Name => "NAME",
            RepoSort::Stars => "STARS",
            RepoSort::Forks => "FORKS",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Code,
    Actions,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepoFocus {
    Tree,
    Content,
}

pub enum Overlay {
    Commit(LineInput),
    BranchPick { sel: usize, scroll: usize },
    OpenRepo(LineInput),
    /// Find-file palette over the already-fetched recursive tree.
    FileSearch { input: LineInput, sel: usize },
    /// GitHub code-search palette (token required; default branch only).
    CodeSearch {
        input: LineInput,
        sel: usize,
        /// Last submitted query — Enter searches when the input differs,
        /// opens the selected hit when it matches.
        searched: String,
        results: Loadable<Vec<crate::github::CodeHit>>,
    },
    Help,
    Confirm { msg: String, action: ConfirmAction },
}

#[derive(Clone)]
pub enum ConfirmAction {
    LeaveRepo,
    SwitchBranch(String),
    OpenFile(String),
    /// An async `RepoOpened` landed while the current file had unsaved
    /// edits; opening the fetched repo needs the usual confirm.
    OpenRepo(Repo),
}

/// Mouse hit-regions, rebuilt on every draw.
#[derive(Clone, Copy, PartialEq)]
pub enum Click {
    Repo(usize), // index into the *filtered* repo list
    TreeRow(usize),
    Tab(Tab),
    BranchBtn,
    Run(usize),
    /// Direct editor position: row + visual cell x (converted to a char
    /// column via x_to_col).
    EditorPos { row: usize, cell_x: usize },
    OverlayItem(usize),
    EditBtn,
    CommitBtn,
    AgentClear,
    AgentResetKey,
    SortCycle,
    SortDir,
    ToggleForks,
    ToggleArchived,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Scroll {
    Repos,
    Tree,
    Content,
    Runs,
    Jobs,
    Overlay,
    Agent,
}

#[derive(Clone, Copy)]
pub struct Layout {
    pub repos_h: usize,
    /// Cards per row in the repo grid (keyboard navigation is 2D).
    pub repos_cols: usize,
    pub tree_h: usize,
    pub content_text: Rect,
    pub gutter: i32,
    pub runs_h: usize,
    pub jobs_h: usize,
    pub overlay_h: usize,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            repos_h: 0,
            repos_cols: 1,
            tree_h: 0,
            content_text: Rect::new(0, 0, 0, 0),
            gutter: 0,
            runs_h: 0,
            jobs_h: 0,
            overlay_h: 0,
        }
    }
}

pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

pub struct OpenFile {
    pub path: String,
    pub sha: String,
    pub editor: Editor,
    pub lang: Option<&'static LangSpec>,
    pub line_states: Vec<LineState>,
    pub binary: bool,
    pub size: u64,
    pub editing: bool,
    pub committing: bool,
    /// Exact text sent with an in-flight commit; on success `modified` is
    /// cleared only if the buffer still matches (edits typed while the
    /// commit was in flight must survive).
    pub pending_commit: Option<String>,
}
