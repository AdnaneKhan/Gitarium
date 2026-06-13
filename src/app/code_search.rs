//! The GitHub code-search palette — repo-scoped (from a repo) or global
//! (from the Repos screen). Opening a repo-scoped hit loads the file in the
//! current repo; a global hit fetches its repo first, then jumps to the file.

use crate::github;
use crate::ui::input::{Key, Mods};

use super::{App, ConfirmAction, Loadable, Msg, Overlay, SearchScope};

impl App {
    pub(super) fn code_search_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::CodeSearch {
            input,
            sel,
            searched,
            results,
            scope,
            page,
            more,
            loading_more,
        }) = &mut self.overlay
        else {
            return false;
        };
        let scope = *scope;
        match key {
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
                if count == 0 {
                } else if *sel + 1 < count {
                    *sel += 1;
                } else if *more && !*loading_more {
                    // At the bottom of the loaded hits: pull the next page and
                    // append it (selection stays; new rows extend below).
                    *loading_more = true;
                    let next = *page + 1;
                    let query = searched.clone();
                    self.spawn_code_search(query, scope, next);
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
                    *page = 0;
                    *more = false;
                    *loading_more = false;
                    // Bump the generation so any page still in flight from the
                    // previous query can't append to this fresh one.
                    self.code_search_gen += 1;
                    self.spawn_code_search(q, scope, 1);
                } else if let Loadable::Ready(hits) = results {
                    let hit = hits.get(*sel).map(|h| (h.repo.clone(), h.path.clone()));
                    if let Some((repo, path)) = hit {
                        self.overlay = None;
                        match scope {
                            SearchScope::Repo => self.open_path_in_repo(path),
                            SearchScope::Global => self.open_global_hit(repo, path),
                        }
                    }
                }
                true
            }
            k => input.handle_key(&k, mods),
        }
    }

    /// Fire a code-search request for `page` (1-based) at the current
    /// generation; the result lands in `on_code_search_done`, which replaces
    /// (page 1) or appends (later pages). The caller bumps `code_search_gen`
    /// for a fresh query and resets `loading_more`/`more` for a load-more.
    fn spawn_code_search(&mut self, query: String, scope: SearchScope, page: u32) {
        // repo: qualifier for the scoped overlay; the global one searches
        // everywhere (the user supplies any qualifiers).
        let scope_q = match scope {
            SearchScope::Repo => self.rv.as_ref().map(|rv| format!("repo:{}", rv.repo.full_name)),
            SearchScope::Global => None,
        };
        let gen = self.code_search_gen;
        let token = self.token.clone();
        crate::spawn_msg(async move {
            let result = github::search_code(&token, &query, scope_q.as_deref(), page).await;
            Msg::CodeSearchDone { gen, page, result }
        });
    }

    /// Open a path in the currently-open repo (repo-scoped hit), confirming
    /// first if the open file has unsaved edits.
    fn open_path_in_repo(&mut self, path: String) {
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

    /// Open a global hit: fetch its repo, then jump to the file once the
    /// repo's branches arrive (see `open_repo_then` / `on_branches`).
    fn open_global_hit(&mut self, repo: String, path: String) {
        if repo.is_empty() {
            self.toast = Some(("result is missing its repository".into(), true));
            return;
        }
        self.toast = Some((format!("opening {}…", repo), false));
        self.opening_repo = Some(repo.clone());
        let token = self.token.clone();
        crate::spawn_msg(async move {
            let result = github::get_repo(&token, &repo).await;
            Msg::RepoOpened { name: repo, result, then_open: Some(path) }
        });
    }

    pub(super) fn on_code_search_done(
        &mut self,
        gen: u64,
        page: u32,
        result: Result<(Vec<github::CodeHit>, u64), String>,
    ) {
        // Stale results (overlay reopened, query reissued, navigated away)
        // carry an older gen and are dropped.
        if gen != self.code_search_gen {
            return;
        }
        let Some(Overlay::CodeSearch { results, sel, page: cur, more, loading_more, .. }) =
            &mut self.overlay
        else {
            return;
        };
        *loading_more = false;
        match result {
            Ok((mut hits, total)) => {
                if page <= 1 {
                    *results = Loadable::Ready(hits);
                    *sel = 0;
                } else if let Loadable::Ready(existing) = results {
                    existing.append(&mut hits);
                } else {
                    // Page 1 never landed (failed or superseded) — treat this
                    // as the first page rather than dropping it.
                    *results = Loadable::Ready(hits);
                    *sel = 0;
                }
                *cur = page.max(1);
                let len = results.ready().map(|h| h.len()).unwrap_or(0) as u64;
                // GitHub's Search API serves at most 1000 results (10 pages).
                *more = len < total.min(1000) && *cur < 10;
            }
            Err(e) => {
                if page <= 1 {
                    *results = Loadable::Failed(e);
                    *sel = 0;
                    *more = false;
                } else {
                    // Keep the pages already loaded; surface the error and
                    // leave `more` set so the user can retry from the bottom.
                    self.toast = Some((e, true));
                }
            }
        }
    }
}
