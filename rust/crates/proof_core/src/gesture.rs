//! Touch gesture recognisers. The engine feeds raw samples; everything else is pure maths.

use crate::geom::{V2, polyline_len, v2};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub pos: V2,
    /// Seconds since the gesture began.
    pub t: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub samples: Vec<Sample>,
}

impl Stroke {
    pub fn points(&self) -> Vec<V2> {
        self.samples.iter().map(|s| s.pos).collect()
    }
    pub fn len(&self) -> f32 {
        polyline_len(&self.points())
    }
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    pub fn duration(&self) -> f32 {
        match (self.samples.first(), self.samples.last()) {
            (Some(a), Some(b)) => (b.t - a.t).max(1e-3),
            _ => 1e-3,
        }
    }
    pub fn chord(&self) -> V2 {
        match (self.samples.first(), self.samples.last()) {
            (Some(a), Some(b)) => b.pos - a.pos,
            _ => V2::ZERO,
        }
    }
}

/// A tap: short and nearly stationary.
pub fn is_tap(s: &Stroke, slop: f32) -> bool {
    !s.is_empty() && s.len() < slop && s.duration() < 0.45
}

/// Dominant direction of a swipe (unit vector) if it travelled at least `min_len`.
pub fn swipe_dir(s: &Stroke, min_len: f32) -> Option<V2> {
    let c = s.chord();
    if c.len() >= min_len { Some(c.norm()) } else { None }
}

/// Snap a direction to one of four compass directions: 0 up, 1 right, 2 down, 3 left.
pub fn compass4(dir: V2) -> usize {
    if dir.x.abs() > dir.y.abs() {
        if dir.x > 0.0 { 1 } else { 3 }
    } else if dir.y < 0.0 {
        0
    } else {
        2
    }
}

pub fn compass_vec(i: usize) -> V2 {
    match i % 4 {
        0 => v2(0.0, -1.0),
        1 => v2(1.0, 0.0),
        2 => v2(0.0, 1.0),
        _ => v2(-1.0, 0.0),
    }
}

/// Signed number of turns a stroke makes around `center` (positive = clockwise on screen).
pub fn turns_around(s: &Stroke, center: V2) -> f32 {
    let pts = s.points();
    let mut total = 0.0;
    for w in pts.windows(2) {
        let a = (w[0] - center).angle();
        let b = (w[1] - center).angle();
        let mut d = b - a;
        while d > TAU * 0.5 {
            d -= TAU;
        }
        while d < -TAU * 0.5 {
            d += TAU;
        }
        total += d;
    }
    total / TAU
}

/// Stir quality from how many circles were made (two good laps = perfect).
pub fn stir_quality(turns: f32) -> f32 {
    (turns.abs() / 2.0).clamp(0.0, 1.0)
}

/// Stretch & fold quality: each swipe compared with the expected compass direction.
pub fn fold_quality(swipes: &[V2], expected: &[usize]) -> f32 {
    if expected.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for (i, e) in expected.iter().enumerate() {
        let q = swipes.get(i).map(|d| d.norm().dot(compass_vec(*e)).max(0.0)).unwrap_or(0.0);
        sum += q;
    }
    (sum / expected.len() as f32).clamp(0.0, 1.0)
}

/// The fold order players follow: up, right, down, left.
pub const FOLD_ORDER: [usize; 4] = [0, 1, 2, 3];

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(pts: &[V2]) -> Stroke {
        Stroke {
            samples: pts.iter().enumerate().map(|(i, p)| Sample { pos: *p, t: i as f32 * 0.016 }).collect(),
        }
    }

    #[test]
    fn circles_count_turns() {
        let pts: Vec<V2> = (0..=100).map(|i| V2::from_angle(i as f32 / 100.0 * TAU * 2.0) * 50.0).collect();
        let t = turns_around(&stroke(&pts), V2::ZERO);
        assert!((t - 2.0).abs() < 0.05, "{t}");
        assert!((stir_quality(t) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn swipes_snap_to_compass() {
        assert_eq!(compass4(v2(0.1, -1.0)), 0);
        assert_eq!(compass4(v2(1.0, 0.2)), 1);
        assert_eq!(compass4(v2(0.0, 1.0)), 2);
        assert_eq!(compass4(v2(-1.0, 0.3)), 3);
        let perfect: Vec<V2> = FOLD_ORDER.iter().map(|i| compass_vec(*i)).collect();
        assert!((fold_quality(&perfect, &FOLD_ORDER) - 1.0).abs() < 1e-5);
        let wrong: Vec<V2> = FOLD_ORDER.iter().map(|i| compass_vec(i + 2)).collect();
        assert_eq!(fold_quality(&wrong, &FOLD_ORDER), 0.0);
    }

    #[test]
    fn taps_are_short() {
        assert!(is_tap(&stroke(&[v2(0.0, 0.0), v2(2.0, 1.0)]), 12.0));
        assert!(!is_tap(&stroke(&[v2(0.0, 0.0), v2(40.0, 0.0)]), 12.0));
    }
}
