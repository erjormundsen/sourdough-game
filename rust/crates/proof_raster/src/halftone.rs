//! Halftone dot tiles, cached per quantised tone.

use std::collections::HashMap;
use tiny_skia::{FilterQuality, IntSize, Pattern, Pixmap, Shader, SpreadMode, Transform};

const TILE: u32 = 24;
const SUB: u32 = 4;
const LEVELS: f32 = 48.0;

/// Rank of every sub-sample in a cell ordered by distance from the centre: a value v in (0,1]
/// means "this sub-sample is inked once tone ≥ v". Produces round dots that grow evenly.
fn ranks() -> Vec<f32> {
    let n = (TILE * SUB) as usize;
    let c = n as f32 * 0.5;
    let mut d: Vec<(f32, usize)> = (0..n * n)
        .map(|i| {
            let x = (i % n) as f32 + 0.5;
            let y = (i / n) as f32 + 0.5;
            // Slightly squarish metric so dots join into a checker at 50%.
            let (dx, dy) = ((x - c).abs() / c, (y - c).abs() / c);
            let r = (dx.powf(2.2) + dy.powf(2.2)).powf(1.0 / 2.2);
            (r + (i as f32) * 1e-9, i)
        })
        .collect();
    d.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = vec![0.0; n * n];
    let total = (n * n) as f32;
    for (rank, (_, i)) in d.into_iter().enumerate() {
        out[i] = (rank as f32 + 1.0) / total;
    }
    out
}

#[derive(Default)]
pub struct TileCache {
    ranks: Vec<f32>,
    tiles: HashMap<u32, Pixmap>,
}

impl TileCache {
    fn tile(&mut self, level: u32) -> &Pixmap {
        if self.ranks.is_empty() {
            self.ranks = ranks();
        }
        let ranks = &self.ranks;
        self.tiles.entry(level).or_insert_with(|| {
            let tone = level as f32 / LEVELS;
            let n = TILE * SUB;
            let mut data = vec![0u8; (TILE * TILE * 4) as usize];
            for ty in 0..TILE {
                for tx in 0..TILE {
                    let mut on = 0u32;
                    for sy in 0..SUB {
                        for sx in 0..SUB {
                            let i = ((ty * SUB + sy) * n + tx * SUB + sx) as usize;
                            if ranks[i] <= tone {
                                on += 1;
                            }
                        }
                    }
                    let a = (on * 255 / (SUB * SUB)) as u8;
                    // Premultiplied black with alpha `a`.
                    data[((ty * TILE + tx) * 4 + 3) as usize] = a;
                }
            }
            Pixmap::from_vec(data, IntSize::from_wh(TILE, TILE).unwrap()).unwrap()
        })
    }

    /// Pattern shader for `tone` with cells of `pitch_px` device pixels at `angle`,
    /// anchored to reference-space origin so neighbouring sprites share a screen.
    pub fn pattern(
        &mut self,
        tone: f32,
        pitch_px: f32,
        angle: f32,
        ox: f32,
        oy: f32,
    ) -> Shader<'_> {
        let level = (tone * LEVELS).round().clamp(1.0, LEVELS - 1.0) as u32;
        let k = pitch_px / TILE as f32;
        let ts = Transform::from_translate(-ox, -oy)
            .pre_concat(Transform::from_rotate(angle.to_degrees()))
            .pre_concat(Transform::from_scale(k, k));
        let tile = self.tile(level);
        Pattern::new(
            tile.as_ref(),
            SpreadMode::Repeat,
            FilterQuality::Bilinear,
            1.0,
            ts,
        )
    }
}
