//! Tileable paper/ink grain texture shared by the CPU preview and the Godot shader.
//!
//! RGBA8, `GRAIN_SIZE`² pixels: R = fine grain, G = coarse "ink starvation" blotches,
//! B = paper fibres, A = 255.

use proof_core::rng::hash01;

pub const GRAIN_SIZE: usize = 256;

fn value_noise(x: f32, y: f32, cell: usize, seed: u32) -> f32 {
    let cells = (GRAIN_SIZE / cell) as i32;
    let fx = x / cell as f32;
    let fy = y / cell as f32;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let h = |i: i32, j: i32| {
        let i = i.rem_euclid(cells) as u32;
        let j = j.rem_euclid(cells) as u32;
        hash01(i.wrapping_mul(7919) ^ seed, j.wrapping_add(seed.wrapping_mul(31)))
    };
    let a = h(x0, y0) + (h(x0 + 1, y0) - h(x0, y0)) * sx;
    let b = h(x0, y0 + 1) + (h(x0 + 1, y0 + 1) - h(x0, y0 + 1)) * sx;
    a + (b - a) * sy
}

pub fn grain_texture() -> Vec<u8> {
    let n = GRAIN_SIZE;
    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (xf, yf) = (x as f32, y as f32);
            // Fine grain: white noise with a touch of 2px value noise; biased so most pixels
            // are near 0 and a few specks are strong (like real riso speckle).
            let white = hash01(x as u32, y as u32 ^ 0x5eed);
            let soft = value_noise(xf, yf, 2, 11);
            let fine = (0.55 * white + 0.45 * soft).powf(2.6);
            let coarse = (0.6 * value_noise(xf, yf, 64, 3) + 0.4 * value_noise(xf, yf, 16, 5)).powf(1.5);
            // Fibres: stretched noise.
            let fib = value_noise(xf * 0.25, yf, 4, 17).powf(3.0);
            let i = (y * n + x) * 4;
            out[i] = (fine.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 1] = (coarse.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 2] = (fib.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 3] = 255;
        }
    }
    out
}
