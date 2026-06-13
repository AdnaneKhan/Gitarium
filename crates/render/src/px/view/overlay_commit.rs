//! The staged-commit overlay: a list of the staged changes plus the commit
//! message and the optional author / committer / date override fields. Tab
//! (or ↑↓) moves the focus ring between fields; Enter commits them all.

use super::*;

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_commit(
        &mut self,
        app: &mut App,
        dl: &mut DrawList,
        atlas: &mut Atlas,
        w: f32,
        h: f32,
        pw: f32,
        lift: f32,
        title: &str,
    ) {
        let Some(Overlay::Commit(form)) = &app.overlay else { return };
        let field = form.field;
        let message = form.message.clone_shallow();
        let an = form.author_name.clone_shallow();
        let ae = form.author_email.clone_shallow();
        let cn = form.committer_name.clone_shallow();
        let ce = form.committer_email.clone_shallow();
        let date = form.date.clone_shallow();
        let new_ref = form.new_ref.clone_shallow();
        let target = form.target;
        let branch = app.rv.as_ref().map(|rv| rv.branch.clone()).unwrap_or_default();
        let staged: Vec<(String, char)> = app
            .rv
            .as_ref()
            .map(|rv| {
                rv.staged
                    .iter()
                    .map(|(p, s)| (p.clone(), if matches!(s, Staged::Delete) { '-' } else { '+' }))
                    .collect()
            })
            .unwrap_or_default();

        let n = staged.len();
        let list_rows = n.min(5);
        let ph = self.f(404.0) + list_rows as f32 * self.f(18.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(
            dl,
            atlas,
            r,
            &format!("{} · {} CHANGE(S) → {}", title, n, branch.to_uppercase()),
        );
        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        let mut y = r.y + self.f(54.0);

        // Staged-changes list (first few; + add/edit, - delete).
        for (p, mark) in staged.iter().take(list_rows) {
            let col = if *mark == '-' { RED } else { GREEN };
            let line = dl.fit(atlas, MONO, self.f(11.5), &format!("{} {}", mark, p), fw);
            dl.text(atlas, MONO, self.f(11.5), lx, y, &line, with_a(col, 0.9), 0.0);
            y += self.f(18.0);
        }
        if n > list_rows {
            dl.text(atlas, UI, self.f(11.0), lx, y, &format!("…and {} more", n - list_rows), FAINT, 0.0);
            y += self.f(18.0);
        }
        y += self.f(8.0);

        // Commit message (field 0).
        self.field_label(dl, atlas, lx, y, "MESSAGE");
        y += self.f(14.0);
        self.input_field(dl, atlas, &message, RectF::new(lx, y, fw, self.f(34.0)), field == 0);
        y += self.f(46.0);

        // Author / committer override columns (name then email).
        let half = (fw - self.f(12.0)) / 2.0;
        let rx = lx + half + self.f(12.0);
        self.field_label(dl, atlas, lx, y, "AUTHOR  name · email");
        self.field_label(dl, atlas, rx, y, "COMMITTER  (blank = author)");
        y += self.f(14.0);
        self.input_field(dl, atlas, &an, RectF::new(lx, y, half, self.f(30.0)), field == 1);
        self.input_field(dl, atlas, &cn, RectF::new(rx, y, half, self.f(30.0)), field == 3);
        y += self.f(34.0);
        self.input_field(dl, atlas, &ae, RectF::new(lx, y, half, self.f(30.0)), field == 2);
        self.input_field(dl, atlas, &ce, RectF::new(rx, y, half, self.f(30.0)), field == 4);
        y += self.f(42.0);

        // Date override.
        self.field_label(dl, atlas, lx, y, "DATE  ISO 8601 · blank = now");
        y += self.f(14.0);
        self.input_field(dl, atlas, &date, RectF::new(lx, y, fw, self.f(30.0)), field == 5);
        y += self.f(40.0);

        // Destination chip (field 6): cycle current branch → new branch → tag.
        self.field_label(dl, atlas, lx, y, "TARGET  click or ←/→ to change");
        y += self.f(14.0);
        let (tlabel, tcol) = match target {
            CommitTarget::Current => (format!("→ {}", branch), CYAN),
            CommitTarget::NewBranch => ("＋ new branch".to_string(), GREEN),
            CommitTarget::NewTag => ("＋ new tag".to_string(), YELLOW),
        };
        let tw = dl.text_width(atlas, UI_BOLD, self.f(12.0), &tlabel, self.f(1.0));
        let chip = RectF::new(lx, y, tw + self.f(24.0), self.f(28.0));
        let hv = self.hover_amt(wid(Z_MENU, 900), chip.contains(self.mouse.0, self.mouse.1));
        let foc = field == CommitForm::TARGET_FIELD;
        dl.rrect(chip, self.f(4.0), with_a(tcol, 0.10 + 0.10 * hv), 1.0);
        dl.border(chip, self.f(4.0), if foc { 2.0 } else { 1.0 }, with_a(tcol, if foc { 0.9 } else { 0.45 }));
        dl.text(atlas, UI_BOLD, self.f(12.0), chip.x + self.f(12.0), chip.y + self.f(19.0), &tlabel, tcol, self.f(1.0));
        self.clicks.push((chip, Click::CommitCycleTarget));
        // Name input for a new branch / tag (field 7).
        if target != CommitTarget::Current {
            let kind = if target == CommitTarget::NewTag { "tag" } else { "branch" };
            self.field_label(dl, atlas, chip.right() + self.f(14.0), y - self.f(2.0), &format!("{} name:", kind));
            let nx = chip.right() + self.f(14.0);
            self.input_field(dl, atlas, &new_ref, RectF::new(nx, y + self.f(10.0), r.right() - nx - self.f(24.0), self.f(28.0)), field == CommitForm::REF_FIELD);
        }

        dl.text(
            atlas,
            UI,
            self.f(11.0),
            lx,
            r.y + ph - self.f(18.0),
            "[ENTER] COMMIT · [TAB / ↑↓] field · [ESC] abort",
            FAINT,
            self.f(1.5),
        );
    }

    pub(super) fn field_label(&mut self, dl: &mut DrawList, atlas: &mut Atlas, x: f32, y: f32, s: &str) {
        dl.text(atlas, UI, self.f(10.5), x, y, s, DIM, self.f(1.5));
    }
}
