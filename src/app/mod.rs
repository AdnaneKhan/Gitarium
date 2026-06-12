//! Application state machine: routes, async message handling, key/mouse
//! dispatch. Pure logic — drawing lives in px/view.rs, IO in github.rs.

pub mod editor;

use std::collections::{HashMap, HashSet};

use crate::github::{self, Branch, Job, Repo, Run, TreeEntry};
use crate::highlight::{self, LangSpec, LineState};
use crate::ui::grid::Rect;
use crate::ui::input::{Event, Key, Mods};
use crate::ui::lineinput::LineInput;
use editor::Editor;

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

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
        results: Loadable<Vec<github::CodeHit>>,
    },
    Help,
    Confirm { msg: String, action: ConfirmAction },
}

#[derive(Clone)]
pub enum ConfirmAction {
    LeaveRepo,
    SwitchBranch(String),
    OpenFile(String),
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
}

pub struct RepoView {
    pub repo: Repo,
    pub branch: String,
    pub branches: Loadable<Vec<Branch>>,
    pub tree: Loadable<Vec<TreeEntry>>,
    pub rows: Vec<TreeRow>,
    pub expanded: HashSet<String>,
    pub tree_sel: usize,
    pub tree_scroll: usize,
    pub truncated: bool,
    pub file: Option<OpenFile>,
    pub file_loading: Option<String>,
    pub tab: Tab,
    pub focus: RepoFocus,
    pub runs: Loadable<Vec<Run>>,
    pub runs_sel: usize,
    pub runs_scroll: usize,
    pub jobs: Option<(u64, Loadable<Vec<Job>>)>,
    pub jobs_scroll: usize,
}

impl RepoView {
    fn new(repo: Repo) -> Self {
        let branch = repo.default_branch.clone();
        RepoView {
            repo,
            branch,
            branches: Loadable::Loading,
            tree: Loadable::Loading,
            rows: Vec::new(),
            expanded: HashSet::new(),
            tree_sel: 0,
            tree_scroll: 0,
            truncated: false,
            file: None,
            file_loading: None,
            tab: Tab::Code,
            focus: RepoFocus::Tree,
            runs: Loadable::Idle,
            runs_sel: 0,
            runs_scroll: 0,
            jobs: None,
            jobs_scroll: 0,
        }
    }

    fn branch_sha(&self) -> Option<String> {
        self.branches
            .ready()?
            .iter()
            .find(|b| b.name == self.branch)
            .map(|b| b.commit.sha.clone())
    }
}

/// One entry in the agent window's transcript.
pub enum AgentItem {
    User(String),
    Text(String),
    /// A github_api invocation; `done` is None while in flight.
    Tool { label: String, done: Option<bool> },
    Error(String),
}

pub struct AgentChat {
    pub key_input: LineInput,
    pub url_input: LineInput,
    /// Which field the key panel is editing (false = API key, true = URL).
    pub url_focused: bool,
    pub input: LineInput,
    pub transcript: Vec<AgentItem>,
    /// Verbatim Messages API history (assistant content blocks are echoed
    /// back unchanged so thinking/tool_use pairing stays valid).
    pub history: Vec<serde_json::Value>,
    pub busy: bool,
    /// Bumped to invalidate in-flight futures (cancel / clear).
    pub gen: u64,
    /// Bumped on every transcript change; the view uses it to re-stick the
    /// scroll position to the bottom.
    pub rev: u64,
    /// Transcript indices of Tool items awaiting results.
    pub pending: Vec<usize>,
}

impl AgentChat {
    fn new() -> Self {
        AgentChat {
            key_input: LineInput::new(true),
            url_input: LineInput::new(false),
            url_focused: false,
            input: LineInput::new(false),
            transcript: Vec::new(),
            history: Vec::new(),
            busy: false,
            gen: 0,
            rev: 0,
            pending: Vec::new(),
        }
    }

    fn push(&mut self, item: AgentItem) {
        self.transcript.push(item);
        self.rev += 1;
    }
}

pub enum Msg {
    TokenChecked {
        token: Option<String>,
        result: Result<github::User, String>,
    },
    Repos {
        source: RepoSource,
        result: Result<Vec<Repo>, String>,
    },
    RepoOpened(Result<Repo, String>),
    Branches {
        repo: String,
        result: Result<Vec<Branch>, String>,
    },
    Tree {
        repo: String,
        result: Result<github::TreeResp, String>,
    },
    FileLoaded {
        repo: String,
        path: String,
        result: Result<(String, Vec<u8>), String>,
    },
    Committed {
        path: String,
        // (new content sha, new commit sha)
        result: Result<(String, String), String>,
    },
    Runs {
        repo: String,
        result: Result<Vec<Run>, String>,
    },
    Jobs {
        repo: String,
        run_id: u64,
        result: Result<Vec<Job>, String>,
    },
    CodeSearchDone {
        repo: String,
        result: Result<Vec<github::CodeHit>, String>,
    },
    /// One Messages API response in the agent loop.
    AgentResponse {
        gen: u64,
        result: Result<serde_json::Value, String>,
    },
    /// All tool calls of one turn finished: (tool_result block, ok) each.
    AgentToolsDone {
        gen: u64,
        results: Vec<(serde_json::Value, bool)>,
    },
}

pub struct App {
    pub token: Option<String>,
    pub login: Option<String>,
    pub route: Route,

    pub token_input: LineInput,
    pub auth_busy: bool,
    pub auth_error: Option<String>,

    pub repos: Loadable<Vec<Repo>>,
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
    pub toast: Option<(String, bool)>,

    pub anthropic_key: Option<String>,
    pub anthropic_url: Option<String>,
    pub agent: AgentChat,

    pub layout: Layout,

