//! The Actions tab: workflow runs, their jobs, and the tab's keys.

use crate::github::{self, Job, Run};
use crate::ui::input::{Key, Mods};

use super::keys::plain;
use super::{App, Loadable, Msg, Tab};

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

    pub(super) fn actions_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(rv) = self.rv.as_mut() else { return false };
        let count = rv.runs.ready().map(|r| r.len()).unwrap_or(0);
        match key {
            Key::Char('?') if plain(mods) => self.overlay = Some(super::Overlay::Help),
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
