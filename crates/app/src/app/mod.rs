//! Application state machine: routes, async message handling, key/mouse
//! dispatch. Pure logic — drawing lives in px/view, IO in github. Split by
//! topic: each submodule owns one slice of `App`'s behavior as `impl App`
//! blocks; this hub holds the struct itself and the cross-cutting helpers.

pub mod editor;

mod actions;
mod agent_compact;
mod agent_history;
mod agent_loop;
mod auth;
mod chat;
mod code_keys;
mod code_search;
mod commit;
mod file_msgs;
mod files;
mod input;
mod issue_actions;
mod issue_detail;
mod issue_msgs;
mod issues;
mod keys;
mod menu;
mod msg;
mod overlays;
mod repo;
mod repo_msgs;
mod repos;
mod search;
mod staging;
mod state;
mod tree;

pub use actions::run_icon;
pub use chat::{AgentChat, AgentItem};
pub use issue_detail::{Detail, MergeMethod};
pub use msg::Msg;
pub use repo::RepoView;
pub use search::search_tree;
pub use state::*;
pub use tree::rebuild_rows;

use crate::github::Repo;
use crate::ui::lineinput::LineInput;

pub struct App {
    pub token: Option<String>,
    pub login: Option<String>,
    pub route: Route,

    pub token_input: LineInput,
    pub auth_busy: bool,
    pub auth_error: Option<String>,

    pub repos: Loadable<Vec<Repo>>,
    /// True while further pages of the repo listing are still streaming in
    /// (the list renders and grows as each page lands).
    pub repos_loading_more: bool,
    /// Bumped on every (re)load; orphans any page chain still in flight.
    pub(super) repos_gen: u64,
    pub repo_source: RepoSource,
    pub repo_sel: usize,
    pub repo_scroll: usize,
    pub filter: LineInput,
    pub filter_active: bool,
    pub hide_forks: bool,
    pub hide_archived: bool,
    pub repo_sort: RepoSort,
    pub sort_asc: bool,

    pub rv: Option<RepoView>,
    pub overlay: Option<Overlay>,
    /// Floating right-click menu (tree actions); drawn above everything.
    pub context_menu: Option<ContextMenu>,
    pub toast: Option<(String, bool)>,
    /// Owner/name of the repo an async open (OpenRepo overlay) is fetching;
    /// a `RepoOpened` for anything else is stale and dropped.
    pub opening_repo: Option<String>,
    /// Bumped per code-search submit; a `CodeSearchDone` with a stale gen
    /// (overlay reopened, query reissued, or navigated away) is dropped.
    pub(super) code_search_gen: u64,

    pub anthropic_key: Option<String>,
    pub anthropic_url: Option<String>,
    /// Selected model id (persisted); chosen via the model picker, never
    /// shown in the UI. Defaults to `agent::MODEL`.
    pub agent_model: String,
    pub agent: AgentChat,

    /// Author/committer/date overrides for staged commits; remembered across
    /// commits in a session (see the commit overlay).
    pub commit_identity: CommitIdentity,

    pub layout: Layout,

    pub dirty: bool,
}

impl App {
    pub fn new(token: Option<String>) -> Self {
        let mut app = App {
            token: None,
            login: None,
            route: Route::Auth,
            token_input: LineInput::new(true),
            auth_busy: false,
            auth_error: None,
            repos: Loadable::Idle,
            repos_loading_more: false,
            repos_gen: 0,
            repo_source: RepoSource::Mine,
            repo_sel: 0,
            repo_scroll: 0,
            filter: LineInput::new(false),
            filter_active: false,
            hide_forks: false,
            hide_archived: false,
            repo_sort: RepoSort::Pushed,
            sort_asc: false,
            rv: None,
            overlay: None,
            context_menu: None,
            toast: None,
            opening_repo: None,
            code_search_gen: 0,
            anthropic_key: crate::agent::load_key(),
            anthropic_url: crate::agent::load_url(),
            agent_model: crate::agent::load_model()
                .unwrap_or_else(|| crate::agent::MODEL.to_string()),
            agent: AgentChat::new(),
            commit_identity: CommitIdentity::default(),
            layout: Layout::default(),
            dirty: true,
        };
        if let Some(t) = token {
            app.validate_token(t);
        }
        app
    }

    pub fn in_editor(&self) -> bool {
        self.rv
            .as_ref()
            .and_then(|rv| rv.file.as_ref().map(|f| f.editing && rv.focus == RepoFocus::Content))
            .unwrap_or(false)
    }

    /// Extend the editor selection during a mouse drag (anchor was set by
    /// the mouse-down click).
    pub fn editor_drag(&mut self, row: usize, cell_x: usize) {
        let Some(rv) = &mut self.rv else { return };
        let Some(f) = &mut rv.file else { return };
        let row = row.min(f.editor.line_count().saturating_sub(1));
        let col = f.editor.x_to_col(row, cell_x);
        f.editor.move_to((row, col), true);
        self.dirty = true;
    }

    pub fn editor_selection_text(&self) -> Option<String> {
        let f = self.rv.as_ref()?.file.as_ref()?;
        f.editor.selection_text()
    }
}

/// "3h ago"-style formatting via the JS Date API (available in both runtimes).
pub fn fmt_age(iso: &str) -> String {
    let t = js_sys::Date::parse(iso);
    if !t.is_finite() {
        return String::new();
    }
    let secs = ((js_sys::Date::now() - t) / 1000.0).max(0.0) as u64;
    match secs {
        0..=59 => "now".to_string(),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86399 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86400),
    }
}
