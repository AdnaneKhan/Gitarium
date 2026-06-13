//! Top-level event dispatch and the Repos screen keys. Every handler
//! reports whether it consumed the event (the host uses this for
//! preventDefault — unconsumed keys keep their browser behavior). Auth-screen
//! handling lives in `auth.rs`.

use crate::ui::input::{Event, Key, Mods};

use super::{App, LineInput, Loadable, Overlay, RepoSource, Route, SearchScope};

/// A char binding fires only without Ctrl/Alt (Cmd arrives as ctrl from the
/// host) so browser shortcuts are never shadowed by single-key bindings.
pub(super) fn plain(mods: Mods) -> bool {
    !mods.ctrl && !mods.alt
}

impl App {
    /// Returns true when the event was consumed.
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

    pub(super) fn repos_key(&mut self, key: Key, mods: Mods) -> bool {
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
            Key::Char('g') if plain(mods) => {
                // Global code search across GitHub. Requires auth, like the
                // repo-scoped one; opening a hit fetches that repo + file.
                if self.token.is_none() {
                    self.toast = Some(("code search requires an access token".into(), true));
                } else {
                    self.overlay = Some(Overlay::CodeSearch {
                        input: LineInput::new(false),
                        sel: 0,
                        searched: String::new(),
                        results: Loadable::Idle,
                        scope: SearchScope::Global,
                        page: 0,
                        more: false,
                        loading_more: false,
                    });
                }
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
}
