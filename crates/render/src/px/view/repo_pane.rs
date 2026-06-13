//! The Repo route: header/tab dispatch and the Code tab's layout
//! (tree pane extracted, editor body in editor_pane).

use super::*;

impl View {
    pub(super) fn repo_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let (title, branch, tab) = match &app.rv {
            Some(rv) => (rv.repo.full_name.clone(), rv.branch.clone(), rv.tab),
            None => return,
        };
        let hh = self.header(app, dl, atlas, w, &title, Some(branch));
        let top = hh + self.f(10.0) + yoff;
        let bottom = h - self.f(34.0);
        match tab {
            Tab::Code => self.code_tab(app, dl, atlas, w, top, bottom),
            Tab::Actions => self.actions_tab(app, dl, atlas, w, top, bottom),
        }
    }

    fn code_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let tree = RectF::new(self.f(16.0), top, self.f(300.0).min(w * 0.3), bottom - top);
        let content = RectF::new(tree.right() + self.f(12.0), top, w - tree.right() - self.f(28.0), bottom - top);
        self.panel(dl, tree);
        self.panel(dl, content);
        // Remembered for the tree's right-click context menu (empty-space hits).
        self.tree_rect = Some(tree);

        let row_h = self.f(27.0);
        // A header strip holds the "+ FILE" add affordance; rows start below.
        let header_h = self.f(26.0);
        self.chip(dl, atlas, "+ FILE", tree.right() - self.f(10.0), tree.y + header_h / 2.0 + self.f(3.0), GREEN, Click::NewFileBtn, wid(Z_CHIP, 7));
        let inner = RectF::new(tree.x + self.f(8.0), tree.y + header_h, tree.w - self.f(16.0), tree.h - header_h - self.f(8.0));
        app.layout.tree_h = (inner.h / row_h).max(1.0) as usize;

        self.tree_pane(app, dl, atlas, tree, inner, row_h);

        // ---- content
        let loading_path = app.rv.as_ref().unwrap().file_loading.clone();
        if let Some(p) = loading_path {
            self.sweep_note(dl, atlas, content.x + self.f(16.0), content.y + self.f(30.0), content.w - self.f(32.0), &format!("LOADING {}", p.to_uppercase()));
            return;
        }
        if app.rv.as_ref().unwrap().file.is_none() {
            let rv = app.rv.as_ref().unwrap();
            let x = content.x + self.f(24.0);
            dl.text(atlas, UI_BOLD, self.f(26.0), x, content.y + self.f(52.0), &rv.repo.full_name, with_a(CYAN, 0.35), self.f(3.0));
            let mut y = content.y + self.f(84.0);
            if let Some(d) = rv.repo.description.clone() {
                let msg = dl.fit(atlas, UI, self.f(14.0), &d, content.w - self.f(48.0));
                dl.text(atlas, UI, self.f(14.0), x, y, &msg, DIM, 0.0);
                y += self.f(30.0);
            }
            for line in [
                "↑↓ NAVIGATE · ENTER OPEN · TAB SWITCH PANE",
                "E EDIT · S STAGE · N NEW · D DELETE · C COMMIT",
                "RIGHT-CLICK TREE FOR ACTIONS · B BRANCH · ? KEYMAP",
            ] {
                dl.text(atlas, UI, self.f(12.5), x, y, line, FAINT, self.f(1.5));
                y += self.f(22.0);
            }
            return;
        }

        // Path bar.
        let bar_h = self.f(34.0);
        let bar = RectF::new(content.x + 1.0, content.y + 1.0, content.w - 2.0, bar_h);
        dl.solid(RectF::new(bar.x, bar.bottom(), bar.w, 1.0), BORDER);
        {
            let rv = app.rv.as_ref().unwrap();
            let file = rv.file.as_ref().unwrap();
            let mut x = content.x + self.f(14.0);
            let path = dl.fit(atlas, MONO, self.f(12.5), &file.path, content.w * 0.5);
            x = dl.text(atlas, MONO, self.f(12.5), x, bar.y + self.f(22.0), &path, with_a(TEXT, 0.8), 0.0);
            if file.editor.modified {
                let dot = RectF::new(x + self.f(8.0), bar.y + self.f(14.0), self.f(7.0), self.f(7.0));
                dl.glow(dot, self.f(3.5), with_a(YELLOW, 0.4), self.f(6.0));
                dl.rrect(dot, self.f(3.5), YELLOW, 1.0);
                x = dot.right();
            }
            if file.editing {
                x += self.f(12.0);
                let tag = "EDIT";
                let tw = dl.text_width(atlas, UI_BOLD, self.f(11.0), tag, self.f(2.0));
                let tr = RectF::new(x, bar.y + self.f(8.0), tw + self.f(12.0), self.f(18.0));
                dl.glow(tr, self.f(2.0), with_a(MAGENTA, 0.25), self.f(7.0));
                dl.border(tr, self.f(2.0), 1.0, MAGENTA);
                dl.text(atlas, UI_BOLD, self.f(11.0), tr.x + self.f(6.0), bar.y + self.f(21.5), tag, MAGENTA, self.f(2.0));
            }
            let _ = x;
        }
        // Action chips (need &mut self, so read flags first).
        let rv = app.rv.as_ref().unwrap();
        let (binary, modified, editing) = {
            let f = rv.file.as_ref().unwrap();
            (f.binary, f.editor.modified, f.editing)
        };
        let staged = rv.staged.len();
        let committing = rv.committing;
        let can_edit = app.can_edit_repo();
        if committing {
            self.active = true;
        }
        let mut right = content.right() - self.f(12.0);
        // COMMIT (N) opens the staged-commit overlay; STAGE captures the
        // current buffer into the workspace.
        if committing {
            right = self.chip(dl, atlas, "COMMITTING…", right, bar.y + bar_h / 2.0, YELLOW, Click::CommitBtn, wid(Z_CHIP, 1));
        } else if staged > 0 {
            let label = format!("COMMIT ({})", staged);
            right = self.chip(dl, atlas, &label, right, bar.y + bar_h / 2.0, GREEN, Click::CommitBtn, wid(Z_CHIP, 1));
        }
        if !binary && modified && !committing {
            right = self.chip(dl, atlas, "STAGE", right, bar.y + bar_h / 2.0, MAGENTA, Click::StageBtn, wid(Z_CHIP, 3));
        }
        // Edit needs write access; anonymous and read-only viewers don't see it.
        if !binary && !editing && can_edit {
            self.chip(dl, atlas, "EDIT", right, bar.y + bar_h / 2.0, CYAN, Click::EditBtn, wid(Z_CHIP, 2));
        }

        let body = RectF::new(
            content.x + self.f(6.0),
            bar.bottom() + self.f(6.0),
            content.w - self.f(12.0),
            content.bottom() - bar.bottom() - self.f(12.0),
        );
        if binary {
            let f = app.rv.as_ref().unwrap().file.as_ref().unwrap();
            dl.text(atlas, UI, self.f(14.0), body.x + self.f(12.0), body.y + self.f(28.0), &format!("BINARY FILE · {} BYTES", f.size), DIM, self.f(1.5));
            return;
        }
        self.editor_body(app, dl, atlas, body);
    }
}
