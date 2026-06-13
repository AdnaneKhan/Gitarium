//! Application state machine: routes, async message handling, key/mouse
//! dispatch. Pure logic — drawing lives in px/view.rs, IO in github.rs.

pub mod editor;

use std::cell::Cell;
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
    /// Consecutive `pause_turn` resends, capped so a misbehaving server
    /// can't loop the turn forever.
    pub pause_count: u32,
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
            pause_count: 0,
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
        result: Result<github::RepoList, String>,
    },
    RepoOpened {
        name: String,
        result: Result<Repo, String>,
    },
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
        branch: String,
        path: String,
        result: Result<(String, Vec<u8>), String>,
    },
    Committed {
        repo: String,
        branch: String,
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
        query: String,
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

thread_local! {
    /// Mirror of `AgentChat::gen` readable from detached futures: a cancel
    /// bumps it, and the sequential tool batch re-checks it between
    /// executions so mutating calls stop instead of outliving the cancel.
    static LIVE_GEN: Cell<u64> = const { Cell::new(0) };
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
    /// Owner/name of the repo an async open (OpenRepo overlay) is fetching;
    /// a `RepoOpened` for anything else is stale and dropped.
    pub opening_repo: Option<String>,

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
            opening_repo: None,
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
                        result: github::list_repos_full(&token).await,
                    }
                });
            }
            RepoSource::Org(name) => {
                self.repos = Loadable::Loading;
                crate::spawn_msg(async move {
                    let result = github::list_owner_repos_full(&token, &name).await;
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
        // Supersedes any async open still in flight.
        self.opening_repo = None;
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
            Msg::FileLoaded { repo: full.clone(), branch, path, result }
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
        let text = file.editor.to_text();
        file.pending_commit = Some(text.clone());
        let content = github::b64_encode(text.as_bytes());
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
            Msg::Committed { repo: full, branch, path, result }
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
        let Some(key) = self.anthropic_key.clone() else {
            // Latent path (callers check the key) — but silently returning
            // here would leave busy=true forever.
            self.agent.busy = false;
            self.agent.push(AgentItem::Error("no API key configured".into()));
            return;
        };
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
        push_user_text(&mut self.agent.history, &text);
        self.agent.busy = true;
        self.agent.gen += 1;
        LIVE_GEN.with(|g| g.set(self.agent.gen));
        self.agent.pause_count = 0;
        self.agent_turn();
    }

    fn agent_cancel(&mut self) {
        self.agent.gen += 1; // orphan any in-flight future
        LIVE_GEN.with(|g| g.set(self.agent.gen));
        self.agent.busy = false;
        for &i in &self.agent.pending {
            if let Some(AgentItem::Tool { done, .. }) = self.agent.transcript.get_mut(i) {
                *done = Some(false);
            }
        }
        self.agent.pending.clear();
        self.sanitize_history_tail();
        self.agent.push(AgentItem::Error("cancelled".into()));
    }

    /// Leave `history` in a shape the Messages API accepts on the next
    /// send: the final message must not be an assistant turn with
    /// unanswered tool_use blocks, and content must stay non-empty. Text
    /// the model produced is kept (the transcript shows it, so the model
    /// should remember it); tool_use blocks that will never get results
    /// are stripped. Every terminal path (cancel, error, refusal,
    /// max_tokens) funnels through here.
    fn sanitize_history_tail(&mut self) {
        let Some(last) = self.agent.history.last_mut() else { return };
        if last["role"] != "assistant" {
            return;
        }
        let Some(blocks) = last["content"].as_array_mut() else {
            self.agent.history.pop();
            return;
        };
        blocks.retain(|b| b["type"] != "tool_use");
        // Thinking-only (or empty) remainders are dropped whole: the API
        // rejects assistant turns without displayable content.
        let keeps_text = blocks.iter().any(|b| {
            b["type"] == "text"
                && b["text"].as_str().map(|t| !t.trim().is_empty()).unwrap_or(false)
        });
        if !keeps_text {
            self.agent.history.pop();
        }
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
                // A pause_turn resend that failed leaves a trailing
                // assistant message; make the history sendable again.
                self.sanitize_history_tail();
                self.agent.push(AgentItem::Error(e));
                return;
            }
        };
        let content = resp["content"].clone();
        let stop = resp["stop_reason"].as_str().unwrap_or("").to_string();
        if stop != "pause_turn" {
            self.agent.pause_count = 0;
        }
        // An empty content array (pre-output refusal) must not enter the
        // history — the API rejects empty assistant turns on later sends.
        let has_content = content.as_array().map(|a| !a.is_empty()).unwrap_or(false);
        if has_content {
            self.agent
                .history
                .push(serde_json::json!({"role": "assistant", "content": content}));
        }
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
                    self.sanitize_history_tail();
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
                        // A cancel orphans the results; it must also stop
                        // the remaining (possibly mutating) executions.
                        if LIVE_GEN.with(|g| g.get()) != gen {
                            break;
                        }
                        let (text, ok) = crate::agent::exec(&token, c).await;
                        results.push((crate::agent::tool_result_block(c.id(), &text, ok), ok));
                    }
                    Msg::AgentToolsDone { gen, results }
                });
            }
            // Server-side pause (defensive — no server tools configured):
            // re-send and the API resumes where it left off.
            "pause_turn" => {
                self.agent.pause_count += 1;
                if self.agent.pause_count > 8 {
                    self.agent.busy = false;
                    self.agent.pause_count = 0;
                    self.sanitize_history_tail();
                    self.agent.push(AgentItem::Error("server kept pausing the turn".into()));
                } else {
                    self.agent_turn();
                }
            }
            "refusal" => {
                self.agent.busy = false;
                self.sanitize_history_tail();
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
                // The cut can land mid-tool-call; without sanitizing, every
                // later send would 400 on the unanswered tool_use.
                self.sanitize_history_tail();
                self.agent
                    .push(AgentItem::Error("response hit the token limit — say 'continue'".into()));
            }
            _ => {
                self.agent.busy = false; // end_turn
                self.sanitize_history_tail();
            }
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
                    Ok(list) => {
                        // Partial success (mid-pagination failure, page cap)
                        // must be visible, not a silently shorter list.
                        match &list.truncated {
                            Some(github::Truncation::Error(e)) => {
                                self.toast =
                                    Some((format!("repo list incomplete: {}", e), true));
                            }
                            Some(github::Truncation::MaxPages) => {
                                self.toast = Some((
                                    "repo list truncated (10,000 repo cap)".into(),
                                    true,
                                ));
                            }
                            None => {}
                        }
                        Loadable::Ready(list.repos)
                    }
                    Err(e) => Loadable::Failed(e),
                };
                self.repo_sel = 0;
                self.repo_scroll = 0;
            }
            Msg::RepoOpened { name, result } => {
                // Only the most recent async open may act; anything else is
                // a stale response the user has navigated away from.
                if self.opening_repo.as_deref() != Some(name.as_str()) {
                    return;
                }
                self.opening_repo = None;
                match result {
                    Ok(repo) => {
                        let modified = self
                            .rv
                            .as_ref()
                            .and_then(|rv| rv.file.as_ref())
                            .map(|f| f.editor.modified)
                            .unwrap_or(false);
                        if modified {
                            self.overlay = Some(Overlay::Confirm {
                                msg: format!(
                                    "discard unsaved edits and open {}?",
                                    repo.full_name
                                ),
                                action: ConfirmAction::OpenRepo(repo),
                            });
                        } else {
                            self.open_repo(repo);
                        }
                    }
                    Err(e) => self.toast = Some((e, true)),
                }
            }
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
            Msg::FileLoaded { repo, branch, path, result } => {
                let Some(rv) = &mut self.rv else { return };
                // The branch guard stops an old-branch response from winning
                // the race after a switch + reopen of the same path (commits
                // would then target the wrong base sha).
                if rv.repo.full_name != repo
                    || rv.branch != branch
                    || rv.file_loading.as_deref() != Some(path.as_str())
                {
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
                            pending_commit: None,
                        };
                        rehighlight(&mut file);
                        rv.file = Some(file);
                    }
                    Err(e) => self.toast = Some((e, true)),
                }
            }
            Msg::Committed { repo, branch, path, result } => {
                // The toast is always shown (the user should hear about a
                // failed commit even after navigating), but state is only
                // mutated when repo, branch and path all still match —
                // a stale result must not touch another view's sha/head.
                let fresh = self
                    .rv
                    .as_ref()
                    .map(|rv| rv.repo.full_name == repo && rv.branch == branch)
                    .unwrap_or(false);
                match &result {
                    Ok((_, commit_sha)) => {
                        let short: String = commit_sha.chars().take(7).collect();
                        self.toast = Some((format!("committed {} ✓", short), false));
                    }
                    Err(e) => self.toast = Some((format!("commit failed: {}", e), true)),
                }
                if !fresh {
                    return;
                }
                let Some(rv) = &mut self.rv else { return };
                let Some(file) = &mut rv.file else { return };
                if file.path != path {
                    return;
                }
                file.committing = false;
                let sent = file.pending_commit.take();
                if let Ok((content_sha, commit_sha)) = result {
                    file.sha = content_sha;
                    // Edits typed while the commit was in flight must stay
                    // marked dirty; only an unchanged buffer becomes clean.
                    // (`sent` is None when the file object was reloaded
                    // since — nothing to compare, leave `modified` alone.)
                    match sent {
                        Some(s) if s == file.editor.to_text() => file.editor.modified = false,
                        Some(_) => {
                            self.toast =
                                Some(("committed ✓ — buffer has newer edits".into(), false));
                        }
                        None => {}
                    }
                    // Keep the branch head fresh so tree reloads work.
                    if let Loadable::Ready(branches) = &mut rv.branches {
                        if let Some(b) = branches.iter_mut().find(|b| b.name == branch) {
                            if !commit_sha.is_empty() {
                                b.commit.sha = commit_sha.clone();
                            }
                        }
                    }
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
            Msg::CodeSearchDone { repo, query, result } => {
                let current = self.rv.as_ref().map(|rv| rv.repo.full_name.clone());
                if current.as_deref() != Some(repo.as_str()) {
                    return;
                }
                // `searched` guard: a reopened overlay (or a newer query)
                // must not be populated by an older search's results.
                if let Some(Overlay::CodeSearch { results, sel, searched, .. }) =
                    &mut self.overlay
                {
                    if *searched != query {
                        return;
                    }
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

    /// Returns true when the event was consumed (the host uses this for
    /// preventDefault — unconsumed keys keep their browser behavior).
    pub fn on_event(&mut self, ev: Event) -> bool {
        self.dirty = true;
        match ev {
            Event::Key(key, mods) => {
                self.toast = None;
                if self.overlay.is_some() {
                    return self.overlay_key(key, mods);
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

    fn auth_key(&mut self, key: Key, mods: Mods) -> bool {
        if self.auth_busy {
            return false;
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
                true
            }
            k => self.token_input.handle_key(&k, mods),
        }
    }

    fn repos_key(&mut self, key: Key, mods: Mods) -> bool {
        if self.filter_active {
            return match key {
                Key::Esc => {
                    self.filter.clear();
                    self.filter_active = false;
                    true
                }
                Key::Enter => {
                    self.filter_active = false;
                    true
                }
                Key::Up | Key::Down => {
                    self.filter_active = false;
                    self.repos_key(key, mods)
                }
                k => {
                    let used = self.filter.handle_key(&k, mods);
                    if used {
                        self.repo_sel = 0;
                        self.repo_scroll = 0;
                    }
                    used
                }
            };
        }
        let count = self.filtered_repos().len();
        match key {
            // Char bindings are plain-key only: with Ctrl/Alt (or Cmd, which
            // the host maps to ctrl) held they fall through unconsumed so
            // browser shortcuts keep working.
            Key::Char('?') if plain(mods) => self.overlay = Some(Overlay::Help),
            Key::Char('/') if plain(mods) => self.filter_active = true,
            Key::Char('i') if plain(mods) => self.open_agent(),
            Key::Char('o') if plain(mods) => {
                self.overlay = Some(Overlay::OpenRepo(LineInput::new(false)))
            }
            Key::Char('r') if plain(mods) => self.load_repos(),
            Key::Char('f') if plain(mods) => {
                self.hide_forks = !self.hide_forks;
                self.repo_sel = 0;
            }
            Key::Char('x') if plain(mods) => {
                self.hide_archived = !self.hide_archived;
                self.repo_sel = 0;
            }
            Key::Char('s') if plain(mods) => self.cycle_sort(),
            Key::Char('S') if plain(mods) => {
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
                } else {
                    return false;
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
            _ => return false,
        }
        true
    }

    fn agent_key(&mut self, key: Key, mods: Mods) -> bool {
        // No API key yet: the window shows the key/endpoint prompt.
        if self.anthropic_key.is_none() {
            return match key {
                Key::Esc => {
                    self.leave_agent();
                    true
                }
                Key::Tab | Key::BackTab | Key::Up | Key::Down => {
                    self.agent.url_focused = !self.agent.url_focused;
                    true
                }
                Key::Enter => {
                    let k = self.agent.key_input.text.trim().to_string();
                    if k.is_empty() {
                        return true;
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
                    true
                }
                k => {
                    if self.agent.url_focused {
                        self.agent.url_input.handle_key(&k, mods)
                    } else {
                        self.agent.key_input.handle_key(&k, mods)
                    }
                }
            };
        }
        match key {
            Key::Esc => {
                if self.agent.busy {
                    self.agent_cancel();
                } else {
                    self.leave_agent();
                }
                true
            }
            Key::Enter => {
                self.agent_send();
                true
            }
            k => self.agent.input.handle_key(&k, mods),
        }
    }

    fn repo_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(rv) = &mut self.rv else { return false };
        match rv.tab {
            Tab::Code => self.code_key(key, mods),
            Tab::Actions => self.actions_key(key, mods),
        }
    }

    fn code_key(&mut self, key: Key, mods: Mods) -> bool {
        let in_editor = self.in_editor();
        let Some(rv) = self.rv.as_mut() else { return false };

        // Editor consumes nearly everything while editing.
        if in_editor {
            if key == Key::Char('s') && mods.ctrl {
                self.begin_commit();
                return true;
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
                return true;
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
                // Plain keys belong to the editor even when they change
                // nothing (arrow at a boundary); modified combos it didn't
                // act on stay with the browser.
                return changed || plain(mods);
            }
            return false;
        }

        match key {
            Key::Char('?') if plain(mods) => self.overlay = Some(Overlay::Help),
            Key::Char('/') if plain(mods) => {
                self.overlay = Some(Overlay::FileSearch { input: LineInput::new(false), sel: 0 });
            }
            Key::Char('g') if plain(mods) => {
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
            Key::Char('b') if plain(mods) => {
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
            Key::Char('a') if plain(mods) => {
                rv.tab = Tab::Actions;
                if matches!(rv.runs, Loadable::Idle) {
                    self.load_runs();
                }
            }
            Key::Char('e') if plain(mods) => self.begin_edit(),
            Key::Char('c') if plain(mods) => self.begin_commit(),
            Key::Char('i') if plain(mods) => self.open_agent(),
            Key::Tab => {
                rv.focus = match rv.focus {
                    RepoFocus::Tree if rv.file.is_some() => RepoFocus::Content,
                    _ => RepoFocus::Tree,
                };
            }
            Key::Esc => {
                if rv.focus == RepoFocus::Content {
                    rv.focus = RepoFocus::Tree;
                    return true;
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
            _ => {
                return match rv.focus {
                    RepoFocus::Tree => self.tree_key(key),
                    RepoFocus::Content => self.viewer_key(key),
                }
            }
        }
        true
    }

    fn tree_key(&mut self, key: Key) -> bool {
        let Some(rv) = self.rv.as_mut() else { return false };
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
                let Some(row) = rv.rows.get(rv.tree_sel) else { return false };
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
            _ => return false,
        }
        true
    }

    fn activate_tree_row(&mut self, expand_only: bool) {
        let Some(rv) = self.rv.as_mut() else { return };
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

    fn viewer_key(&mut self, key: Key) -> bool {
        let lay = self.layout;
        let Some(rv) = self.rv.as_mut() else { return false };
        let Some(f) = &mut rv.file else { return false };
        let h = lay.content_text.h.max(1) as usize;
        match key {
            Key::Up => f.editor.scroll_by(-1, h),
            Key::Down => f.editor.scroll_by(1, h),
            Key::PageUp => f.editor.scroll_by(-(h as i32), h),
            Key::PageDown => f.editor.scroll_by(h as i32, h),
            Key::Home => f.editor.scroll = 0,
            Key::End => f.editor.scroll = f.editor.line_count().saturating_sub(h),
            _ => return false,
        }
        true
    }

    fn actions_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(rv) = self.rv.as_mut() else { return false };
        let count = rv.runs.ready().map(|r| r.len()).unwrap_or(0);
        match key {
            Key::Char('?') if plain(mods) => self.overlay = Some(Overlay::Help),
            Key::Char('a') if plain(mods) => rv.tab = Tab::Code,
            Key::Esc => rv.tab = Tab::Code,
            Key::Char('i') if plain(mods) => self.open_agent(),
            Key::Char('r') if plain(mods) => self.load_runs(),
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
            _ => return false,
        }
        true
    }

    fn begin_edit(&mut self) {
        let anonymous = self.token.is_none();
        let Some(rv) = self.rv.as_mut() else { return };
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
        let Some(rv) = self.rv.as_mut() else { return };
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

    fn overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(overlay) = &mut self.overlay else { return false };
        match overlay {
            Overlay::Help => {
                self.overlay = None;
                true
            }
            Overlay::Commit(input) => match key {
                Key::Esc => {
                    self.overlay = None;
                    true
                }
                Key::Enter => {
                    let msg = input.text.trim().to_string();
                    if msg.is_empty() {
                        return true;
                    }
                    self.overlay = None;
                    self.commit_file(msg);
                    true
                }
                k => input.handle_key(&k, mods),
            },
            Overlay::FileSearch { input, sel } => match key {
                Key::Esc => {
                    self.overlay = None;
                    true
                }
                Key::Up => {
                    *sel = sel.saturating_sub(1);
                    true
                }
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
                    true
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
                    true
                }
                k => {
                    let used = input.handle_key(&k, mods);
                    if used {
                        *sel = 0;
                    }
                    used
                }
            },
            Overlay::CodeSearch { input, sel, searched, results } => match key {
                Key::Esc => {
                    self.overlay = None;
                    true
                }
                Key::Up => {
                    *sel = sel.saturating_sub(1);
                    true
                }
                Key::Down => {
                    let count = results.ready().map(|h| h.len()).unwrap_or(0);
                    if count > 0 {
                        *sel = (*sel + 1).min(count - 1);
                    }
                    true
                }
                Key::Enter => {
                    let q = input.text.trim().to_string();
                    if q.is_empty() {
                        return true;
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
                            Msg::CodeSearchDone { repo: full, query: q, result }
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
                    true
                }
                k => input.handle_key(&k, mods),
            },
            Overlay::OpenRepo(input) => match key {
                Key::Esc => {
                    self.overlay = None;
                    true
                }
                Key::Enter => {
                    let name = input.text.trim().trim_matches('/').to_string();
                    if name.is_empty() {
                        return true;
                    }
                    self.overlay = None;
                    if name.contains('/') {
                        self.toast = Some((format!("opening {}…", name), false));
                        self.opening_repo = Some(name.clone());
                        let token = self.token.clone();
                        crate::spawn_msg(async move {
                            let result = github::get_repo(&token, &name).await;
                            Msg::RepoOpened { name, result }
                        });
                    } else {
                        // Bare name: browse that organization (or user).
                        self.open_org(name);
                    }
                    true
                }
                k => input.handle_key(&k, mods),
            },
            Overlay::BranchPick { sel, scroll } => {
                let count = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.branches.ready().map(|b| b.len()))
                    .unwrap_or(0);
                let view_h = self.layout.overlay_h.max(1);
                match key {
                    Key::Esc => {
                        self.overlay = None;
                        true
                    }
                    Key::Up => {
                        *sel = sel.saturating_sub(1);
                        if *sel < *scroll {
                            *scroll = *sel;
                        }
                        true
                    }
                    Key::Down => {
                        if count > 0 {
                            *sel = (*sel + 1).min(count - 1);
                        }
                        if *sel >= *scroll + view_h {
                            *scroll = *sel + 1 - view_h;
                        }
                        true
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
                                return true;
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
                        true
                    }
                    _ => false,
                }
            }
            Overlay::Confirm { action, .. } => {
                let action = action.clone();
                match key {
                    Key::Enter | Key::Char('y') if plain(mods) => {
                        self.overlay = None;
                        match action {
                            ConfirmAction::LeaveRepo => {
                                self.route = Route::Repos;
                                self.rv = None;
                            }
                            ConfirmAction::SwitchBranch(name) => self.switch_branch(name),
                            ConfirmAction::OpenFile(path) => self.open_file(path),
                            ConfirmAction::OpenRepo(repo) => self.open_repo(repo),
                        }
                        true
                    }
                    Key::Esc | Key::Char('n') if plain(mods) => {
                        self.overlay = None;
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    fn on_paste(&mut self, text: String) -> bool {
        match &mut self.overlay {
            Some(Overlay::Commit(input)) | Some(Overlay::OpenRepo(input)) => {
                input.insert(&text.replace('\n', " "));
                return true;
            }
            Some(Overlay::FileSearch { input, sel }) => {
                input.insert(&text.replace('\n', " "));
                *sel = 0;
                return true;
            }
            Some(Overlay::CodeSearch { input, .. }) => {
                input.insert(&text.replace('\n', " "));
                return true;
            }
            Some(_) => return false,
            None => {}
        }
        match self.route {
            Route::Auth => {
                // Same lock as typed keys: no mutating the token while a
                // validation request is in flight.
                if self.auth_busy {
                    return false;
                }
                self.token_input.insert(text.trim());
                true
            }
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
                true
            }
            Route::Repos if self.filter_active => {
                self.filter.insert(&text.replace('\n', " "));
                // Pasted filter text resets selection like typed chars do.
                self.repo_sel = 0;
                self.repo_scroll = 0;
                true
            }
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
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
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
                    self.actions_key(Key::Enter, Mods::NONE);
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

/// Append user text to the Messages history, merging into a trailing user
/// message when one exists (cancel/error paths can leave the history ending
/// on a tool_result turn; consecutive user messages are rejected by the
/// API, while one message with tool_results followed by text is valid).
fn push_user_text(history: &mut Vec<serde_json::Value>, text: &str) {
    use serde_json::{json, Value};
    if let Some(last) = history.last_mut() {
        if last["role"] == "user" {
            let block = json!({"type": "text", "text": text});
            match &mut last["content"] {
                Value::String(s) => {
                    let prev = std::mem::take(s);
                    last["content"] = json!([{"type": "text", "text": prev}, block]);
                }
                Value::Array(a) => a.push(block),
                _ => {}
            }
            return;
        }
    }
    history.push(json!({"role": "user", "content": text}));
}

/// A char binding fires only without Ctrl/Alt (Cmd arrives as ctrl from the
/// host) so browser shortcuts are never shadowed by single-key bindings.
fn plain(mods: Mods) -> bool {
    !mods.ctrl && !mods.alt
}

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
        pending_commit: None,
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
