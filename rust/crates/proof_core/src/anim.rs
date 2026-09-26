//! Tiny animation helpers: easings, tweens and springs (engine-agnostic).

use crate::geom::V2;

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Overshoots then settles (pop-in).
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Bouncy elastic settle.
pub fn ease_out_elastic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = std::f32::consts::TAU / 3.0;
    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A critically-damped-ish spring for smooth follow motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    pub pos: f32,
    pub vel: f32,
    pub target: f32,
}

impl Spring {
    pub fn new(v: f32) -> Spring {
        Spring { pos: v, vel: 0.0, target: v }
    }
    /// `stiffness` ~ 120–400, `damping` ~ 10–30.
    pub fn step(&mut self, dt: f32, stiffness: f32, damping: f32) -> f32 {
        let dt = dt.min(1.0 / 20.0);
        let f = stiffness * (self.target - self.pos) - damping * self.vel;
        self.vel += f * dt;
        self.pos += self.vel * dt;
        self.pos
    }
    pub fn kick(&mut self, v: f32) {
        self.vel += v;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring2 {
    pub x: Spring,
    pub y: Spring,
}

impl Spring2 {
    pub fn new(p: V2) -> Spring2 {
        Spring2 { x: Spring::new(p.x), y: Spring::new(p.y) }
    }
    pub fn set_target(&mut self, p: V2) {
        self.x.target = p.x;
        self.y.target = p.y;
    }
    pub fn snap(&mut self, p: V2) {
        *self = Spring2::new(p);
    }
    pub fn step(&mut self, dt: f32, k: f32, d: f32) -> V2 {
        crate::geom::v2(self.x.step(dt, k, d), self.y.step(dt, k, d))
    }
    pub fn pos(&self) -> V2 {
        crate::geom::v2(self.x.pos, self.y.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_hit_endpoints() {
        for f in [ease_out_cubic, ease_in_out, ease_out_back, ease_out_elastic] {
            assert!(f(0.0).abs() < 1e-4);
            assert!((f(1.0) - 1.0).abs() < 1e-4);
        }
        assert!(ease_out_back(0.7) > 1.0, "overshoots");
    }

    #[test]
    fn spring_settles() {
        let mut s = Spring::new(0.0);
        s.target = 10.0;
        for _ in 0..300 {
            s.step(1.0 / 60.0, 200.0, 24.0);
        }
        assert!((s.pos - 10.0).abs() < 0.05);
    }
}
