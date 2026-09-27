//! Press sheets: how the riso inks, screens, overprints and edges actually print.
//! Owned by the press area (`proof_raster`, `ink`, Godot riso shaders).

use crate::Board;
use proof_core::art::bread::{LoafView, loaf_top};
use proof_core::art::{Expr, face};
use proof_core::draw::{Paint, Screen};
use proof_core::geom::{V2, circle, rect, rect_poly, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

const INKS: [Ink; 4] = [Ink::Pink, Ink::Yellow, Ink::Blue, Ink::Key];
const TONES: [f32; 10] = [0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0];

pub fn render(out: &Path, scale: f32) {
    for ed in [Edition::Dawn, Edition::Daylight, Edition::Dusk] {
        let mut b = Board::new(1200.0, 1500.0, scale * 0.6, ed);
        // Tint ramps: solid tint, halftone and coarse screens for every ink.
        for (r, ink) in INKS.iter().enumerate() {
            for (c, t) in TONES.iter().enumerate() {
                for (k, screen) in [Screen::Solid, Screen::Halftone, Screen::Coarse].iter().enumerate() {
                    let x = 40.0 + c as f32 * 112.0;
                    let y = 40.0 + r as f32 * 250.0 + k as f32 * 76.0;
                    let p =
                        Paint { ink: *ink, tone: *t, screen: *screen, mode: proof_core::draw::Mode::Over };
                    b.put(V2::ZERO, |d| d.fill_p(p, &rect_poly(rect(x, y, 100.0, 66.0))));
                }
            }
        }
        // Overprints: every pair and the three-colour stack, solid and 50% screened.
        let pairs = [
            (Ink::Pink, Ink::Yellow),
            (Ink::Yellow, Ink::Blue),
            (Ink::Pink, Ink::Blue),
            (Ink::Key, Ink::Yellow),
            (Ink::Key, Ink::Pink),
            (Ink::Key, Ink::Blue),
        ];
        for (i, (a, c)) in pairs.iter().enumerate() {
            let x = 40.0 + i as f32 * 150.0;
            b.put(V2::ZERO, |d| {
                d.fill(*a, 1.0, &circle(v2(x + 45.0, 1060.0), 44.0));
                d.fill(*c, 1.0, &circle(v2(x + 85.0, 1060.0), 44.0));
                d.ht(*a, 0.5, &circle(v2(x + 45.0, 1170.0), 44.0));
                d.ht(*c, 0.5, &circle(v2(x + 85.0, 1170.0), 44.0));
            });
        }
        b.put(V2::ZERO, |d| {
            for (k, ink) in [Ink::Pink, Ink::Yellow, Ink::Blue].iter().enumerate() {
                let c = v2(1010.0, 1100.0) + V2::from_angle(k as f32 * 2.09 - 1.57) * 42.0;
                d.fill_p(Paint::solid(*ink, 1.0).add(), &circle(c, 62.0));
            }
        });
        // Edges and fine lines: widths 1..8 in every ink, plus knockout text-sized marks.
        for (r, ink) in INKS.iter().enumerate() {
            b.put(V2::ZERO, |d| {
                for w in 1..=8 {
                    let x = 40.0 + w as f32 * 34.0;
                    d.line(
                        *ink,
                        w as f32,
                        &[v2(x, 1250.0 + r as f32 * 58.0), v2(x + 20.0, 1300.0 + r as f32 * 58.0)],
                    );
                }
                let blk = rect_poly(rect(360.0 + r as f32 * 110.0, 1250.0, 96.0, 96.0));
                d.fill(*ink, 1.0, &blk);
                d.knock(&circle(v2(408.0 + r as f32 * 110.0, 1298.0), 22.0));
                d.knock_line(
                    1.0,
                    3.0,
                    &[v2(372.0 + r as f32 * 110.0, 1332.0), v2(444.0 + r as f32 * 110.0, 1262.0)],
                    false,
                );
            });
        }
        // Real art at game size: a face (key-ink legibility) and a loaf (screens + overprint).
        b.put(v2(880.0, 1350.0), |d| {
            d.fill(Ink::Yellow, 0.42, &circle(V2::ZERO, 70.0));
            face(d, v2(0.0, -4.0), 82.0, Expr::Content, V2::ZERO);
        });
        b.put(v2(1080.0, 1350.0), |d| loaf_top(d, &LoafView { r: 80.0, ..LoafView::default() }));
        b.save(&out.join(format!("riso_swatches_{}.png", format!("{ed:?}").to_lowercase())));
    }
}
