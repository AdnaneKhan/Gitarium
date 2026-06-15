//! Shared state types: routes, list sources, focus, overlays, hit-regions,
//! layout, and the open-file model.

use crate::github::Repo;
use crate::highlight::{LangSpec, LineState};
use crate::ui::grid::Rect;
use crate::ui::lineinput::LineInput;

use super::editor::Editor;
use super::settings::SettingsForm;

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
    Issues,
    Pulls,
    Actions,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepoFocus {
    Tree,
    Content,
}

/// What a code-search palette searches, and how opening a hit behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    /// Within the currently-open repo; opening a hit loads the file here.
    Repo,
    /// Across GitHub (the Repos screen); opening a hit fetches that repo,
    /// then jumps to the file.
    Global,
}

/// A pending change to one path in the staged workspace.
#[derive(Clone)]
pub enum Staged {
    /// Add a new file or modify an existing one: the full new file text.
    Upsert(String),
    /// Remove the path from the tree.
    Delete,
}

/// Author/committer/date overrides for the next commit. Empty name+email
/// falls back to the token's GitHub identity; an empty date means "now".
/// Persisted on the `App` so the values stick across commits in a session.
#[derive(Clone, Default)]
pub struct CommitIdentity {
    pub author_name: String,
    pub author_email: String,
    pub committer_name: String,
    pub committer_email: String,
    pub date: String,
}

/// Where a commit lands: the current branch, or a brand-new branch / tag
/// pointed at the new commit. Cycled by the commit dialog's target chip.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommitTarget {
    Current,
    NewBranch,
    NewTag,
}

impl CommitTarget {
    pub fn next(self) -> Self {
        match self {
            CommitTarget::Current => CommitTarget::NewBranch,
            CommitTarget::NewBranch => CommitTarget::NewTag,
            CommitTarget::NewTag => CommitTarget::Current,
        }
    }
    pub fn prev(self) -> Self {
        self.next().next()
    }
}

/// The multi-field commit overlay: message, the identity overrides, and the
/// destination. `field` selects the focused control (0 = message, 1‥5 =
/// override rows, 6 = target chip, 7 = new ref name).
pub struct CommitForm {
    pub message: LineInput,
    pub author_name: LineInput,
    pub author_email: LineInput,
    pub committer_name: LineInput,
    pub committer_email: LineInput,
    pub date: LineInput,
    /// Destination: current branch, or a new branch / tag.
    pub target: CommitTarget,
    /// Name for the new branch / tag (unused when target is Current).
    pub new_ref: LineInput,
    pub field: usize,
}

impl CommitForm {
    /// Focusable controls (Tab cycles through these).
    pub const FIELDS: usize = 8;
    /// `field` index of the target chip and the new-ref name input.
    pub const TARGET_FIELD: usize = 6;
    pub const REF_FIELD: usize = 7;

    /// Seed a fresh form, pre-filling the override rows from `id`.
    pub fn new(id: &CommitIdentity) -> Self {
        let mk = |s: &str| {
            let mut l = LineInput::new(false);
            if !s.is_empty() {
                l.insert(s);
            }
            l
        };
        CommitForm {
            message: LineInput::new(false),
            author_name: mk(&id.author_name),
            author_email: mk(&id.author_email),
            committer_name: mk(&id.committer_name),
            committer_email: mk(&id.committer_email),
            date: mk(&id.date),
            target: CommitTarget::Current,
            new_ref: LineInput::new(false),
            field: 0,
        }
    }

    /// The focused text input, for key routing. The target chip (field 6)
    /// isn't a text input, so it maps to a harmless field — the key handler
    /// special-cases it before reaching here.
    pub fn focused(&mut self) -> &mut LineInput {
        match self.field {
            1 => &mut self.author_name,
            2 => &mut self.author_email,
            3 => &mut self.committer_name,
            4 => &mut self.committer_email,
            5 => &mut self.date,
            7 => &mut self.new_ref,
            _ => &mut self.message,
        }
    }

    /// Snapshot the override rows (trimmed) to persist back onto the `App`.
    pub fn identity(&self) -> CommitIdentity {
        let t = |l: &LineInput| l.text.trim().to_string();
        CommitIdentity {
            author_name: t(&self.author_name),
            author_email: t(&self.author_email),
            committer_name: t(&self.committer_name),
            committer_email: t(&self.committer_email),
            date: t(&self.date),
        }
    }
}

