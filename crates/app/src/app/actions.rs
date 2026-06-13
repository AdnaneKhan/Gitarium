//! The Actions tab: workflow runs, their jobs, and the tab's keys.

use crate::github::{self, Job, Run};
use crate::ui::input::{Key, Mods};
use crate::ui::lineinput::LineInput;

use super::keys::plain;
use super::{App, Loadable, LogSearch, Msg, Tab};

/// localStorage key for a job's cached log.
fn log_cache_key(repo: &str, job_id: u64) -> String {
    format!("rustvm_joblog:{}:{}", repo, job_id)
}

impl App {
    pub(super) fn load_runs(&mut self) {
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

    pub(super) fn on_runs(&mut self, repo: String, result: Result<Vec<Run>, String>) {
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

    pub(super) fn on_jobs(&mut self, repo: String, run_id: u64, result: Result<Vec<Job>, String>) {
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

    /// Drill into a job's logs (requires auth — the logs endpoint is
    /// token-only). Served from the localStorage cache when present (no API
    /// call); otherwise fetched into a scrollable view.
    pub(super) fn open_job_logs(&mut self, job_id: u64) {
        if self.login.is_none() {
            self.toast = Some(("sign in to view job logs".into(), true));
            return;
        }
        let Some(rv) = &mut self.rv else { return };
        let repo = rv.repo.full_name.clone();
        rv.jobs_scroll = 0;
        rv.log_search = None;
        // Cache hit → no request.
        if let Some(text) = crate::store::get(&log_cache_key(&repo, job_id)) {
            rv.job_logs = Some((job_id, Loadable::Ready(text)));
            return;
        }
        rv.job_logs = Some((job_id, Loadable::Loading));
        let token = self.token.clone();
        crate::spawn_msg(async move {
            Msg::JobLogs {
                repo: repo.clone(),
                job_id,
                result: github::get_job_logs(&token, &repo, job_id).await,
            }
        });
    }

    /// Back out of the log view to the jobs list.
    pub(super) fn close_job_logs(&mut self) {
        if let Some(rv) = &mut self.rv {
            rv.job_logs = None;
            rv.jobs_scroll = 0;
            rv.log_search = None;
        }
    }

    pub(super) fn on_job_logs(&mut self, repo: String, job_id: u64, result: Result<String, String>) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        // Cache the log so the same job is never re-fetched — but only once
        // the job is completed (an in-progress log keeps growing).
        if let Ok(text) = &result {
            let done = rv
                .jobs
                .as_ref()
                .and_then(|(_, l)| l.ready())
                .and_then(|js| js.iter().find(|j| j.id == job_id))
                .map(|j| j.status == "completed")
                .unwrap_or(false);
            if done {
                crate::store::set(&log_cache_key(&repo, job_id), text);
            }
        }
        if let Some((id, slot)) = &mut rv.job_logs {
            if *id == job_id {
                *slot = match result {
                    Ok(t) => Loadable::Ready(t),
                    Err(e) => Loadable::Failed(e),
                };
            }
        }
    }

    // ---- in-log search + scrolling --------------------------------------

    pub(super) fn open_log_search(&mut self) {
        if let Some(rv) = &mut self.rv {
            if rv.log_search.is_none() {
                rv.log_search = Some(LogSearch { query: LineInput::new(false), matches: Vec::new(), idx: 0 });
            }
        }
    }

    pub(super) fn close_log_search(&mut self) {
        if let Some(rv) = &mut self.rv {
            rv.log_search = None;
        }
    }

    /// Recompute the matching line indices for the current query and jump to
    /// the first one.
    fn recompute_log_matches(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        let q = match &rv.log_search {
            Some(s) => s.query.text.trim().to_lowercase(),
            None => return,
        };
        let matches: Vec<usize> = match &rv.job_logs {
            Some((_, Loadable::Ready(text))) if !q.is_empty() => text
                .lines()
                .enumerate()
                .filter(|(_, l)| l.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        };
        let first = matches.first().copied();
        if let Some(s) = &mut rv.log_search {
            s.matches = matches;
            s.idx = 0;
        }
        if let Some(m) = first {
            rv.jobs_scroll = m.saturating_sub(2);
        }
    }

    /// Step to the next (`+1`) / previous (`-1`) match and scroll to it.
    pub(super) fn log_match_step(&mut self, dir: i32) {
        let Some(rv) = &mut self.rv else { return };
        let line = {
            let Some(s) = &mut rv.log_search else { return };
            if s.matches.is_empty() {
                return;
            }
            let n = s.matches.len() as i32;
            s.idx = (((s.idx as i32 + dir) % n + n) % n) as usize;
            s.matches[s.idx]
        };
        rv.jobs_scroll = line.saturating_sub(2);
    }

    fn scroll_log(&mut self, delta: i32) {
        let visible = self.layout.jobs_h.max(1);
        let Some(rv) = &mut self.rv else { return };
        let lines = match &rv.job_logs {
            Some((_, Loadable::Ready(t))) => t.lines().count(),
            _ => return,
        };
        let max = lines.saturating_sub(visible) as i32;
        rv.jobs_scroll = (rv.jobs_scroll as i32 + delta).clamp(0, max.max(0)) as usize;
    }

    /// Keys while a job-log view is open (search box or plain log view).
    fn job_log_key(&mut self, key: Key, mods: Mods) -> bool {
        let searching = self.rv.as_ref().map(|rv| rv.log_search.is_some()).unwrap_or(false);
        if searching {
            match key {
                Key::Esc => self.close_log_search(),
                Key::Enter => self.log_match_step(if mods.shift { -1 } else { 1 }),
                Key::Up => self.log_match_step(-1),
                Key::Down => self.log_match_step(1),
                k => {
                    let changed = self
                        .rv
                        .as_mut()
                        .and_then(|rv| rv.log_search.as_mut())
                        .map(|s| s.query.handle_key(&k, mods))
                        .unwrap_or(false);
                    if changed {
                        self.recompute_log_matches();
                    }
                }
            }
            return true;
        }
        match key {
            Key::Esc => self.close_job_logs(),
            Key::Char('/') if plain(mods) => self.open_log_search(),
            Key::Up => self.scroll_log(-1),
            Key::Down => self.scroll_log(1),
            Key::PageUp => self.scroll_log(-(self.layout.jobs_h.max(1) as i32)),
            Key::PageDown => self.scroll_log(self.layout.jobs_h.max(1) as i32),
            _ => {}
        }
        true
    }

    pub(super) fn actions_key(&mut self, key: Key, mods: Mods) -> bool {
        // The log view (drilled into a job) has its own keys: search box,
        // scrolling, and Esc to back out.
        if self.rv.as_ref().map(|rv| rv.job_logs.is_some()).unwrap_or(false) {
            return self.job_log_key(key, mods);
        }
        let Some(rv) = self.rv.as_mut() else { return false };
        let count = rv.runs.ready().map(|r| r.len()).unwrap_or(0);
        match key {
            Key::Char('?') if plain(mods) => self.overlay = Some(super::Overlay::Help),
            Key::Char('a') if plain(mods) => self.switch_tab(Tab::Actions),
            Key::Char('t') if plain(mods) => self.switch_tab(Tab::Issues),
            Key::Char('p') if plain(mods) => self.switch_tab(Tab::Pulls),
            Key::Esc => self.switch_tab(Tab::Code),
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
