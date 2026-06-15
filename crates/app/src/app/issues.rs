//! The Issues and Pulls tabs: lazy list loading (100 most-recently-updated),
//! their async results, tab switching, and the shared list-key navigation.
//! The per-row detail view lives in `issue_detail.rs`.

use crate::github::{self, Issue, Pull};
use crate::ui::input::{Key, Mods};

use super::keys::plain;
use super::{App, Loadable, Msg, Overlay, Tab};

impl App {
    /// Switch the Repo route's tab, lazily loading the new tab's data the
    /// first time it's shown. Leaving Issues/Pulls drops any open detail.
    pub(super) fn switch_tab(&mut self, tab: Tab) {
        let load = {
            let Some(rv) = self.rv.as_mut() else { return };
            if rv.tab != tab {
                rv.detail = None;
            }
            rv.tab = tab;
            match tab {
                Tab::Actions => matches!(rv.runs, Loadable::Idle),
                Tab::Issues => matches!(rv.issues, Loadable::Idle),
                Tab::Pulls => matches!(rv.pulls, Loadable::Idle),
                Tab::Code => false,
                Tab::Settings => false, // General needs no fetch; sections load on selection.
            }
        };
        if load {
            match tab {
                Tab::Actions => self.load_runs(),
                Tab::Issues => self.load_issues(),
                Tab::Pulls => self.load_pulls(),
                Tab::Code => {}
                Tab::Settings => {}
            }
        }
    }

    pub(super) fn load_issues(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        rv.issues = Loadable::Loading;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::IssuesLoaded { repo: full.clone(), result: github::list_issues(&token, &full).await }
        });
    }

    pub(super) fn load_pulls(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        rv.pulls = Loadable::Loading;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::PullsLoaded { repo: full.clone(), result: github::list_pulls(&token, &full).await }
        });
    }

    pub(super) fn on_issues_loaded(&mut self, repo: String, result: Result<Vec<Issue>, String>) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        rv.issues = match result {
            Ok(v) => Loadable::Ready(v),
            Err(e) => Loadable::Failed(e),
        };
        rv.issues_sel = 0;
        rv.issues_scroll = 0;
    }

    pub(super) fn on_pulls_loaded(&mut self, repo: String, result: Result<Vec<Pull>, String>) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        rv.pulls = match result {
            Ok(v) => Loadable::Ready(v),
            Err(e) => Loadable::Failed(e),
        };
        rv.pulls_sel = 0;
        rv.pulls_scroll = 0;
    }

    /// Key handling for both the Issues and Pulls list (active tab decides
    /// which list). Detail-view keys are handled separately in `detail_key`.
    pub(super) fn issues_key(&mut self, key: Key, mods: Mods) -> bool {
        let page = self.layout.issues_h.max(1);
        let (is_pulls, count, sel) = {
            let Some(rv) = self.rv.as_ref() else { return false };
            let is_pulls = matches!(rv.tab, Tab::Pulls);
            let count = if is_pulls {
                rv.pulls.ready().map(|v| v.len()).unwrap_or(0)
            } else {
                rv.issues.ready().map(|v| v.len()).unwrap_or(0)
            };
            let sel = if is_pulls { rv.pulls_sel } else { rv.issues_sel };
            (is_pulls, count, sel)
        };
        match key {
            Key::Char('?') if plain(mods) => {
                self.overlay = Some(Overlay::Help);
                return true;
            }
            Key::Char('i') if plain(mods) => {
                self.open_agent();
                return true;
            }
            Key::Char('t') if plain(mods) => {
                self.switch_tab(Tab::Issues);
                return true;
            }
            Key::Char('p') if plain(mods) => {
                self.switch_tab(Tab::Pulls);
                return true;
            }
            Key::Char('a') if plain(mods) => {
                self.switch_tab(Tab::Actions);
                return true;
            }
            Key::Char(',') if plain(mods) => {
                self.switch_tab(Tab::Settings);
                return true;
            }
            Key::Char('r') if plain(mods) => {
                if is_pulls {
                    self.load_pulls();
                } else {
                    self.load_issues();
                }
                return true;
            }
            Key::Esc => {
                self.switch_tab(Tab::Code);
                return true;
            }
            Key::Enter => {
                if is_pulls {
                    self.open_pull_detail(sel);
                } else {
                    self.open_issue_detail(sel);
                }
                return true;
            }
            _ => {}
        }
        let new_sel = match key {
            Key::Up => sel.saturating_sub(1),
            Key::Down if count > 0 => (sel + 1).min(count - 1),
            Key::PageUp => sel.saturating_sub(page),
            Key::PageDown if count > 0 => (sel + page).min(count - 1),
            Key::Home => 0,
            Key::End => count.saturating_sub(1),
            _ => return false,
        };
        if let Some(rv) = self.rv.as_mut() {
            if is_pulls {
                rv.pulls_sel = new_sel;
            } else {
                rv.issues_sel = new_sel;
            }
        }
        true
    }
}
