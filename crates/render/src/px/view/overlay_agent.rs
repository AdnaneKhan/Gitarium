//! The interactive agent's safety modals: the per-turn approval prompt for
//! mutating API calls, and the one-time risk warning shown before YOLO
//! (auto-approve) mode is enabled. Both are danger-tinted (RED) so they read
//! as distinct from the routine cyan/green overlays.

use super::*;

impl View {
    /// Approval gate: lists the write call(s) the agent wants to run; Enter
    /// approves and dispatches them, Esc denies (the app answers the model
    /// with a refusal).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_agent_approval(
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
        let Some(Overlay::AgentApproval { summary, .. }) = &app.overlay else { return };
        let lines: Vec<&str> = summary.lines().collect();
        let row = self.f(20.0);
        let ph = self.f(150.0) + lines.len().max(1) as f32 * row;
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, title);
        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        dl.text(
            atlas,
            UI,
            self.f(14.0),
            lx,
            r.y + self.f(64.0),
            "The agent wants to run these state-changing API call(s):",
            TEXT,
            0.0,
        );
        for (i, l) in lines.iter().enumerate() {
            let y = r.y + self.f(92.0) + i as f32 * row;
            let s = dl.fit(atlas, MONO, self.f(12.5), l, fw);
            dl.text(atlas, MONO, self.f(12.5), lx, y, &s, RED, 0.0);
        }
        dl.text(
            atlas,
            UI,
            self.f(12.0),
            lx,
            r.y + ph - self.f(24.0),
            "[ENTER/Y] APPROVE & RUN · [ESC/N] DENY",
            FAINT,
            self.f(1.5),
        );
    }

    /// Risk warning shown the first time the user clicks the YOLO chip; only
    /// confirming here actually enables auto-approve.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_yolo_warn(
        &mut self,
        dl: &mut DrawList,
        atlas: &mut Atlas,
        w: f32,
        h: f32,
        pw: f32,
        lift: f32,
        title: &str,
    ) {
        let body: [&str; 8] = [
            "YOLO mode lets the agent run every mutating GitHub API call —",
            "creating, editing, deleting, merging, pushing, changing repo",
            "settings — WITHOUT asking you to approve each one first.",
            "",
            "It acts with your token's full permissions and can make",
            "irreversible changes. Only enable it for a task you trust and",
            "are actively watching. It is session-only and resets to OFF",
            "when you reload.",
        ];
        let row = self.f(20.0);
        let ph = self.f(150.0) + body.len() as f32 * row;
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        dl.glow(r, self.f(4.0), with_a(RED, 0.07), self.f(30.0));
        self.overlay_panel(dl, atlas, r, title);
        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        dl.text(
            atlas,
            UI_BOLD,
            self.f(14.5),
            lx,
            r.y + self.f(64.0),
            "⚠  AUTO-APPROVE WRITES — READ BEFORE ENABLING",
            RED,
            self.f(1.0),
        );
        for (i, l) in body.iter().enumerate() {
            let y = r.y + self.f(92.0) + i as f32 * row;
            let s = dl.fit(atlas, UI, self.f(13.0), l, fw);
            dl.text(atlas, UI, self.f(13.0), lx, y, &s, with_a(TEXT, 0.9), 0.0);
        }
        dl.text(
            atlas,
            UI,
            self.f(12.0),
            lx,
            r.y + ph - self.f(24.0),
            "[ENTER/Y] ENABLE YOLO · [ESC/N] CANCEL",
            FAINT,
            self.f(1.5),
        );
    }
}
