//! The Repos screen's data: the streamed paginated listing, filtering, and
//! sorting. Pages populate the view as they land — no waiting for the full
//! set before the user can browse.

use crate::github;

use super::{App, Loadable, Msg, RepoSort, RepoSource};

impl App {
    pub(super) fn load_repos(&mut self) {
        self.repos_gen += 1;
        self.repos_loading_more = false;
        let gen = self.repos_gen;
        let token = self.token.clone();
        let owner = match self.repo_source.clone() {
            RepoSource::Mine => {
                if token.is_none() {
                    self.repos = Loadable::Idle;
                    return;
                }
                None
            }
            RepoSource::Org(name) => Some(name),
        };
        self.repos = Loadable::Loading;
        crate::spawn_msg(async move {
            match github::repos_first_page(&token, owner.as_deref()).await {
                Ok((base, page)) => Msg::ReposPage { gen, base, page: 1, result: Ok(page) },
                Err(e) => Msg::ReposPage { gen, base: String::new(), page: 1, result: Err(e) },
            }
        });
    }

    fn load_repos_next_page(&mut self, base: String, page: usize) {
        let gen = self.repos_gen;
        let token = self.token.clone();
        crate::spawn_msg(async move {
            let result = github::repos_page(&token, &base, page).await;
            Msg::ReposPage { gen, base, page, result }
        });
    }

    pub(super) fn on_repos_page(
        &mut self,
        gen: u64,
        base: String,
        page: usize,
        result: Result<github::RepoPage, String>,
    ) {
        if gen != self.repos_gen {
            return; // a refresh or source switch superseded this chain
        }
        match result {
            Ok(p) => {
                let last = p.last;
                if page == 1 {
                    self.repos = Loadable::Ready(p.repos);
                    self.repo_sel = 0;
                    self.repo_scroll = 0;
                } else if let Loadable::Ready(repos) = &mut self.repos {
                    repos.extend(p.repos);
                } else {
                    return;
                }
                if last {
                    self.repos_loading_more = false;
                } else if page >= github::MAX_PAGES {
                    self.repos_loading_more = false;
                    self.toast = Some(("repo list truncated (10,000 repo cap)".into(), true));
                } else {
                    self.repos_loading_more = true;
                    self.load_repos_next_page(base, page + 1);
                }
            }
            Err(e) => {
                if page == 1 {
                    self.repos = Loadable::Failed(e);
                } else {
                    // Keep what already streamed in, but the gap must stay
                    // visible, not look like the full list.
                    self.repos_loading_more = false;
                    self.toast = Some((format!("repo list incomplete: {}", e), true));
                }
            }
        }
    }

    /// Browse another org's (or user's) repositories on the Repos screen.
    pub(super) fn open_org(&mut self, name: String) {
        self.repo_source = RepoSource::Org(name);
        self.repo_sel = 0;
        self.repo_scroll = 0;
        self.filter.clear();
        self.filter_active = false;
        self.route = super::Route::Repos;
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

    pub(super) fn cycle_sort(&mut self) {
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
}
