//! Per-frame orchestration: timing, route dispatch, busy sweep, cursor.

use super::*;

impl View {

    pub fn frame(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, t_ms: f64) {
        self.dt = if self.started {
            (((t_ms - self.time) / 1000.0) as f32).clamp(0.001, 0.05)
        } else {
            0.016
        };
        self.time = t_ms;
        self.started = true;
        self.active = false;
        self.clicks.clear();
        self.wheels.clear();
        self.editor_geom = None;
        self.agent_geom = None;
        if app.route != Route::Agent {
            self.agent_sel = None;
        }
        self.hot = if app.overlay.is_some() { (-1e6, -1e6) } else { self.mouse };

        dl.begin(w, h);
        self.background(dl, w, h);

        // Route entrance.
        let tag = match app.route {
            Route::Auth => 0,
            Route::Repos => 1,
            Route::Repo => 2,
            Route::Agent => 3,
        };
        if tag != self.last_route {
            self.last_route = tag;
            self.route_t.snap(0.0);
            self.route_t.target = 1.0;
        }
        if self.route_t.tick_n(self.dt, 9.0) {
            self.active = true;
        }
        let yoff = (1.0 - ease_out(self.route_t.v)) * self.f(16.0);

        match app.route {
            Route::Auth => self.auth_screen(app, dl, atlas, w, h, yoff),
            Route::Repos => self.repos_screen(app, dl, atlas, w, h, yoff),
            Route::Repo => self.repo_screen(app, dl, atlas, w, h, yoff),
            Route::Agent => self.agent_screen(app, dl, atlas, w, h, yoff),
        }

        self.status_bar(app, dl, atlas, w, h);
        self.overlay(app, dl, atlas, w, h);
        self.toast(app, dl, atlas, w, h);

        // Busy sweep along the very top.
        if busy(app) {
            let p = ((self.time * 0.00045) % 1.3) as f32 - 0.15;
            let bw = w * 0.22;
            let r = RectF::new(p * w, 0.0, bw, self.f(2.0));
            dl.glow(r, 1.0, with_a(CYAN, 0.25), self.f(8.0));
            dl.solid(r, with_a(CYAN, 0.9));
            self.active = true;
        }

        dl.scanlines(w, h, 0.05);

        self.cursor_pointer = {
            let (mx, my) = self.mouse;
            self.clicks.iter().any(|(r, _)| r.contains(mx, my))
        };
        self.cursor_text = {
            let (mx, my) = self.mouse;
            !self.cursor_pointer
                && self.agent_geom.map(|(r, ..)| r.contains(mx, my)).unwrap_or(false)
        };
        self.hover.retain(|_, s| s.v > 0.002 || s.target > 0.0);
    }

    fn background(&self, dl: &mut DrawList, w: f32, h: f32) {
        let step = self.f(72.0);
        let c = with_a(CYAN, 0.022);
        let mut x = step;
        while x < w {
            dl.solid(RectF::new(x, 0.0, 1.0, h), c);
            x += step;
        }
        let mut y = step;
        while y < h {
            dl.solid(RectF::new(0.0, y, w, 1.0), c);
            y += step;
        }
    }
}

fn busy(app: &App) -> bool {
    if app.auth_busy || app.agent.busy || matches!(app.repos, Loadable::Loading) {
        return true;
    }
    if let Some(rv) = &app.rv {
        if matches!(rv.branches, Loadable::Loading)
            || matches!(rv.tree, Loadable::Loading)
            || matches!(rv.runs, Loadable::Loading)
            || rv.file_loading.is_some()
            || matches!(rv.jobs, Some((_, Loadable::Loading)))
            || rv.file.as_ref().map(|f| f.committing).unwrap_or(false)
        {
            return true;
        }
    }
    false
}