/// A floating right-click menu, anchored at (`x`, `y`) in device pixels.
pub struct ContextMenu {
    pub x: f32,
    pub y: f32,
    pub items: Vec<MenuItem>,
}

pub struct MenuItem {
    pub label: String,
    pub action: MenuAction,
}

/// What a context-menu item does when chosen.
#[derive(Clone)]
pub enum MenuAction {
    /// Open the new-file prompt, pre-filled with this directory prefix.
    NewFile(String),
    /// Stage a deletion of this path.
    Delete(String),
    /// Drop this path's staged change.
    Unstage(String),
    /// Download this folder (empty = whole repo) as a `.tar.gz`.
    DownloadDir(String),
    /// Download this single file as a one-entry `.tar.gz`.
    DownloadFile(String),
    /// Delete a workflow run (Actions tab) by its database id.
    DeleteRun(u64),
}

pub enum Overlay {
    Commit(CommitForm),
    BranchPick { sel: usize, scroll: usize },
    OpenRepo(LineInput),
    /// New-file prompt: the path to create as a staged, empty file.
    NewFile(LineInput),
    /// New-branch modal: pick the base branch (index into `branches`) and the
    /// new branch name; Create makes the ref immediately.
    NewBranch { name: LineInput, base: usize },
    /// Model picker: the provider's models, fetched on open; selecting one
    /// sets the agent model (the id itself is never displayed in the toolbar).
    ModelPick { models: Loadable<Vec<crate::agent::ModelInfo>>, sel: usize },
    /// Find-file palette over the already-fetched recursive tree.
    FileSearch { input: LineInput, sel: usize },
    /// GitHub code-search palette (token required; default branch only).
    /// `scope` selects repo-local vs. global search.
    CodeSearch {
        input: LineInput,
        sel: usize,
        /// Last submitted query — Enter searches when the input differs,
        /// opens the selected hit when it matches.
        searched: String,
        results: Loadable<Vec<crate::github::CodeHit>>,
        scope: SearchScope,
        /// 1-based index of the last page appended (0 before the first
        /// result lands); "load more" then requests `page + 1`.
        page: u32,
        /// Another page may exist — accumulated hits are below the query's
        /// total and GitHub's 1000-result search cap. Drives the load-more
        /// trigger and the hint.
        more: bool,
        /// A next-page fetch is in flight: suppresses duplicate load-more
        /// requests and shows a "loading more" hint.
        loading_more: bool,
    },
    Help,
    /// A settings create/edit form (secrets, variables, deploy keys, …).
    SettingsForm(SettingsForm),
    Confirm { msg: String, action: ConfirmAction },
    /// A mutating github_api turn in the interactive agent, paused for manual
    /// approval before it runs. `summary` lists the write call(s); `content`
    /// is the assistant turn, re-parsed and dispatched on approval or answered
    /// with a refusal on deny. Reached only in the app — the headless agent
    /// runs autonomously and never gates writes.
    AgentApproval { summary: String, content: serde_json::Value },
    /// Risk warning shown before YOLO mode (auto-approve agent writes) is
    /// turned on; confirming enables it, aborting leaves it off.
    YoloWarn,
}

#[derive(Clone)]
pub enum ConfirmAction {
    LeaveRepo,
    SwitchBranch(String),
    OpenFile(String),
    /// An async `RepoOpened` landed while the current file had unsaved
    /// edits; opening the fetched repo needs the usual confirm. `then_open`
    /// carries a file path to jump to once the repo is open (global code
    /// search), or None for a plain repo open.
    OpenRepo { repo: Repo, then_open: Option<String> },
    /// Submit an approving review on the open PR.
    ApprovePr(u64),
    /// Merge the open PR with the given method ("merge"|"squash"|"rebase").
    MergePr { number: u64, method: String },
    /// Delete a workflow run (Actions tab) from the given repo.
    DeleteRun { repo: String, run_id: u64 },
    /// Delete an Actions secret / variable, or a deploy key, from the Settings tab.
    DeleteSecret { repo: String, name: String },
    DeleteVariable { repo: String, name: String },
    DeleteDeployKey { repo: String, id: i64 },
    /// Remove a collaborator, cancel a pending invite, delete a webhook
    /// (Settings tab), and the General danger zone: archive / permanently
    /// delete the repository.
    RemoveCollaborator { repo: String, user: String },
    CancelInvitation { repo: String, invite_id: i64 },
    DeleteWebhook { repo: String, id: i64 },
    ArchiveRepo { repo: String },
    DeleteRepo { repo: String },
}