    pub dirty: bool,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

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
            toast: None,
            anthropic_key: crate::agent::load_key(),
            anthropic_url: crate::agent::load_url(),
            agent: AgentChat::new(),
            layout: Layout::default(),
            dirty: true,
        };
        if let Some(t) = token {
            app.validate_token(t);
        }
        app
    }

    fn validate_token(&mut self, t: String) {
        self.auth_busy = true;
        self.auth_error = None;
        let token = Some(t.clone());
        crate::spawn_msg(async move {
            let result = github::current_user(&token).await;
            Msg::TokenChecked { token, result }
        });
    }

    fn load_repos(&mut self) {
        let token = self.token.clone();
        match self.repo_source.clone() {
            RepoSource::Mine => {
                if token.is_none() {
                    self.repos = Loadable::Idle;
                    return;
                }
                self.repos = Loadable::Loading;
                crate::spawn_msg(async move {
                    Msg::Repos {
                        source: RepoSource::Mine,
                        result: github::list_repos(&token).await,
                    }
                });
            }
            RepoSource::Org(name) => {
                self.repos = Loadable::Loading;
                crate::spawn_msg(async move {
                    let result = github::list_owner_repos(&token, &name).await;
                    Msg::Repos { source: RepoSource::Org(name), result }
                });
            }
        }
    }

    /// Browse another org's (or user's) repositories on the Repos screen.
    fn open_org(&mut self, name: String) {
        self.repo_source = RepoSource::Org(name);
        self.repo_sel = 0;
        self.repo_scroll = 0;
        self.filter.clear();
        self.filter_active = false;
        self.route = Route::Repos;
        self.load_repos();
    }

    /// Indices into `repos` after text filter, fork/archived toggles, and
    /// the active sort.
    pub fn filtered_repos(&self) -> Vec<usize> {
        let needle = self.filter.text.to_lowercase();
        let Some(repos) = self.repos.ready() else {
            return Vec::new();
        };
        let mut idx: Vec<usize> = repos
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                let text_match = needle.is_empty()
                    || r.full_name.to_lowercase().contains(&needle)
                    || r.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&needle))
                        .unwrap_or(false)
                    || r.language
                        .as_ref()
                        .map(|l| l.to_lowercase().contains(&needle))
                        .unwrap_or(false);
                text_match
                    && !(self.hide_forks && r.fork)
                    && !(self.hide_archived && r.archived)
            })
            .map(|(i, _)| i)
            .collect();
        idx.sort_by(|&a, &b| {
            let (ra, rb) = (&repos[a], &repos[b]);
            let ord = match self.repo_sort {
                RepoSort::Name => ra.full_name.to_lowercase().cmp(&rb.full_name.to_lowercase()),
                RepoSort::Stars => ra.stargazers_count.cmp(&rb.stargazers_count),
                RepoSort::Forks => ra.forks_count.cmp(&rb.forks_count),
                // ISO-8601 strings compare chronologically; None sorts last.
                RepoSort::Pushed => ra.pushed_at.cmp(&rb.pushed_at),
            };
            if self.sort_asc { ord } else { ord.reverse() }
        });
        idx
    }

    fn cycle_sort(&mut self) {
        // Each key starts in its natural direction.
        let (next, asc) = match self.repo_sort {
            RepoSort::Pushed => (RepoSort::Name, true),
            RepoSort::Name => (RepoSort::Stars, false),
            RepoSort::Stars => (RepoSort::Forks, false),
            RepoSort::Forks => (RepoSort::Pushed, false),
        };
        self.repo_sort = next;
        self.sort_asc = asc;
        self.repo_sel = 0;
    }

    fn open_repo(&mut self, repo: Repo) {
        let full = repo.full_name.clone();
        self.rv = Some(RepoView::new(repo));
        self.route = Route::Repo;
        let token = self.token.clone();
        let full2 = full.clone();
        crate::spawn_msg(async move {
            Msg::Branches {
                repo: full2.clone(),
                result: github::list_branches(&token, &full2).await,
            }
        });
        let _ = full;
    }

    fn load_tree(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        let Some(sha) = rv.branch_sha() else {
            rv.tree = Loadable::Failed("branch not found".into());
            return;
        };
        rv.tree = Loadable::Loading;
        rv.rows.clear();
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::Tree {
                repo: full.clone(),
                result: github::get_tree(&token, &full, &sha).await,
            }
        });
    }

    fn load_runs(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        rv.runs = Loadable::Loading;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::Runs {
                repo: full.clone(),
                result: github::list_runs(&token, &full).await,
            }
        });
    }

    fn load_jobs(&mut self, run_id: u64) {
        let Some(rv) = &mut self.rv else { return };
        rv.jobs = Some((run_id, Loadable::Loading));
        rv.jobs_scroll = 0;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::Jobs {
                repo: full.clone(),
                run_id,
                result: github::list_jobs(&token, &full, run_id).await,
            }
        });
    }

    fn open_file(&mut self, path: String) {
        let Some(rv) = &mut self.rv else { return };
        rv.file = None;
        rv.file_loading = Some(path.clone());
        rv.focus = RepoFocus::Content;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let branch = rv.branch.clone();
        crate::spawn_msg(async move {
            let result: Result<(String, Vec<u8>), String> = async {
                let cf = github::get_file(&token, &full, &path, &branch).await?;
                let b64 = match cf.content.as_deref() {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ if cf.size == 0 => String::new(),
                    _ => github::get_blob(&token, &full, &cf.sha).await?.content,
                };
                let bytes = if b64.is_empty() {
                    Vec::new()
                } else {
                    github::b64_decode(&b64)?
                };
                Ok((cf.sha, bytes))
            }
            .await;
            Msg::FileLoaded { repo: full.clone(), path, result }
        });
    }

    fn commit_file(&mut self, message: String) {
        let Some(rv) = &mut self.rv else { return };
        let Some(file) = &mut rv.file else { return };
        file.committing = true;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let path = file.path.clone();
        let branch = rv.branch.clone();
        let sha = file.sha.clone();
        let content = github::b64_encode(file.editor.to_text().as_bytes());
        crate::spawn_msg(async move {
            let result: Result<(String, String), String> = async {
                let resp =
                    github::put_file(&token, &full, &path, &message, &content, Some(&sha), &branch)
                        .await?;
                let csha = resp.content.map(|c| c.sha).ok_or("no content sha in response")?;
                let ksha = resp.commit.map(|c| c.sha).unwrap_or_default();
                Ok((csha, ksha))
            }
            .await;
            Msg::Committed { path, result }
        });
        self.toast = Some(("committing…".into(), false));
    }

    fn switch_branch(&mut self, name: String) {
        let Some(rv) = &mut self.rv else { return };
        rv.branch = name;
        rv.file = None;
        rv.file_loading = None;
        rv.expanded.clear();
        rv.tree_sel = 0;
        rv.tree_scroll = 0;
        self.load_tree();
    }

    // -----------------------------------------------------------------------
    // Agent loop (Claude + github_api tool)
    // -----------------------------------------------------------------------

    fn open_agent(&mut self) {
        self.route = Route::Agent;
    }

    fn leave_agent(&mut self) {
        self.route = if self.rv.is_some() { Route::Repo } else { Route::Repos };
    }

    /// Fire one Messages API request for the current history.
    fn agent_turn(&mut self) {
        let Some(key) = self.anthropic_key.clone() else { return };
        let repo_ctx = self
            .rv
            .as_ref()
            .map(|rv| (rv.repo.full_name.clone(), rv.branch.clone()));
        let file = self
            .rv
            .as_ref()
            .and_then(|rv| rv.file.as_ref())
            .map(|f| f.path.clone());
        let system = crate::agent::system_prompt(
            self.login.as_deref(),
            repo_ctx.as_ref().map(|(r, b)| (r.as_str(), b.as_str())),
            file.as_deref(),
        );
        let body = crate::agent::build_request(&system, &self.agent.history);
        let base = self.anthropic_url.clone();
        let gen = self.agent.gen;
        crate::spawn_msg(async move {
            Msg::AgentResponse {
                gen,
                result: crate::agent::complete(&key, base.as_deref(), body).await,
            }
        });
    }

    fn agent_send(&mut self) {
        let text = self.agent.input.text.trim().to_string();
        if text.is_empty() || self.agent.busy {
            return;
        }
        self.agent.input.clear();
        self.agent.push(AgentItem::User(text.clone()));
        self.agent
            .history
            .push(serde_json::json!({"role": "user", "content": text}));
        self.agent.busy = true;
        self.agent.gen += 1;
        self.agent_turn();
    }

    fn agent_cancel(&mut self) {
        self.agent.gen += 1; // orphan any in-flight future
        self.agent.busy = false;
        for &i in &self.agent.pending {
            if let Some(AgentItem::Tool { done, .. }) = self.agent.transcript.get_mut(i) {
                *done = Some(false);
            }
        }
        self.agent.pending.clear();
        // Keep the history valid for the next send: a trailing assistant
        // message with unanswered tool_use blocks would be rejected.
        let dangling = self
            .agent
            .history
            .last()
            .map(|m| {
                m["role"] == "assistant"
                    && m["content"]
                        .as_array()
                        .map(|c| c.iter().any(|b| b["type"] == "tool_use"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if dangling {
            self.agent.history.pop();
        }
        self.agent.push(AgentItem::Error("cancelled".into()));
    }

    fn agent_clear(&mut self) {
        if self.agent.busy {
            self.agent_cancel();
        }
        self.agent.transcript.clear();
        self.agent.history.clear();
        self.agent.pending.clear();
        crate::agent::clear_store();
        self.agent.rev += 1;
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

    fn on_agent_response(&mut self, result: Result<serde_json::Value, String>) {
        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                self.agent.busy = false;
                self.agent.push(AgentItem::Error(e));
                return;
            }
        };
        let content = resp["content"].clone();
        let stop = resp["stop_reason"].as_str().unwrap_or("").to_string();
        self.agent
            .history
            .push(serde_json::json!({"role": "assistant", "content": content}));
        if let Some(blocks) = content.as_array() {
            for b in blocks {
                if b["type"] == "text" {
                    if let Some(t) = b["text"].as_str() {
                        if !t.trim().is_empty() {
                            self.agent.push(AgentItem::Text(t.to_string()));
                        }
                    }
                }
            }
        }
        match stop.as_str() {
            "tool_use" => {
                let calls = crate::agent::parse_tool_calls(&content);
                if calls.is_empty() {
                    self.agent.busy = false;
                    return;
                }
                self.agent.pending.clear();
                for c in &calls {
                    self.agent.push(AgentItem::Tool { label: c.label(), done: None });
                    self.agent.pending.push(self.agent.transcript.len() - 1);
                }
                let token = self.token.clone();
                let gen = self.agent.gen;
                crate::spawn_msg(async move {
                    let mut results = Vec::with_capacity(calls.len());
                    for c in &calls {
                        let (text, ok) = crate::agent::exec(&token, c).await;
                        results.push((crate::agent::tool_result_block(c.id(), &text, ok), ok));
                    }
                    Msg::AgentToolsDone { gen, results }
                });
            }
            // Server-side pause (defensive — no server tools configured):
            // re-send and the API resumes where it left off.
            "pause_turn" => self.agent_turn(),
            "refusal" => {
                self.agent.busy = false;
                let cat = resp["stop_details"]["category"].as_str().unwrap_or("");
                let msg = if cat.is_empty() {
                    "request declined by the model".to_string()
                } else {
                    format!("request declined by the model ({})", cat)
                };
                self.agent.push(AgentItem::Error(msg));
            }
            "max_tokens" => {
                self.agent.busy = false;
                self.agent
                    .push(AgentItem::Error("response hit the token limit — say 'continue'".into()));
            }
            _ => self.agent.busy = false, // end_turn
        }
    }

    // -----------------------------------------------------------------------
    // Messages
    // -----------------------------------------------------------------------

    pub fn on_msg(&mut self, msg: Msg) {
        self.dirty = true;
        match msg {
            Msg::TokenChecked { token, result } => {
                self.auth_busy = false;
                match result {
                    Ok(user) => {
                        self.token = token;
                        self.login = Some(user.login);
                        self.route = Route::Repos;
                        self.load_repos();
                    }
                    Err(e) => {
                        self.auth_error = Some(e);
                        self.route = Route::Auth;
                    }
                }
            }
            Msg::Repos { source, result } => {
                if source != self.repo_source {
                    return; // stale response from a previous source
                }
                self.repos = match result {
                    Ok(r) => Loadable::Ready(r),
                    Err(e) => Loadable::Failed(e),
                };
                self.repo_sel = 0;
                self.repo_scroll = 0;
            }
            Msg::RepoOpened(result) => match result {
                Ok(repo) => self.open_repo(repo),
                Err(e) => self.toast = Some((e, true)),
            },
            Msg::Branches { repo, result } => {
                let current = self.rv.as_ref().map(|rv| rv.repo.full_name.clone());
                if current.as_deref() != Some(repo.as_str()) {
                    return;
                }
                match result {
                    Ok(branches) => {
                        if let Some(rv) = &mut self.rv {
                            if !branches.iter().any(|b| b.name == rv.branch) {
                                if let Some(first) = branches.first() {
                                    rv.branch = first.name.clone();
                                }
                            }
                            rv.branches = Loadable::Ready(branches);
                        }
                        self.load_tree();
                    }
                    Err(e) => {
                        if let Some(rv) = &mut self.rv {
                            rv.branches = Loadable::Failed(e.clone());
                            rv.tree = Loadable::Failed(e);
                        }
                    }
                }
            }
            Msg::Tree { repo, result } => {
                let Some(rv) = &mut self.rv else { return };
                if rv.repo.full_name != repo {
                    return;
                }
                match result {
                    Ok(mut t) => {
                        rv.truncated = t.truncated;
                        t.tree.retain(|e| e.kind == "blob" || e.kind == "tree");
                        rv.tree = Loadable::Ready(t.tree);
                        rv.tree_sel = 0;
                        rv.tree_scroll = 0;
                        rebuild_rows(rv);
                    }
                    Err(e) => rv.tree = Loadable::Failed(e),
                }
            }
            Msg::FileLoaded { repo, path, result } => {
                let Some(rv) = &mut self.rv else { return };
                if rv.repo.full_name != repo || rv.file_loading.as_deref() != Some(path.as_str()) {
                    return;
                }
                rv.file_loading = None;
                match result {
                    Ok((sha, bytes)) => {
                        let size = bytes.len() as u64;
                        let binary = bytes.contains(&0);
                        let text = if binary {
                            String::new()
                        } else {
                            match String::from_utf8(bytes) {
                                Ok(t) => t,
                                Err(_) => {
                                    rv.file = Some(make_binary_file(path, sha, size));
                                    return;
                                }
                            }
                        };
                        if binary {
                            rv.file = Some(make_binary_file(path, sha, size));
                            return;
                        }
                        let lang = highlight::lang_for_path(&path);
                        let mut file = OpenFile {
                            path,
                            sha,
                            editor: Editor::from_text(&text),
                            lang,
                            line_states: Vec::new(),
                            binary: false,
                            size,
                            editing: false,
                            committing: false,
                        };
                        rehighlight(&mut file);
                        rv.file = Some(file);
                    }
                    Err(e) => self.toast = Some((e, true)),
                }
            }
            Msg::Committed { path, result } => {
                let Some(rv) = &mut self.rv else { return };
                let branch = rv.branch.clone();
                let Some(file) = &mut rv.file else { return };
                if file.path != path {
                    return;
                }
                file.committing = false;
                match result {
                    Ok((content_sha, commit_sha)) => {
                        file.sha = content_sha;
                        file.editor.modified = false;
                        // Keep the branch head fresh so tree reloads work.
                        if let Loadable::Ready(branches) = &mut rv.branches {
                            if let Some(b) = branches.iter_mut().find(|b| b.name == branch) {
                                if !commit_sha.is_empty() {
                                    b.commit.sha = commit_sha.clone();
                                }
                            }
                        }
                        let short: String = commit_sha.chars().take(7).collect();
                        self.toast = Some((format!("committed {} ✓", short), false));
                    }
                    Err(e) => self.toast = Some((format!("commit failed: {}", e), true)),
                }
            }
            Msg::Runs { repo, result } => {
                let Some(rv) = &mut self.rv else { return };
                if rv.repo.full_name != repo {
                    return;
                }
                rv.runs = match result {
                    Ok(r) => Loadable::Ready(r),
                    Err(e) => Loadable::Failed(e),
                };
                rv.runs_sel = 0;
                rv.runs_scroll = 0;
            }
            Msg::Jobs { repo, run_id, result } => {
                let Some(rv) = &mut self.rv else { return };
                if rv.repo.full_name != repo {
                    return;
                }
                if let Some((id, slot)) = &mut rv.jobs {
                    if *id == run_id {
                        *slot = match result {
                            Ok(j) => Loadable::Ready(j),
                            Err(e) => Loadable::Failed(e),
                        };
                    }
                }
            }
            Msg::AgentResponse { gen, result } => {
                if gen != self.agent.gen || !self.agent.busy {
                    return; // cancelled or superseded
                }
                self.on_agent_response(result);
            }
            Msg::AgentToolsDone { gen, results } => {
                if gen != self.agent.gen || !self.agent.busy {
                    return;
                }
                for (i, &idx) in self.agent.pending.iter().enumerate() {
                    if let Some(AgentItem::Tool { done, .. }) = self.agent.transcript.get_mut(idx) {
                        *done = results.get(i).map(|(_, ok)| *ok);
                    }
                }
                self.agent.pending.clear();
                self.agent.rev += 1;
                let blocks: Vec<serde_json::Value> =
                    results.into_iter().map(|(b, _)| b).collect();
                self.agent
                    .history
                    .push(serde_json::json!({"role": "user", "content": blocks}));
                self.agent_turn();
            }
            Msg::CodeSearchDone { repo, result } => {
                let current = self.rv.as_ref().map(|rv| rv.repo.full_name.clone());
                if current.as_deref() != Some(repo.as_str()) {
                    return;
                }
                if let Some(Overlay::CodeSearch { results, sel, .. }) = &mut self.overlay {
                    *results = match result {
                        Ok(h) => Loadable::Ready(h),
                        Err(e) => Loadable::Failed(e),
                    };
                    *sel = 0;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    pub fn on_event(&mut self, ev: Event) {
        self.dirty = true;
        match ev {
            Event::Key(key, mods) => {
                self.toast = None;
                if self.overlay.is_some() {
                    self.overlay_key(key, mods);
                    return;
                }
                match self.route {
                    Route::Auth => self.auth_key(key, mods),
                    Route::Repos => self.repos_key(key, mods),
                    Route::Repo => self.repo_key(key, mods),
                    Route::Agent => self.agent_key(key, mods),
                }
            }
            Event::Paste(text) => self.on_paste(text),
        }
    }

    pub fn in_editor(&self) -> bool {
        self.rv
            .as_ref()
            .and_then(|rv| rv.file.as_ref().map(|f| f.editing && rv.focus == RepoFocus::Content))
            .unwrap_or(false)
    }

    fn auth_key(&mut self, key: Key, mods: Mods) {
        if self.auth_busy {
            return;
        }
        match key {
            Key::Enter => {
                let t = self.token_input.text.trim().to_string();
                if t.is_empty() {
                    // Anonymous mode: public repos, read-only commits will fail.
                    self.token = None;
                    self.login = None;
                    self.route = Route::Repos;
                    self.repos = Loadable::Idle;
                    self.overlay = Some(Overlay::OpenRepo(LineInput::new(false)));
                } else {
                    self.validate_token(t);
                }
            }
            k => {
                self.token_input.handle_key(&k, mods);
            }
        }
    }

    fn repos_key(&mut self, key: Key, mods: Mods) {
        if self.filter_active {
            match key {
                Key::Esc => {
                    self.filter.clear();
                    self.filter_active = false;
                }
                Key::Enter => self.filter_active = false,
                Key::Up | Key::Down => {
                    self.filter_active = false;
                    self.repos_key(key, mods);
                }
                k => {
                    if self.filter.handle_key(&k, mods) {
                        self.repo_sel = 0;
                        self.repo_scroll = 0;
                    }
                }
            }
            return;
        }
        let count = self.filtered_repos().len();
        match key {
            Key::Char('?') => self.overlay = Some(Overlay::Help),
            Key::Char('/') => self.filter_active = true,
            Key::Char('i') => self.open_agent(),
            Key::Char('o') => self.overlay = Some(Overlay::OpenRepo(LineInput::new(false))),
            Key::Char('r') => self.load_repos(),
            Key::Char('f') => {
                self.hide_forks = !self.hide_forks;
                self.repo_sel = 0;
            }
            Key::Char('x') => {
                self.hide_archived = !self.hide_archived;
                self.repo_sel = 0;
            }
            Key::Char('s') => self.cycle_sort(),
            Key::Char('S') => {
                self.sort_asc = !self.sort_asc;
                self.repo_sel = 0;
            }
            Key::Esc => {
                // Leave an org listing, back to the user's own repos.
                if self.repo_source != RepoSource::Mine {
                    self.repo_source = RepoSource::Mine;
                    self.repo_sel = 0;
                    self.repo_scroll = 0;
                    self.filter.clear();
                    self.load_repos();
                }
            }
            // 2D navigation over the card grid.
            Key::Left => self.repo_sel = self.repo_sel.saturating_sub(1),
            Key::Right => {
                if count > 0 {
                    self.repo_sel = (self.repo_sel + 1).min(count - 1);
                }
            }
            Key::Up => self.repo_sel = self.repo_sel.saturating_sub(self.layout.repos_cols.max(1)),
            Key::Down => {
                if count > 0 {
                    self.repo_sel =
                        (self.repo_sel + self.layout.repos_cols.max(1)).min(count - 1);
                }
            }
            Key::PageUp => {
                let page = self.layout.repos_cols.max(1) * self.layout.repos_h.max(1);
                self.repo_sel = self.repo_sel.saturating_sub(page);
            }
            Key::PageDown => {
                if count > 0 {
                    let page = self.layout.repos_cols.max(1) * self.layout.repos_h.max(1);
                    self.repo_sel = (self.repo_sel + page).min(count - 1);
                }
            }
            Key::Home => self.repo_sel = 0,
            Key::End => self.repo_sel = count.saturating_sub(1),
            Key::Enter => {
                let filtered = self.filtered_repos();
                if let Some(&idx) = filtered.get(self.repo_sel) {
                    if let Some(repos) = self.repos.ready() {
                        let repo = repos[idx].clone();
                        self.open_repo(repo);
                    }
                }
            }
            _ => {}
        }
    }

    fn agent_key(&mut self, key: Key, mods: Mods) {
        // No API key yet: the window shows the key/endpoint prompt.
        if self.anthropic_key.is_none() {
            match key {
                Key::Esc => self.leave_agent(),
                Key::Tab | Key::BackTab | Key::Up | Key::Down => {
                    self.agent.url_focused = !self.agent.url_focused;
                }
                Key::Enter => {
                    let k = self.agent.key_input.text.trim().to_string();
                    if k.is_empty() {
                        return;
                    }
                    let url = crate::agent::normalize_base(&self.agent.url_input.text);
                    match &url {
                        Some(u) => crate::agent::save_url(u),
                        None => crate::agent::clear_url(),
                    }
                    self.anthropic_url = url;
                    crate::agent::save_key(&k);
                    self.anthropic_key = Some(k);
                    self.agent.key_input.clear();
                    self.agent.url_input.clear();
                    self.agent.url_focused = false;
                }
                k => {
                    if self.agent.url_focused {
                        self.agent.url_input.handle_key(&k, mods);
                    } else {
                        self.agent.key_input.handle_key(&k, mods);
                    }
                }
            }
            return;
        }
        match key {
            Key::Esc => {
                if self.agent.busy {
                    self.agent_cancel();
                } else {
                    self.leave_agent();
                }
            }
            Key::Enter => self.agent_send(),
            k => {
                self.agent.input.handle_key(&k, mods);
            }
        }
    }

    fn repo_key(&mut self, key: Key, mods: Mods) {
        let Some(rv) = &mut self.rv else { return };
        match rv.tab {
            Tab::Code => self.code_key(key, mods),
            Tab::Actions => self.actions_key(key),
        }
    }

    fn code_key(&mut self, key: Key, mods: Mods) {
        let in_editor = self.in_editor();
        let rv = self.rv.as_mut().unwrap();

        // Editor consumes nearly everything while editing.
        if in_editor {
            if key == Key::Char('s') && mods.ctrl {
                self.begin_commit();
                return;
            }
            if key == Key::Esc {
                if let Some(f) = &mut rv.file {
                    f.editing = false;
                    f.editor.read_only = true;
                    if f.editor.modified {
                        self.toast =
                            Some(("buffer modified — press c to commit".into(), false));
                    }
                }
                return;
            }
            let lay = self.layout;
            if let Some(f) = &mut rv.file {
                let changed = f.editor.handle_key(
                    &key,
                    mods,
                    lay.content_text.h.max(1) as usize,
                );
                if changed {
                    f.editor.ensure_visible(
                        lay.content_text.h.max(1) as usize,
                        lay.content_text.w.max(1) as usize,
                    );
                    rehighlight(f);
                }
            }
            return;
        }

        match key {
            Key::Char('?') => self.overlay = Some(Overlay::Help),
            Key::Char('/') => {
                self.overlay = Some(Overlay::FileSearch { input: LineInput::new(false), sel: 0 });
            }
            Key::Char('g') => {
                if self.token.is_none() {
                    self.toast = Some(("code search requires an access token".into(), true));
                } else {
                    self.overlay = Some(Overlay::CodeSearch {
                        input: LineInput::new(false),
                        sel: 0,
                        searched: String::new(),
                        results: Loadable::Idle,
                    });
                }
            }
            Key::Char('b') => {
                if rv.branches.ready().is_some() {
                    let sel = rv
                        .branches
                        .ready()
                        .and_then(|bs| bs.iter().position(|b| b.name == rv.branch))
                        .unwrap_or(0);
                    // Open with the current branch near the top of the list.
                    self.overlay = Some(Overlay::BranchPick { sel, scroll: sel.saturating_sub(3) });
                }
            }
            Key::Char('a') => {
                rv.tab = Tab::Actions;
                if matches!(rv.runs, Loadable::Idle) {
                    self.load_runs();
                }
            }
            Key::Char('e') => self.begin_edit(),
            Key::Char('c') => self.begin_commit(),
            Key::Char('i') => self.open_agent(),
            Key::Tab => {
                rv.focus = match rv.focus {
                    RepoFocus::Tree if rv.file.is_some() => RepoFocus::Content,
                    _ => RepoFocus::Tree,
                };
            }
            Key::Esc => {
                if rv.focus == RepoFocus::Content {
                    rv.focus = RepoFocus::Tree;
                    return;
                }
                let modified = rv
                    .file
                    .as_ref()
                    .map(|f| f.editor.modified)
                    .unwrap_or(false);
                if modified {
                    self.overlay = Some(Overlay::Confirm {
                        msg: "discard unsaved edits and leave repo?".into(),
                        action: ConfirmAction::LeaveRepo,
                    });
                } else {
                    self.route = Route::Repos;
                    self.rv = None;
                }
            }
            _ => match rv.focus {
                RepoFocus::Tree => self.tree_key(key),
                RepoFocus::Content => self.viewer_key(key),
            },
        }
    }

    fn tree_key(&mut self, key: Key) {
        let rv = self.rv.as_mut().unwrap();
        let count = rv.rows.len();
        match key {
            Key::Up => rv.tree_sel = rv.tree_sel.saturating_sub(1),
            Key::Down => {
                if count > 0 {
                    rv.tree_sel = (rv.tree_sel + 1).min(count - 1);
                }
            }
            Key::PageUp => rv.tree_sel = rv.tree_sel.saturating_sub(self.layout.tree_h.max(1)),
            Key::PageDown => {
                if count > 0 {
                    rv.tree_sel = (rv.tree_sel + self.layout.tree_h.max(1)).min(count - 1);
                }
            }
            Key::Home => rv.tree_sel = 0,
            Key::End => rv.tree_sel = count.saturating_sub(1),
            Key::Enter | Key::Right => {
                self.activate_tree_row(key == Key::Right);
            }
            Key::Left => {
                let Some(row) = rv.rows.get(rv.tree_sel) else { return };
                if row.is_dir && rv.expanded.contains(&row.path) {
                    let p = row.path.clone();
                    rv.expanded.remove(&p);
                    rebuild_rows(rv);
                } else if let Some(parent) = row.path.rsplit_once('/').map(|(p, _)| p.to_string()) {
                    if let Some(idx) = rv.rows.iter().position(|r| r.path == parent) {
                        rv.tree_sel = idx;
                    }
                }
            }
            _ => {}
        }
    }

    fn activate_tree_row(&mut self, expand_only: bool) {
        let rv = self.rv.as_mut().unwrap();
        let Some(row) = rv.rows.get(rv.tree_sel) else { return };
        if row.is_dir {
            let p = row.path.clone();
            if rv.expanded.contains(&p) {
                if !expand_only {
                    rv.expanded.remove(&p);
                }
            } else {
                rv.expanded.insert(p);
            }
            rebuild_rows(rv);
        } else if !expand_only {
            let path = row.path.clone();
            let modified = rv.file.as_ref().map(|f| f.editor.modified).unwrap_or(false);
            if modified {
                self.overlay = Some(Overlay::Confirm {
                    msg: "discard unsaved edits and open another file?".into(),
                    action: ConfirmAction::OpenFile(path),
                });
            } else {
                self.open_file(path);
            }
        }
    }

    fn viewer_key(&mut self, key: Key) {
        let lay = self.layout;
        let rv = self.rv.as_mut().unwrap();
        let Some(f) = &mut rv.file else { return };
        let h = lay.content_text.h.max(1) as usize;
        match key {
            Key::Up => f.editor.scroll_by(-1, h),
            Key::Down => f.editor.scroll_by(1, h),
            Key::PageUp => f.editor.scroll_by(-(h as i32), h),
            Key::PageDown => f.editor.scroll_by(h as i32, h),
            Key::Home => f.editor.scroll = 0,
            Key::End => f.editor.scroll = f.editor.line_count().saturating_sub(h),
            _ => {}
        }
    }

    fn actions_key(&mut self, key: Key) {
        let rv = self.rv.as_mut().unwrap();
        let count = rv.runs.ready().map(|r| r.len()).unwrap_or(0);
        match key {
            Key::Char('?') => self.overlay = Some(Overlay::Help),
            Key::Char('a') | Key::Esc => rv.tab = Tab::Code,
            Key::Char('i') => self.open_agent(),
            Key::Char('r') => self.load_runs(),
            Key::Up => rv.runs_sel = rv.runs_sel.saturating_sub(1),
            Key::Down => {
                if count > 0 {
                    rv.runs_sel = (rv.runs_sel + 1).min(count - 1);
                }
            }
            Key::PageUp => rv.runs_sel = rv.runs_sel.saturating_sub(self.layout.runs_h.max(1)),
            Key::PageDown => {
                if count > 0 {
                    rv.runs_sel = (rv.runs_sel + self.layout.runs_h.max(1)).min(count - 1);
                }
            }
            Key::Enter => {
                if let Some(runs) = rv.runs.ready() {
                    if let Some(run) = runs.get(rv.runs_sel) {
                        let id = run.id;
                        self.load_jobs(id);
                    }
                }
            }
            _ => {}
        }
    }

    fn begin_edit(&mut self) {
        let anonymous = self.token.is_none();
        let rv = self.rv.as_mut().unwrap();
        if let Some(f) = &mut rv.file {
            if f.binary {
                self.toast = Some(("cannot edit a binary file".into(), true));
                return;
            }
            f.editing = true;
            f.editor.read_only = false;
            rv.focus = RepoFocus::Content;
            if anonymous {
                self.toast = Some((
                    "anonymous mode: editing locally, commits will be rejected".into(),
                    false,
                ));
            }
        }
    }

    fn begin_commit(&mut self) {
        let rv = self.rv.as_mut().unwrap();
        let Some(f) = &rv.file else { return };
        if f.committing {
            return;
        }
        if !f.editor.modified {
            self.toast = Some(("no changes to commit".into(), false));
            return;
        }
        self.overlay = Some(Overlay::Commit(LineInput::new(false)));
    }

    // -----------------------------------------------------------------------
    // Overlays
    // -----------------------------------------------------------------------

    fn overlay_key(&mut self, key: Key, mods: Mods) {
        let Some(overlay) = &mut self.overlay else { return };
        match overlay {
            Overlay::Help => {
                self.overlay = None;
            }
            Overlay::Commit(input) => match key {
                Key::Esc => self.overlay = None,
                Key::Enter => {
                    let msg = input.text.trim().to_string();
                    if msg.is_empty() {
                        return;
                    }
                    self.overlay = None;
                    self.commit_file(msg);
                }
                k => {
                    input.handle_key(&k, mods);
                }
            },
            Overlay::FileSearch { input, sel } => match key {
                Key::Esc => self.overlay = None,
                Key::Up => *sel = sel.saturating_sub(1),
                Key::Down => {
                    let count = self
                        .rv
                        .as_ref()
                        .and_then(|rv| rv.tree.ready())
                        .map(|t| search_tree(t, &input.text).len())
                        .unwrap_or(0);
                    if count > 0 {
                        *sel = (*sel + 1).min(count - 1);
                    }
                }
                Key::Enter => {
                    let path = self.rv.as_ref().and_then(|rv| rv.tree.ready()).and_then(|t| {
                        search_tree(t, &input.text).get(*sel).map(|&i| t[i].path.clone())
                    });
                    self.overlay = None;
                    if let Some(path) = path {
                        let modified = self
                            .rv
                            .as_ref()
                            .and_then(|rv| rv.file.as_ref())
                            .map(|f| f.editor.modified)
                            .unwrap_or(false);
                        if modified {
                            self.overlay = Some(Overlay::Confirm {
                                msg: format!("discard unsaved edits and open {}?", path),
                                action: ConfirmAction::OpenFile(path),
                            });
                        } else {
                            self.open_file(path);
                        }
                    }
                }
                k => {
                    if input.handle_key(&k, mods) {
                        *sel = 0;
                    }
                }
            },
            Overlay::CodeSearch { input, sel, searched, results } => match key {
                Key::Esc => self.overlay = None,
                Key::Up => *sel = sel.saturating_sub(1),
                Key::Down => {
                    let count = results.ready().map(|h| h.len()).unwrap_or(0);
                    if count > 0 {
                        *sel = (*sel + 1).min(count - 1);
                    }
                }
                Key::Enter => {
                    let q = input.text.trim().to_string();
                    if q.is_empty() {
                        return;
                    }
                    if q != *searched {
                        // Submit (explicitly — code search is 10 req/min).
                        *searched = q.clone();
                        *results = Loadable::Loading;
                        *sel = 0;
                        let token = self.token.clone();
                        let full = self
                            .rv
                            .as_ref()
                            .map(|rv| rv.repo.full_name.clone())
                            .unwrap_or_default();
                        crate::spawn_msg(async move {
                            let result = github::search_code(&token, &full, &q).await;
                            Msg::CodeSearchDone { repo: full, result }
                        });
                    } else if let Loadable::Ready(hits) = results {
                        let path = hits.get(*sel).map(|h| h.path.clone());
                        if let Some(path) = path {
                            self.overlay = None;
                            let modified = self
                                .rv
                                .as_ref()
                                .and_then(|rv| rv.file.as_ref())
                                .map(|f| f.editor.modified)
                                .unwrap_or(false);
                            if modified {
                                self.overlay = Some(Overlay::Confirm {
                                    msg: format!("discard unsaved edits and open {}?", path),
                                    action: ConfirmAction::OpenFile(path),
                                });
                            } else {
                                self.open_file(path);
                            }
                        }
                    }
                }
                k => {
                    input.handle_key(&k, mods);
                }
            },
            Overlay::OpenRepo(input) => match key {
                Key::Esc => self.overlay = None,
                Key::Enter => {
                    let name = input.text.trim().trim_matches('/').to_string();
                    if name.is_empty() {
                        return;
                    }
                    self.overlay = None;
                    if name.contains('/') {
                        self.toast = Some((format!("opening {}…", name), false));
                        let token = self.token.clone();
                        crate::spawn_msg(async move {
                            Msg::RepoOpened(github::get_repo(&token, &name).await)
                        });
                    } else {
                        // Bare name: browse that organization (or user).
                        self.open_org(name);
                    }
                }
                k => {
                    input.handle_key(&k, mods);
                }
            },
            Overlay::BranchPick { sel, scroll } => {
                let count = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.branches.ready().map(|b| b.len()))
                    .unwrap_or(0);
                let view_h = self.layout.overlay_h.max(1);
                match key {
                    Key::Esc => self.overlay = None,
                    Key::Up => {
                        *sel = sel.saturating_sub(1);
                        if *sel < *scroll {
                            *scroll = *sel;
                        }
                    }
                    Key::Down => {
                        if count > 0 {
                            *sel = (*sel + 1).min(count - 1);
                        }
                        if *sel >= *scroll + view_h {
                            *scroll = *sel + 1 - view_h;
                        }
                    }
                    Key::Enter => {
                        let pick = self.rv.as_ref().and_then(|rv| {
                            rv.branches.ready().and_then(|b| b.get(*sel)).map(|b| b.name.clone())
                        });
                        self.overlay = None;
                        if let Some(name) = pick {
                            let modified = self
                                .rv
                                .as_ref()
                                .and_then(|rv| rv.file.as_ref())
                                .map(|f| f.editor.modified)
                                .unwrap_or(false);
                            let same = self
                                .rv
                                .as_ref()
                                .map(|rv| rv.branch == name)
                                .unwrap_or(true);
                            if same {
                                return;
                            }
                            if modified {
                                self.overlay = Some(Overlay::Confirm {
                                    msg: format!("discard unsaved edits and switch to {}?", name),
                                    action: ConfirmAction::SwitchBranch(name),
                                });
                            } else {
                                self.switch_branch(name);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Overlay::Confirm { action, .. } => {
                let action = action.clone();
                match key {
                    Key::Enter | Key::Char('y') => {
                        self.overlay = None;
                        match action {
                            ConfirmAction::LeaveRepo => {
                                self.route = Route::Repos;
                                self.rv = None;
                            }
                            ConfirmAction::SwitchBranch(name) => self.switch_branch(name),
                            ConfirmAction::OpenFile(path) => self.open_file(path),
                        }
                    }
                    Key::Esc | Key::Char('n') => self.overlay = None,
                    _ => {}
                }
            }
        }
    }

    fn on_paste(&mut self, text: String) {
        match &mut self.overlay {
            Some(Overlay::Commit(input)) | Some(Overlay::OpenRepo(input)) => {
                input.insert(&text.replace('\n', " "));
                return;
            }
            Some(Overlay::FileSearch { input, sel }) => {
                input.insert(&text.replace('\n', " "));
                *sel = 0;
                return;
            }
            Some(Overlay::CodeSearch { input, .. }) => {
                input.insert(&text.replace('\n', " "));
                return;
            }
            Some(_) => return,
            None => {}
        }
        match self.route {
            Route::Auth => self.token_input.insert(text.trim()),
            Route::Agent => {
                if self.anthropic_key.is_none() {
                    if self.agent.url_focused {
                        self.agent.url_input.insert(text.trim());
                    } else {
                        self.agent.key_input.insert(text.trim());
                    }
                } else {
                    self.agent.input.insert(&text.replace('\n', " "));
                }
            }
            Route::Repos if self.filter_active => self.filter.insert(&text.replace('\n', " ")),
            Route::Repo => {
                let lay = self.layout;
                if self.in_editor() {
                    if let Some(rv) = &mut self.rv {
                        if let Some(f) = &mut rv.file {
                            f.editor.insert_text(&text);
                            f.editor.ensure_visible(
                                lay.content_text.h.max(1) as usize,
                                lay.content_text.w.max(1) as usize,
                            );
                            rehighlight(f);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Apply a click resolved by the px view's hit-testing.
    pub fn perform_click(&mut self, click: Click) {
        self.toast = None;
        self.dirty = true;
        match click {
            Click::Repo(i) => {
                if self.repo_sel == i {
                    self.repos_key(Key::Enter, Mods::NONE);
                } else {
                    self.repo_sel = i;
                }
            }
            Click::TreeRow(i) => {
                let Some(rv) = &mut self.rv else { return };
                rv.focus = RepoFocus::Tree;
                if rv.tree_sel == i {
                    self.activate_tree_row(false);
                } else {
                    rv.tree_sel = i;
                }
            }
            Click::Tab(t) => {
                let Some(rv) = &mut self.rv else { return };
                rv.tab = t;
                if t == Tab::Actions && matches!(rv.runs, Loadable::Idle) {
                    self.load_runs();
                }
            }
            Click::BranchBtn => {
                if self.overlay.is_none() {
                    self.code_key(Key::Char('b'), Mods::NONE);
                }
            }
            Click::Run(i) => {
                let Some(rv) = &mut self.rv else { return };
                if rv.runs_sel == i {
                    self.actions_key(Key::Enter);
                } else {
                    rv.runs_sel = i;
                }
            }
            Click::EditorPos { row, cell_x } => {
                let Some(rv) = &mut self.rv else { return };
                rv.focus = RepoFocus::Content;
                if let Some(f) = &mut rv.file {
                    let row = row.min(f.editor.line_count() - 1);
                    let col = f.editor.x_to_col(row, cell_x);
                    f.editor.move_to((row, col), false);
                }
            }
            Click::OverlayItem(i) => {
                let sel = match &mut self.overlay {
                    Some(Overlay::BranchPick { sel, .. }) => Some(sel),
                    Some(Overlay::FileSearch { sel, .. }) => Some(sel),
                    Some(Overlay::CodeSearch { sel, .. }) => Some(sel),
                    _ => None,
                };
                if let Some(sel) = sel {
                    if *sel == i {
                        self.overlay_key(Key::Enter, Mods::NONE);
                    } else {
                        *sel = i;
                    }
                }
            }
            Click::EditBtn => self.begin_edit(),
            Click::CommitBtn => self.begin_commit(),
            Click::AgentClear => self.agent_clear(),
            Click::AgentResetKey => {
                if self.agent.busy {
                    self.agent_cancel();
                }
                crate::agent::clear_key();
                self.anthropic_key = None;
                self.agent.key_input.clear();
                // Pre-fill the endpoint so changing only the key keeps it.
                self.agent.url_input.clear();
                if let Some(u) = &self.anthropic_url {
                    self.agent.url_input.insert(u);
                }
                self.agent.url_focused = false;
            }
            Click::SortCycle => self.cycle_sort(),
            Click::SortDir => {
                self.sort_asc = !self.sort_asc;
                self.repo_sel = 0;
            }
            Click::ToggleForks => {
                self.hide_forks = !self.hide_forks;
                self.repo_sel = 0;
            }
            Click::ToggleArchived => {
                self.hide_archived = !self.hide_archived;
                self.repo_sel = 0;
            }
        }
    }

}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Search blob paths in the fetched tree: case-insensitive, every
/// whitespace-separated term must match, ranked by match position and path
/// length so filename hits beat deep-path hits.
pub fn search_tree(entries: &[TreeEntry], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().collect();
    let mut hits: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == "blob")
        .filter_map(|(i, e)| {
            let p = e.path.to_lowercase();
            if terms.is_empty() {
                return Some((p.len(), i));
            }
            let mut score = p.len();
            for t in &terms {
                match p.find(t) {
                    Some(pos) => score += pos,
                    None => return None,
                }
            }
            Some((score, i))
        })
        .collect();
    hits.sort_by_key(|h| h.0);
    hits.truncate(200);
    hits.into_iter().map(|(_, i)| i).collect()
}

/// Status/conclusion → (icon, color) for workflow runs, jobs, and steps.
pub fn run_icon(status: &str, conclusion: Option<&str>) -> (char, crate::ui::grid::Rgb) {
    use crate::ui::grid::Rgb;
    use crate::ui::theme;
    match (status, conclusion) {
        ("completed", Some("success")) => ('✓', theme::GREEN),
        ("completed", Some("failure")) => ('✗', theme::RED),
        ("completed", Some("cancelled")) => ('○', theme::DIM),
        ("completed", Some("skipped")) => ('○', theme::DIM),
        ("completed", _) => ('•', theme::DIM),
        ("in_progress", _) => ('●', theme::YELLOW),
        ("queued", _) | ("waiting", _) => ('●', Rgb(0x6e, 0x76, 0x81)),
        _ => ('•', theme::DIM),
    }
}

fn make_binary_file(path: String, sha: String, size: u64) -> OpenFile {
    OpenFile {
        path,
        sha,
        editor: Editor::from_text(""),
        lang: None,
        line_states: Vec::new(),
        binary: true,
        size,
        editing: false,
        committing: false,
    }
}

pub fn rehighlight(file: &mut OpenFile) {
    let n = file.editor.lines.len();
    let Some(spec) = file.lang else {
        file.line_states = vec![LineState::Normal; n];
        return;
    };
    let mut states = Vec::with_capacity(n);
    let mut st = LineState::Normal;
    for line in &file.editor.lines {
        states.push(st);
        st = highlight::highlight(spec, line, st).1;
    }
    file.line_states = states;
}

/// Flatten the recursive tree entries into visible rows honoring `expanded`.
pub fn rebuild_rows(rv: &mut RepoView) {
    let Some(entries) = rv.tree.ready() else {
        rv.rows.clear();
        return;
    };
    // parent dir -> indices of children
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let parent = e.path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        children.entry(parent).or_default().push(i);
    }
    for v in children.values_mut() {
        v.sort_by(|&a, &b| {
            let (ea, eb) = (&entries[a], &entries[b]);
            (eb.kind == "tree")
                .cmp(&(ea.kind == "tree"))
                .then_with(|| ea.path.to_lowercase().cmp(&eb.path.to_lowercase()))
        });
    }
    let mut rows = Vec::new();
    fn descend(
        entries: &[TreeEntry],
        children: &HashMap<&str, Vec<usize>>,
        expanded: &HashSet<String>,
        dir: &str,
        depth: usize,
        rows: &mut Vec<TreeRow>,
    ) {
        let Some(idxs) = children.get(dir) else { return };
        for &i in idxs {
            let e = &entries[i];
            let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_string();
            let is_dir = e.kind == "tree";
            rows.push(TreeRow { path: e.path.clone(), name, depth, is_dir });
            if is_dir && expanded.contains(&e.path) {
                descend(entries, children, expanded, &e.path, depth + 1, rows);
            }
        }
    }
    descend(entries, &children, &rv.expanded, "", 0, &mut rows);
    rv.rows = rows;
    if rv.tree_sel >= rv.rows.len() {
        rv.tree_sel = rv.rows.len().saturating_sub(1);
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
