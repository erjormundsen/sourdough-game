//! Small deterministic PCG32 random number generator (serialisable, no dependencies).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng { state: 0, inc: (seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(seed ^ 0x853C_49E6_748F_EA9B);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform integer in [0, n).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { ((self.next_u32() as u64 * n as u64) >> 32) as u32 }
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() { None } else { items.get(self.below(items.len() as u32) as usize) }
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// Stateless hash → [0,1), handy for deterministic art scatter.
pub fn hash01(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x27d4_eb2d) ^ b.wrapping_mul(0x1656_67b1).rotate_left(13);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let x = a.f32();
            assert_eq!(x, b.f32());
            assert!((0.0..1.0).contains(&x));
        }
        let mut c = Rng::new(7);
        let hits: u32 = (0..10_000).map(|_| c.below(10)).filter(|&v| v == 3).count() as u32;
        assert!((800..1200).contains(&hits), "{hits}");
    }
}
