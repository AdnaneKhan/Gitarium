//! Per-section content for the Settings tab. General is a metadata view with an
//! edit chip + danger zone; the list sections (Secrets / Variables / DeployKeys
//! / Collaborators / Webhooks) share a selectable-row renderer.

use super::*;

impl View {
    /// Section title + Add/Edit/Delete chips. Edit/Delete appear only when a
    /// row is selected; all three only when the viewer can edit.
    fn settings_header(&mut self, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF, title: &str, can_edit: bool, has_sel: bool) {
        dl.text(atlas, UI_BOLD, self.f(18.0), rect.x + self.f(20.0), rect.y + self.f(30.0), title, TEXT, self.f(2.0));
        if !can_edit {
            return;
        }
        let mut right = rect.right() - self.f(16.0);
        let cy = rect.y + self.f(26.0);
        if has_sel {
            right = self.chip(dl, atlas, "Delete", right, cy, RED, Click::SettingsDelete, wid(Z_SET, 900));
            right = self.chip(dl, atlas, "Edit", right, cy, CYAN, Click::SettingsEdit, wid(Z_SET, 901));
        }
        self.chip(dl, atlas, "+ Add", right, cy, GREEN, Click::SettingsAdd, wid(Z_SET, 902));
    }

    /// A list of selectable rows, or a loading/error/empty note. `rows` is
    /// (primary, secondary); `note` overrides the list when set.
    fn settings_list_body(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF, rows: &[(String, String)], note: Option<(&str, bool)>) {
        let body = RectF::new(rect.x + self.f(8.0), rect.y + self.f(56.0), rect.w - self.f(16.0), rect.h - self.f(64.0));
        if let Some((note, err)) = note {
            if err {
                let m = dl.fit(atlas, UI, self.f(13.0), note, body.w);
                dl.text(atlas, UI, self.f(13.0), body.x + self.f(8.0), body.y + self.f(20.0), &m, RED, 0.0);
            } else {
                self.sweep_note(dl, atlas, body.x + self.f(8.0), body.y + self.f(20.0), body.w, note);
            }
            return;
        }
        let row_h = self.f(28.0);
        let sel = app.rv.as_ref().map(|rv| rv.settings.list_sel).unwrap_or(0);
        for (i, (prim, sec)) in rows.iter().enumerate() {
            let y = body.y + i as f32 * row_h;
            let rr = RectF::new(body.x, y, body.w, row_h - 2.0);
            let on = i == sel;
            let hv = self.hover_amt(wid(Z_SET, 100 + i), rr.contains(self.hot.0, self.hot.1));
            let a = if on { 0.12 } else { 0.05 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            let prim_fit = dl.fit(atlas, UI_BOLD, self.f(13.0), prim, body.w * 0.55);
            dl.text(atlas, UI_BOLD, self.f(13.0), rr.x + self.f(10.0), y + self.f(18.0), &prim_fit, TEXT, 0.0);
            let sec_fit = dl.fit(atlas, UI, self.f(11.0), sec, body.w * 0.4);
            let sw = dl.text_width(atlas, UI, self.f(11.0), &sec_fit, 0.0);
            dl.text(atlas, UI, self.f(11.0), rr.right() - self.f(10.0) - sw, y + self.f(18.0), &sec_fit, DIM, 0.0);
            self.clicks.push((rr, Click::SettingsRow(i)));
        }
    }

    pub(super) fn render_settings_general(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let admin = app.is_admin();
        dl.text(atlas, UI_BOLD, self.f(18.0), rect.x + self.f(20.0), rect.y + self.f(30.0), "General", TEXT, self.f(2.0));
        if admin {
            self.chip(dl, atlas, "Edit", rect.right() - self.f(16.0), rect.y + self.f(26.0), CYAN, Click::SettingsEdit, wid(Z_SET, 900));
        }
        let Some(rv) = app.rv.as_ref() else { return };
        let x = rect.x + self.f(24.0);
        let mut y = rect.y + self.f(74.0);
        let pairs: [(&str, String); 6] = [
            ("Name", rv.repo.name.clone()),
            ("Visibility", if rv.repo.private { "private".into() } else { "public".into() }),
            ("Default branch", rv.repo.default_branch.clone()),
            ("Description", rv.repo.description.clone().unwrap_or_else(|| "—".into())),
            ("Archived", if rv.repo.archived { "yes".into() } else { "no".into() }),
            ("Stars", rv.repo.stargazers_count.to_string()),
        ];
        for (k, v) in pairs {
            let kw = dl.text_width(atlas, UI_BOLD, self.f(13.0), k, 0.0);
            dl.text(atlas, UI_BOLD, self.f(13.0), x, y, k, DIM, 0.0);
            let vv = dl.fit(atlas, UI, self.f(13.0), &v, rect.right() - x - kw - self.f(24.0));
            dl.text(atlas, UI, self.f(13.0), x + kw + self.f(12.0), y, &vv, TEXT, 0.0);
            y += self.f(26.0);
        }
        // Danger zone (admin only): archive + permanent delete.
        if admin {
            y += self.f(18.0);
            dl.text(atlas, UI_BOLD, self.f(13.0), x, y, "DANGER ZONE", RED, self.f(2.0));
            y += self.f(8.0);
            let mut right = rect.right() - self.f(16.0);
            let cy = y + self.f(16.0);
            right = self.chip(dl, atlas, "Delete repository", right, cy, RED, Click::SettingsDeleteRepo, wid(Z_SET, 910));
            let label = if rv.repo.archived { "Un-archive" } else { "Archive" };
            self.chip(dl, atlas, label, right, cy, YELLOW, Click::SettingsArchiveRepo, wid(Z_SET, 911));
        }
    }

    pub(super) fn render_settings_secrets(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let can_edit = app.can_edit_repo();
        let (rows, note) = secrets_rows(app);
        self.settings_header(dl, atlas, rect, "Actions secrets", can_edit, note.is_none() && !rows.is_empty());
        self.settings_list_body(app, dl, atlas, rect, &rows, note);
    }

    pub(super) fn render_settings_variables(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let can_edit = app.can_edit_repo();
        let (rows, note) = variables_rows(app);
        self.settings_header(dl, atlas, rect, "Actions variables", can_edit, note.is_none() && !rows.is_empty());
        self.settings_list_body(app, dl, atlas, rect, &rows, note);
    }

    pub(super) fn render_settings_deploy_keys(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let can_edit = app.is_admin();
        let (rows, note) = deploy_keys_rows(app);
        self.settings_header(dl, atlas, rect, "Deploy keys", can_edit, note.is_none() && !rows.is_empty());
        self.settings_list_body(app, dl, atlas, rect, &rows, note);
    }

    pub(super) fn render_settings_collaborators(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let can_edit = app.is_admin();
        let (rows, note) = collaborators_rows(app);
        self.settings_header(dl, atlas, rect, "Collaborators", can_edit, note.is_none() && !rows.is_empty());
        self.settings_list_body(app, dl, atlas, rect, &rows, note);
    }

    pub(super) fn render_settings_webhooks(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rect: &RectF) {
        let can_edit = app.is_admin();
        let (rows, note) = webhooks_rows(app);
        self.settings_header(dl, atlas, rect, "Webhooks", can_edit, note.is_none() && !rows.is_empty());
        self.settings_list_body(app, dl, atlas, rect, &rows, note);
    }
}

fn secrets_rows(app: &App) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    list_state(app.rv.as_ref().map(|rv| &rv.settings.secrets), |s| (s.name.clone(), crate::app::fmt_age(&s.updated_at)))
}

