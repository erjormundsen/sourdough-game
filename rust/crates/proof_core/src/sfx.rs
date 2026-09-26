//! A tiny procedural sound synthesiser: every effect in the game is generated here,
//! so there are no audio assets to ship.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

pub const RATE: u32 = 22_050;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sound {
    Tap,
    Squish,
    Blade,
    Pour,
    Bubble,
    Ding,
    Coin,
    Stamp,
    Sparkle,
    Whoosh,
    Chirp,
    Plop,
}

impl Sound {
    pub const ALL: [Sound; 12] = [
        Sound::Tap,
        Sound::Squish,
        Sound::Blade,
        Sound::Pour,
        Sound::Bubble,
        Sound::Ding,
        Sound::Coin,
        Sound::Stamp,
        Sound::Sparkle,
        Sound::Whoosh,
        Sound::Chirp,
        Sound::Plop,
    ];
}

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn buf(secs: f32) -> Vec<f32> {
    vec![0.0; (secs * RATE as f32) as usize]
}

/// Add a sine sweep from `f0` to `f1` Hz with an exponential decay.
fn tone(out: &mut [f32], start: f32, dur: f32, f0: f32, f1: f32, amp: f32, decay: f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = (dur * RATE as f32) as usize;
    let mut ph = 0.0f32;
    for i in 0..n {
        let Some(o) = out.get_mut(s0 + i) else { break };
        let t = i as f32 / RATE as f32;
        let k = i as f32 / n as f32;
        let f = f0 + (f1 - f0) * k;
        ph += TAU * f / RATE as f32;
        let attack = (t / 0.004).min(1.0);
        *o += ph.sin() * amp * attack * (-t * decay).exp();
    }
}

#[allow(clippy::too_many_arguments)]
/// Filtered noise burst; `cutoff` sweeps from `c0` to `c1` (0..1 one-pole coefficient).
fn hiss(out: &mut [f32], start: f32, dur: f32, c0: f32, c1: f32, amp: f32, seed: u32, shape: fn(f32) -> f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = (dur * RATE as f32) as usize;
    let mut rng = Noise(seed | 1);
    let (mut lp, mut prev) = (0.0f32, 0.0f32);
    for i in 0..n {
        let Some(o) = out.get_mut(s0 + i) else { break };
        let k = i as f32 / n as f32;
        let c = c0 + (c1 - c0) * k;
        let x = rng.next();
        lp += c * (x - lp);
        let hp = lp - prev * 0.6;
        prev = lp;
        *o += hp * amp * shape(k);
    }
}

fn env_bump(k: f32) -> f32 {
    (k * std::f32::consts::PI).sin().powf(0.7)
}
fn env_fall(k: f32) -> f32 {
    (1.0 - k).powf(2.0) * (k / 0.03).min(1.0)
}

/// Render a sound effect as mono f32 samples at [`RATE`].
pub fn render(s: Sound) -> Vec<f32> {
    let mut o;
    match s {
        Sound::Tap => {
            o = buf(0.09);
            tone(&mut o, 0.0, 0.09, 900.0, 520.0, 0.5, 40.0);
        }
        Sound::Squish => {
            o = buf(0.26);
            hiss(&mut o, 0.0, 0.26, 0.08, 0.02, 1.2, 7, env_bump);
            tone(&mut o, 0.0, 0.2, 220.0, 120.0, 0.25, 12.0);
        }
        Sound::Blade => {
            o = buf(0.24);
            hiss(&mut o, 0.0, 0.24, 0.9, 0.35, 0.55, 11, env_fall);
        }
        Sound::Pour => {
            o = buf(0.5);
            hiss(&mut o, 0.0, 0.5, 0.25, 0.4, 0.5, 5, env_bump);
        }
        Sound::Bubble => {
            o = buf(0.3);
            tone(&mut o, 0.0, 0.1, 260.0, 700.0, 0.35, 25.0);
            tone(&mut o, 0.13, 0.1, 340.0, 900.0, 0.28, 25.0);
        }
        Sound::Ding => {
            o = buf(1.2);
            tone(&mut o, 0.0, 1.2, 1318.5, 1318.5, 0.35, 4.0);
            tone(&mut o, 0.0, 1.2, 2637.0, 2637.0, 0.12, 6.0);
            tone(&mut o, 0.0, 1.2, 1975.5, 1975.5, 0.08, 5.0);
        }
        Sound::Coin => {
            o = buf(0.3);
            tone(&mut o, 0.0, 0.08, 988.0, 988.0, 0.3, 10.0);
            tone(&mut o, 0.07, 0.22, 1318.5, 1318.5, 0.3, 12.0);
        }
        Sound::Stamp => {
            o = buf(0.3);
            tone(&mut o, 0.0, 0.3, 140.0, 60.0, 0.7, 14.0);
            hiss(&mut o, 0.0, 0.05, 0.5, 0.2, 0.6, 3, env_fall);
        }
        Sound::Sparkle => {
            o = buf(0.6);
            for (i, f) in [1046.5f32, 1318.5, 1568.0, 2093.0].iter().enumerate() {
                tone(&mut o, i as f32 * 0.07, 0.3, *f, *f, 0.18, 9.0);
            }
        }
        Sound::Whoosh => {
            o = buf(0.4);
            hiss(&mut o, 0.0, 0.4, 0.05, 0.5, 0.45, 9, env_bump);
        }
        Sound::Chirp => {
            o = buf(0.3);
            tone(&mut o, 0.0, 0.1, 700.0, 1100.0, 0.25, 12.0);
            tone(&mut o, 0.1, 0.15, 900.0, 1500.0, 0.25, 12.0);
        }
        Sound::Plop => {
            o = buf(0.18);
            tone(&mut o, 0.0, 0.18, 320.0, 140.0, 0.5, 18.0);
        }
    }
    // Gentle limiter.
    let peak = o.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if peak > 0.9 {
        for x in &mut o {
            *x *= 0.9 / peak;
        }
    }
    o
}

/// 16-bit little-endian PCM bytes.
pub fn to_pcm16(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|x| ((x.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sounds_render_bounded_and_nonsilent() {
        for s in Sound::ALL {
            let o = render(s);
            assert!(!o.is_empty());
            let peak = o.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.05 && peak <= 0.91, "{s:?} peak {peak}");
            assert!(o.iter().all(|x| x.is_finite()));
        }
    }
}