/// Mouse hit-regions, rebuilt on every draw.
#[derive(Clone, Copy, PartialEq)]
pub enum Click {
    Repo(usize), // index into the *filtered* repo list
    TreeRow(usize),
    Tab(Tab),
    BranchBtn,
    Run(usize),
    /// A job header in the jobs pane → open that job's logs (index into the
    /// loaded jobs list).
    JobRow(usize),
    /// "‹ BACK" in the job-log view → back to the jobs list.
    JobLogBack,
    /// "Download Log" chip → save the cached log as a text file (handled in
    /// the view layer, which has DOM access).
    DownloadLog,
    /// Log-search controls: open the box, jump prev/next, close.
    LogSearchOpen,
    LogSearchPrev,
    LogSearchNext,
    LogSearchClose,
    /// A row in the issues or pulls list (which one depends on the active
    /// tab); opening it shows the issue/PR detail.
    IssueRow(usize),
    /// Issue/PR detail in-page search controls (open box, step, close).
    DetailSearchOpen,
    DetailSearchPrev,
    DetailSearchNext,
    DetailSearchClose,
    /// Detail-view actions on the open PR.
    Approve,
    Merge,
    /// Cycle the merge method chip (merge → squash → rebase).
    MergeMethodCycle,
    /// "‹ BACK" in the detail view → back to the list.
    DetailBack,
    /// Direct editor position: row + visual cell x (converted to a char
    /// column via x_to_col).
    EditorPos { row: usize, cell_x: usize },
    OverlayItem(usize),
    EditBtn,
    StageBtn,
    CommitBtn,
    NewFileBtn,
    /// The commit dialog's destination chip (cycles current/new branch/tag).
    CommitCycleTarget,
    /// "+ New branch" in the branch picker → opens the new-branch modal.
    NewBranchBtn,
    /// The new-branch modal's base chip (cycles the base branch).
    CycleBranchBase,
    /// "MODEL" chip in the agent toolbar → opens the model picker.
    ModelPickBtn,
    AgentClear,
    AgentResetKey,
    /// "YOLO" chip in the agent toolbar → toggle auto-approve of mutating
    /// API calls (enabling first pops a risk modal; disabling is immediate).
    AgentYolo,
    /// A hyperlink in the agent transcript: index into the px view's
    /// per-frame url table. Opened by the view layer (browser `window.open`),
    /// not `perform_click` — the app crate has no DOM access.
    OpenUrl(usize),
    SortCycle,
    SortDir,
    ToggleForks,
    ToggleArchived,
    /// Settings tab: a left-nav section row, a right-content list row, and the
    /// add / edit / delete / chip-cycle affordances.
    SettingsNav(usize),
    SettingsRow(usize),
    SettingsAdd,
    SettingsEdit,
    SettingsDelete,
    SettingsCycleChip,
    /// Click a simple-form input field → focus it (index into `fields`).
    SettingsFocusField(usize),
    /// Webhook (Multi) form: cycle the content-type chip, toggle one event row.
    SettingsCycleContentType,
    SettingsToggleEvent(usize),
    /// General danger-zone buttons.
    SettingsArchiveRepo,
    SettingsDeleteRepo,
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
    Issues,
    Detail,
    /// The PR detail's right column (checks / reviews / mergeability).
    DetailMeta,
    /// Settings tab: the section nav list and the section's content list.
    SettingsNav,
    SettingsList,
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
    /// Visible rows in the issues/pulls list and the detail body (for paging).
    pub issues_h: usize,
    pub detail_h: usize,
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
            issues_h: 0,
            detail_h: 0,
        }
    }
}

pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

/// In-log text search: the query, the line indices that match, and which one
/// is the current jump target (`< >` / Enter step through `matches`).
pub struct LogSearch {
    pub query: LineInput,
    pub matches: Vec<usize>,
    pub idx: usize,
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
}