fn variables_rows(app: &App) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    list_state(app.rv.as_ref().map(|rv| &rv.settings.variables), |v| (v.name.clone(), v.value.clone()))
}

fn deploy_keys_rows(app: &App) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    list_state(app.rv.as_ref().map(|rv| &rv.settings.deploy_keys), |k| {
        let title = k.title.clone().unwrap_or_else(|| format!("#{}", k.id));
        let access = if k.read_only { "read-only" } else { "read/write" };
        (title, access.into())
    })
}

fn collaborators_rows(app: &App) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    list_state(app.rv.as_ref().map(|rv| &rv.settings.collaborators), |c| (c.login.clone(), c.role().to_string()))
}

fn webhooks_rows(app: &App) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    list_state(app.rv.as_ref().map(|rv| &rv.settings.webhooks), |h| {
        let url = h.config.url.clone().unwrap_or_else(|| format!("#{}", h.id));
        let evs = if h.events.is_empty() {
            "no events".to_string()
        } else if h.events.len() <= 3 {
            h.events.join(", ")
        } else {
            format!("{} events", h.events.len())
        };
        (url, evs)
    })
}

/// Map a section's `Loadable` to (rows, note). Ready→rows; Loading→"fetching";
/// Failed→error; empty→"none yet".
fn list_state<T, F: Fn(&T) -> (String, String)>(
    slot: Option<&Loadable<Vec<T>>>,
    map: F,
) -> (Vec<(String, String)>, Option<(&'static str, bool)>) {
    match slot {
        None => (vec![], Some(("no repository", true))),
        Some(Loadable::Idle) | Some(Loadable::Loading) => (vec![], Some(("fetching…", false))),
        Some(Loadable::Failed(_)) => (vec![], Some(("failed to load (press r to retry)", true))),
        Some(Loadable::Ready(v)) if v.is_empty() => (vec![], Some(("none yet", false))),
        Some(Loadable::Ready(v)) => (v.iter().map(&map).collect(), None),
    }
}
