//! The Actions tab: workflow runs and the jobs/steps pane.

use super::*;

impl View {
    pub(super) fn actions_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let span = w - self.f(44.0);
        let ratio = self.actions_split.clamp(0.15, 0.85);
        let left = RectF::new(self.f(16.0), top, span * ratio, bottom - top);
        let right = RectF::new(left.right() + self.f(12.0), top, w - left.right() - self.f(28.0), bottom - top);
        self.panel(dl, left);
        self.panel(dl, right);
        // Draggable splitter in the gap between the two panes.
        let gap_cx = (left.right() + right.x) / 2.0;
        let handle = RectF::new(gap_cx - self.f(2.0), top + (bottom - top) / 2.0 - self.f(14.0), self.f(4.0), self.f(28.0));
        let hit = RectF::new(left.right(), top, right.x - left.right(), bottom - top);
        let hv = self.hover_amt(wid(Z_RUN, 9000), hit.contains(self.hot.0, self.hot.1));
        dl.rrect(handle, self.f(2.0), with_a(CYAN, 0.25 + 0.4 * hv), 1.0);
        self.actions_split_hit = Some((hit, left.x, span));
        dl.text(atlas, UI, self.f(12.0), left.x + self.f(14.0), top + self.f(24.0), "WORKFLOW RUNS", DIM, self.f(2.5));
        dl.text(atlas, UI, self.f(12.0), right.x + self.f(14.0), top + self.f(24.0), "JOBS", DIM, self.f(2.5));

        let row_h = self.f(32.0);
        let list = RectF::new(left.x + self.f(8.0), top + self.f(36.0), left.w - self.f(16.0), left.h - self.f(46.0));
        app.layout.runs_h = (list.h / row_h).max(1.0) as usize;

        enum RState {
            Note(String, bool),
            Ready,
        }
        let rstate = match &app.rv.as_ref().unwrap().runs {
            Loadable::Loading | Loadable::Idle => RState::Note("FETCHING RUNS…".into(), false),
            Loadable::Failed(e) => RState::Note(e.clone(), true),
            Loadable::Ready(r) if r.is_empty() => RState::Note("NO WORKFLOW RUNS".into(), false),
            Loadable::Ready(_) => RState::Ready,
        };
        match rstate {
            RState::Note(msg, err) => {
                if err {
                    let m = dl.fit(atlas, UI, self.f(13.0), &msg, list.w);
                    dl.text(atlas, UI, self.f(13.0), list.x + self.f(6.0), list.y + self.f(20.0), &m, RED, 0.0);
                } else {
                    self.sweep_note(dl, atlas, list.x + self.f(6.0), list.y + self.f(20.0), list.w, &msg);
                }
            }
            RState::Ready => {
                let (sel, count) = {
                    let rv = app.rv.as_ref().unwrap();
                    (rv.runs_sel.min(rv.runs.ready().unwrap().len().saturating_sub(1)), rv.runs.ready().unwrap().len())
                };
                let offset = self.list_scroll(Scroll::Runs, Z_RUN, sel, count, row_h, list.h);
                dl.push_clip(list);
                {
                    let rv = app.rv.as_ref().unwrap();
                    let runs = rv.runs.ready().unwrap();
                    let first = (offset / row_h) as usize;
                    for vis in 0..(list.h / row_h) as usize + 2 {
                        let i = first + vis;
                        if i >= runs.len() {
                            break;
                        }
                        let run = &runs[i];
                        let y = list.y + i as f32 * row_h - offset;
                        let rr = RectF::new(list.x, y, list.w, row_h - 2.0);
                        let selected = i == sel;
                        let hv = self.hover_amt(wid(Z_RUN, i), rr.contains(self.hot.0, self.hot.1));
                        let a = if selected { 0.12 } else { 0.05 * hv };
                        if a > 0.005 {
                            dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                        }
                        let (icon, rgb) = run_icon(&run.status, run.conclusion.as_deref());
                        let mut ic = crate::px::theme::c(rgb, 1.0);
                        if run.status == "in_progress" {
                            ic[3] = 0.5 + 0.5 * ((self.time * 0.005).sin() as f32);
                            self.active = true;
                        }
                        let baseline = y + self.f(21.0);
                        dl.text(atlas, MONO, self.f(13.0), rr.x + self.f(8.0), baseline, &icon.to_string(), ic, 0.0);
                        let title = run
                            .display_title
                            .clone()
                            .or_else(|| run.name.clone())
                            .unwrap_or_else(|| format!("run {}", run.id));
                        let label = format!("#{} {}", run.run_number, title);
                        let main_w = rr.w * 0.55;
                        let fitted = dl.fit(atlas, UI, self.f(13.5), &label, main_w);
                        let mut x = dl.text(atlas, UI, self.f(13.5), rr.x + self.f(26.0), baseline, &fitted, TEXT, 0.0);
                        if let Some(b) = &run.head_branch {
                            let bb = dl.fit(atlas, MONO, self.f(11.0), b, rr.w * 0.2);
                            x = dl.text(atlas, MONO, self.f(11.0), x + self.f(10.0), baseline, &bb, with_a(MAGENTA, 0.8), 0.0);
                        }
                        let meta = format!("{} {}", run.event, crate::app::fmt_age(&run.created_at));
                        let mw = dl.text_width(atlas, UI, self.f(11.0), &meta, 0.0);
                        let mx = (rr.right() - mw - self.f(8.0)).max(x + self.f(8.0));
                        dl.text(atlas, UI, self.f(11.0), mx, baseline, &meta, DIM, 0.0);
                        self.clicks.push((rr, Click::Run(i)));
                    }
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, count as f32 * row_h, offset);
                self.wheels.push((left, Scroll::Runs, row_h, (count as f32 * row_h - list.h).max(0.0)));
            }
        }

