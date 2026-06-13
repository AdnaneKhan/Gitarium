//! The Repos screen: toolbar, filter bar, and the streamed card list.

use super::*;

impl View {
    pub(super) fn repos_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let title = match &app.repo_source {
            RepoSource::Mine => "RUSTVM::GITHUB".to_string(),
            RepoSource::Org(n) => format!("ORG::{}", n.to_uppercase()),
        };
        let hh = self.header(app, dl, atlas, w, &title, None);
        let mut top = hh + self.f(6.0);

        // Toolbar: sort + visibility toggles (only once a list is loaded).
        if app.repos.ready().is_some() {
            let ty = top + self.f(4.0);
            let mut x = self.f(16.0);
            let (sort_label, dir, hide_forks, hide_archived) =
                (app.repo_sort.label(), app.sort_asc, app.hide_forks, app.hide_archived);
            x = self.tool_chip(dl, atlas, &format!("SORT: {}", sort_label), x, ty, CYAN, Click::SortCycle, wid(Z_CHIP, 20));
            x = self.tool_chip(dl, atlas, if dir { "↑" } else { "↓" }, x, ty, CYAN, Click::SortDir, wid(Z_CHIP, 21));
            x += self.f(8.0);
            let fc = if hide_forks { MAGENTA } else { with_a(TEXT, 0.55) };
            x = self.tool_chip(
                dl,
                atlas,
                if hide_forks { "FORKS: HIDDEN" } else { "FORKS: SHOWN" },
                x,
                ty,
                fc,
                Click::ToggleForks,
                wid(Z_CHIP, 22),
            );
            let ac = if hide_archived { MAGENTA } else { with_a(TEXT, 0.55) };
            x = self.tool_chip(
                dl,
                atlas,
                if hide_archived { "ARCHIVED: HIDDEN" } else { "ARCHIVED: SHOWN" },
                x,
                ty,
                ac,
                Click::ToggleArchived,
                wid(Z_CHIP, 23),
            );
            let _ = x;
            let shown = app.filtered_repos().len();
            let total = app.repos.ready().map(|r| r.len()).unwrap_or(0);
            // Trailing ellipsis while further pages are still streaming in.
            let count = if app.repos_loading_more {
                format!("{}/{}…", shown, total)
            } else {
                format!("{}/{}", shown, total)
            };
            let cw = dl.text_width(atlas, MONO, self.f(12.0), &count, 0.0);
            dl.text(atlas, MONO, self.f(12.0), w - cw - self.f(20.0), ty + self.f(16.0), &count, DIM, 0.0);
            top += self.f(34.0);
        }

        if app.filter_active || !app.filter.text.is_empty() {
            let bar = RectF::new(self.f(16.0), top, w - self.f(32.0), self.f(36.0));
            dl.text(atlas, UI_BOLD, self.f(16.0), bar.x + self.f(2.0), bar.y + self.f(24.0), "/", CYAN, 0.0);
            let field = RectF::new(bar.x + self.f(18.0), bar.y, bar.w - self.f(18.0), bar.h);
            let input = app.filter.clone_shallow();
            self.input_field(dl, atlas, &input, field, app.filter_active);
            top += self.f(44.0);
        }

        let list = RectF::new(self.f(16.0), top + yoff, w - self.f(32.0), h - top - self.f(34.0) - yoff);

        match &app.repos {
            Loadable::Loading => {
                self.sweep_note(dl, atlas, list.x + self.f(8.0), list.y + self.f(24.0), list.w, "SCANNING REPOSITORIES…")
            }
            Loadable::Failed(e) => {
                let msg = dl.fit(atlas, UI, self.f(14.0), e, list.w - self.f(16.0));
                dl.text(atlas, UI, self.f(14.0), list.x + self.f(8.0), list.y + self.f(24.0), &msg, RED, 0.0);
            }
            Loadable::Idle => {
                dl.text(
                    atlas,
                    UI,
                    self.f(15.0),
                    list.x + self.f(8.0),
                    list.y + self.f(28.0),
                    "ANONYMOUS MODE — NO REPOSITORY LIST",
                    DIM,
                    self.f(2.0),
                );
                dl.text(
                    atlas,
                    UI_BOLD,
                    self.f(15.0),
                    list.x + self.f(8.0),
                    list.y + self.f(54.0),
                    "[O] OPEN owner/repo OR AN ORGANIZATION",
                    CYAN,
                    self.f(1.5),
                );
            }
            Loadable::Ready(_) => {
                let filtered = app.filtered_repos();
                if filtered.is_empty() {
                    dl.text(atlas, UI, self.f(14.0), list.x + self.f(8.0), list.y + self.f(24.0), "no matches", DIM, 0.0);
                }
                if app.repo_sel >= filtered.len() && !filtered.is_empty() {
                    app.repo_sel = filtered.len() - 1;
                }
                // One full-width card per row.
                let gap = self.f(10.0);
                let card_h = self.f(88.0);
                let row_h = card_h + gap;
                app.layout.repos_cols = 1;
                app.layout.repos_h = ((list.h / row_h) as usize).max(1);
                let offset = self.list_scroll(Scroll::Repos, Z_REPO, app.repo_sel, filtered.len(), row_h, list.h);
                let repos = app.repos.ready().unwrap();
                dl.push_clip(list);
                let first = (offset / row_h) as usize;
                for vis in 0..(list.h / row_h) as usize + 2 {
                    let fi = first + vis;
                    if fi >= filtered.len() {
                        break;
                    }
                    let repo = &repos[filtered[fi]];
                    let card = RectF::new(list.x, list.y + fi as f32 * row_h - offset, list.w - self.f(8.0), card_h);
                    let selected = fi == app.repo_sel;
                    self.repo_card(dl, atlas, repo, card, selected, fi);
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, filtered.len() as f32 * row_h, offset);
                self.wheels.push((list, Scroll::Repos, row_h, (filtered.len() as f32 * row_h - list.h).max(0.0)));
            }
        }
    }
}
