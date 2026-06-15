//! Overlay shell (dim, entrance animation, input swallowing) plus the
//! simple overlays; pickers and code search live in their own modules.

use super::*;

impl View {
    pub(super) fn overlay(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        self.overlay_t.target = if app.overlay.is_some() { 1.0 } else { 0.0 };
        if app.overlay.is_none() {
            self.overlay_t.snap(0.0);
            // Reset picker scroll state so the next open re-anchors on the
            // current selection instead of inheriting a stale offset.
            self.scrolls.remove(&skey(Scroll::Overlay));
            self.last_sel.remove(&Z_OVER);
            return;
        }
        if self.overlay_t.tick_n(self.dt, 14.0) {
            self.active = true;
        }
        let k = ease_out(self.overlay_t.v);
        // Overlay swallows all main-screen input.
        self.clicks.clear();
        self.wheels.clear();
        self.editor_geom = None;
        self.agent_geom = None;

        dl.solid(RectF::new(0.0, 0.0, w, h), [0.0, 0.0, 0.0, 0.55 * k]);
        let pw = self.f(560.0).min(w - self.f(40.0));
        let lift = (1.0 - k) * self.f(16.0);

        let title_of = |o: &Overlay| match o {
            Overlay::Commit(_) => "COMMIT",
            Overlay::NewFile(_) => "NEW FILE",
            Overlay::NewBranch { .. } => "NEW BRANCH",
            Overlay::ModelPick { .. } => "SELECT MODEL",
            Overlay::OpenRepo(_) => "OPEN REPOSITORY",
            Overlay::BranchPick { .. } => "SWITCH BRANCH",
            Overlay::FileSearch { .. } => "FIND FILE",
            Overlay::CodeSearch { scope: SearchScope::Global, .. } => "CODE SEARCH · GLOBAL",
            Overlay::CodeSearch { .. } => "CODE SEARCH",
            Overlay::SettingsForm(_) => "SETTINGS",
            Overlay::Confirm { .. } => "CONFIRM",
            Overlay::AgentApproval { .. } => "APPROVE API WRITE",
            Overlay::YoloWarn => "ENABLE YOLO MODE",
            Overlay::Help => "KEYMAP",
        };
        let title = app.overlay.as_ref().map(title_of).unwrap_or("").to_string();


        match app.overlay.as_ref().unwrap() {
            Overlay::Commit(_) => self.ov_commit(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::NewFile(_) => self.ov_new_file(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::NewBranch { .. } => self.ov_new_branch(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::ModelPick { .. } => self.ov_model_pick(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::OpenRepo(_) => self.ov_open_repo(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::BranchPick { .. } => self.ov_branch_pick(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::FileSearch { .. } => self.ov_file_search(app, dl, atlas, w, h, lift, &title),
            Overlay::CodeSearch { .. } => self.ov_code_search(app, dl, atlas, w, h, lift, &title),
            Overlay::SettingsForm(_) => self.ov_settings_form(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::Confirm { .. } => self.ov_confirm(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::AgentApproval { .. } => self.ov_agent_approval(app, dl, atlas, w, h, pw, lift, &title),
            Overlay::YoloWarn => self.ov_yolo_warn(dl, atlas, w, h, pw, lift, &title),
            Overlay::Help => self.ov_help(dl, atlas, w, h, pw, lift, &title),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ov_new_file(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::NewFile(input)) = &app.overlay else { return };
        let input = input.clone_shallow();
        let ph = self.f(190.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, title);
        dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), r.y + self.f(64.0), "new file path (staged, then opened to edit):", DIM, self.f(1.5));
        let field = RectF::new(r.x + self.f(24.0), r.y + self.f(76.0), r.w - self.f(48.0), self.f(40.0));
        self.input_field(dl, atlas, &input, field, true);
        dl.text(atlas, MONO, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "e.g. src/new_mod.rs · [ENTER] CREATE · [ESC] ABORT", FAINT, 0.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn ov_new_branch(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::NewBranch { name, base }) = &app.overlay else { return };
        let name_in = name.clone_shallow();
        let base_idx = *base;
        let base_name = app
            .rv
            .as_ref()
            .and_then(|rv| rv.branches.ready())
            .and_then(|b| b.get(base_idx))
            .map(|b| b.name.clone())
            .unwrap_or_default();
        let ph = self.f(214.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, title);
        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        // Base-branch chip (↑/↓ or click to cycle).
        self.field_label(dl, atlas, lx, r.y + self.f(56.0), "BASE BRANCH  ↑/↓ or click");
        let blabel = format!("⎇ {}", base_name);
        let tw = dl.text_width(atlas, UI_BOLD, self.f(12.0), &blabel, self.f(1.0));
        let chip = RectF::new(lx, r.y + self.f(70.0), tw + self.f(24.0), self.f(28.0));
        let hv = self.hover_amt(wid(Z_MENU, 901), chip.contains(self.mouse.0, self.mouse.1));
        dl.rrect(chip, self.f(4.0), with_a(CYAN, 0.10 + 0.10 * hv), 1.0);
        dl.border(chip, self.f(4.0), 1.0, with_a(CYAN, 0.5));
        dl.text(atlas, UI_BOLD, self.f(12.0), chip.x + self.f(12.0), chip.y + self.f(19.0), &blabel, CYAN, self.f(1.0));
        self.clicks.push((chip, Click::CycleBranchBase));
        // New-branch name field.
        self.field_label(dl, atlas, lx, r.y + self.f(116.0), "NEW BRANCH NAME");
        let field = RectF::new(lx, r.y + self.f(130.0), fw, self.f(36.0));
        self.input_field(dl, atlas, &name_in, field, true);
        dl.text(atlas, UI, self.f(11.0), lx, r.y + ph - self.f(18.0), "[ENTER] CREATE · [↑↓] base · [ESC] abort", FAINT, self.f(1.5));
    }

    #[allow(clippy::too_many_arguments)]
    fn ov_open_repo(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::OpenRepo(input)) = &app.overlay else { return };
        let input = input.clone_shallow();
        let ph = self.f(190.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);
        dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), r.y + self.f(64.0), "owner/repo or organization:", DIM, self.f(1.5));
        let field = RectF::new(r.x + self.f(24.0), r.y + self.f(76.0), r.w - self.f(48.0), self.f(40.0));
        self.input_field(dl, atlas, &input, field, true);
        dl.text(atlas, UI, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "[ENTER] OPEN · [ESC] ABORT", FAINT, self.f(1.5));
    }

    #[allow(clippy::too_many_arguments)]
    fn ov_confirm(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::Confirm { msg, .. }) = &app.overlay else { return };
        let msg = msg.clone();
        let ph = self.f(150.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);
        let m = dl.fit(atlas, UI, self.f(14.5), &msg, r.w - self.f(48.0));
        dl.text(atlas, UI, self.f(14.5), r.x + self.f(24.0), r.y + self.f(72.0), &m, TEXT, 0.0);
        dl.text(atlas, UI, self.f(12.0), r.x + self.f(24.0), r.y + ph - self.f(24.0), "[ENTER/Y] CONFIRM · [ESC/N] ABORT", FAINT, self.f(1.5));
    }

    #[allow(clippy::too_many_arguments)]
    fn ov_help(&mut self, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let lines: [(&str, &str); 23] = [
            ("GLOBAL", ""),
            ("?", "this help · esc closes"),
            ("REPOSITORIES", ""),
            ("/ O R ENTER", "filter · open repo or org · reload · open"),
            ("S ⇧S F X", "cycle sort · flip order · toggle forks/archived"),
            ("G", "global code search across GitHub (needs token)"),
            ("CODE", ""),
            ("↑↓ ←→ ENTER", "navigate tree · expand/collapse · open"),
            ("/ G", "find file in tree · code search (needs token)"),
            ("TAB B A T P", "pane · branch · actions · issues · pulls"),
            ("E S N D U C", "edit·stage·new·stage-del·unstage·commit staged"),
            ("EDITOR", ""),
            ("CTRL+S", "stage + commit · ctrl+z undo · ctrl+y redo"),
            ("SHIFT+ARROWS", "select · esc back to view mode"),
            ("ISSUES · PULLS", ""),
            ("↑↓ ENTER R", "browse list · open detail · refresh"),
            ("DETAIL", ""),
            ("↑↓ ESC", "scroll body · back to list"),
            ("A M", "approve PR · merge PR (with method)"),
            ("ACTIONS", ""),
            ("ENTER R", "load jobs · refresh"),
            ("AGENT", ""),
            ("I", "AI agent window · drives the GitHub API for you"),
        ];
        let row = self.f(24.0);
        let ph = lines.len() as f32 * row + self.f(86.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);
        for (i, (k, v)) in lines.iter().enumerate() {
            let y = r.y + self.f(64.0) + i as f32 * row;
            if v.is_empty() {
                dl.text(atlas, UI_BOLD, self.f(12.0), r.x + self.f(24.0), y, k, CYAN, self.f(2.5));
            } else {
                dl.text(atlas, MONO, self.f(12.0), r.x + self.f(36.0), y, k, with_a(MAGENTA, 0.9), 0.0);
                dl.text(atlas, UI, self.f(13.0), r.x + self.f(190.0), y, v, with_a(TEXT, 0.8), 0.0);
            }
        }
    }
}