        // Jobs pane.
        let jlist = RectF::new(right.x + self.f(8.0), top + self.f(36.0), right.w - self.f(16.0), right.h - self.f(46.0));
        let jrow = self.f(26.0);
        app.layout.jobs_h = (jlist.h / jrow).max(1.0) as usize;
        // Drilled into a job → the log view replaces the jobs/steps list.
        if app.rv.as_ref().unwrap().job_logs.is_some() {
            self.job_log_view(app, dl, atlas, right, jlist);
            return;
        }
        let jstate = app.rv.as_ref().unwrap().jobs.as_ref().map(|(_, l)| match l {
            Loadable::Loading | Loadable::Idle => 0,
            Loadable::Failed(_) => 1,
            Loadable::Ready(_) => 2,
        });
        match jstate {
            None => {
                dl.text(atlas, UI, self.f(13.0), jlist.x + self.f(6.0), jlist.y + self.f(20.0), "PRESS ENTER ON A RUN TO LOAD ITS JOBS", FAINT, self.f(1.0));
            }
            Some(0) => self.sweep_note(dl, atlas, jlist.x + self.f(6.0), jlist.y + self.f(20.0), jlist.w, "FETCHING JOBS…"),
            Some(1) => {
                let e = match app.rv.as_ref().unwrap().jobs.as_ref() {
                    Some((_, Loadable::Failed(e))) => e.clone(),
                    _ => String::new(),
                };
                let m = dl.fit(atlas, UI, self.f(13.0), &e, jlist.w);
                dl.text(atlas, UI, self.f(13.0), jlist.x + self.f(6.0), jlist.y + self.f(20.0), &m, RED, 0.0);
            }
            _ => {
                let scroll_rows = app.rv.as_ref().unwrap().jobs_scroll;
                dl.push_clip(jlist);
                let rv = app.rv.as_ref().unwrap();
                if let Some((_, Loadable::Ready(jobs))) = &rv.jobs {
                    // (indent, icon, color, name, Some(job index) for headers).
                    let mut lines: Vec<(f32, char, Color, String, Option<usize>)> = Vec::new();
                    for (ji, job) in jobs.iter().enumerate() {
                        let (icon, rgb) = run_icon(&job.status, job.conclusion.as_deref());
                        lines.push((0.0, icon, crate::px::theme::c(rgb, 1.0), job.name.clone(), Some(ji)));
                        for step in &job.steps {
                            let (si, srgb) = run_icon(&step.status, step.conclusion.as_deref());
                            lines.push((self.f(18.0), si, crate::px::theme::c(srgb, 1.0), step.name.clone(), None));
                        }
                    }
                    for (vis, li) in (scroll_rows..lines.len()).enumerate() {
                        let y = jlist.y + vis as f32 * jrow;
                        if y > jlist.bottom() {
                            break;
                        }
                        let (indent, icon, ic, name, job_idx) = &lines[li];
                        let baseline = y + self.f(17.0);
                        // Job headers are clickable → open that job's logs.
                        if let Some(ji) = job_idx {
                            let rr = RectF::new(jlist.x, y - self.f(2.0), jlist.w, jrow);
                            let hv = self.hover_amt(wid(Z_RUN, 1000 + *ji), rr.contains(self.hot.0, self.hot.1));
                            if hv > 0.005 {
                                dl.rrect(rr, self.f(3.0), with_a(CYAN, 0.06 * hv), 1.0);
                            }
                            self.clicks.push((rr, Click::JobRow(*ji)));
                        }
                        let header = job_idx.is_some();
                        dl.text(atlas, MONO, self.f(12.0), jlist.x + self.f(4.0) + indent, baseline, &icon.to_string(), *ic, 0.0);
                        let font = if header { UI_BOLD } else { UI };
                        let fitted = dl.fit(atlas, font, self.f(13.0), name, jlist.w - indent - self.f(30.0));
                        dl.text(atlas, font, self.f(13.0), jlist.x + self.f(22.0) + indent, baseline, &fitted, if header { TEXT } else { with_a(TEXT, 0.75) }, 0.0);
                    }
                    self.wheels.push((right, Scroll::Jobs, jrow, (lines.len() as f32 * jrow - jlist.h).max(0.0)));
                }
                dl.pop_clip();
            }
        }
    }
}
