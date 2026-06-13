//! Tiny animation primitives: exponential approach toward a target.
//! Frame-rate independent; `tick` returns true while still moving.

#[derive(Clone, Copy)]
pub struct Smooth {
    pub v: f32,
    pub target: f32,
}

impl Smooth {
    pub fn new(v: f32) -> Self {
        Smooth { v, target: v }
    }

    pub fn snap(&mut self, v: f32) {
        self.v = v;
        self.target = v;
    }

    /// For pixel-valued animations. `speed` ~ 12.0 gives a ~150ms feel.
    pub fn tick(&mut self, dt: f32, speed: f32) -> bool {
        self.tick_eps(dt, speed, 0.25)
    }

    /// For normalized 0..1 animations.
    pub fn tick_n(&mut self, dt: f32, speed: f32) -> bool {
        self.tick_eps(dt, speed, 0.003)
    }

    pub fn tick_eps(&mut self, dt: f32, speed: f32, eps: f32) -> bool {
        let d = self.target - self.v;
        if d.abs() < eps {
            self.v = self.target;
            return false;
        }
        self.v += d * (1.0 - (-dt * speed).exp());
        true
    }
}

/// 0→1 ease-out cubic.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
