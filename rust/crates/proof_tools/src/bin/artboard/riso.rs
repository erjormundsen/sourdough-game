//! Press sheets: how the riso inks, screens, overprints and edges actually print.
//! Owned by the press area (`proof_raster`, `ink`, Godot riso shaders).
//!
//! * `riso_swatches_<edition>` — tint ramps (grain / halftone / coarse), overprints, lines,
//!   knockouts, a face and a loaf. Same layout as ever, for before/after comparison.
//! * `riso_press_<edition>` — the press itself: ink film on big solids and overprints,
//!   registration marks across the page (drum rotation), the print-in kick, printed props.
//! * `riso_zoom_<edition>` — a 4× loupe on grain, dots, edges and a face.

use crate::Board;
use proof_core::art::bread::{LoafView, loaf_top};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::{Expr, face, props};
use proof_core::draw::{DrawList, Paint, Screen};
use proof_core::geom::{V2, circle, heart, rect, rect_poly, rounded_rect, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

const INKS: [Ink; 4] = [Ink::Pink, Ink::Yellow, Ink::Blue, Ink::Key];
const TONES: [f32; 10] = [0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0];
const EDITIONS: [Edition; 3] = [Edition::Dawn, Edition::Daylight, Edition::Dusk];

fn name(ed: Edition) -> String {
    format!("{ed:?}").to_lowercase()
}

pub fn render(out: &Path, scale: f32) {
    for ed in EDITIONS {
        swatches(out, scale, ed);
        press(out, scale, ed);
        zoom(out, ed);
    }
}

fn swatches(out: &Path, scale: f32, ed: Edition) {
    let mut b = Board::new(1200.0, 1500.0, scale * 0.6, ed);
    // Tint ramps: flat tint (printed as grain), halftone and coarse screens for every ink.
    for (r, ink) in INKS.iter().enumerate() {
        for (c, t) in TONES.iter().enumerate() {
            for (k, screen) in [Screen::Solid, Screen::Halftone, Screen::Coarse].iter().enumerate() {
                let x = 40.0 + c as f32 * 112.0;
                let y = 40.0 + r as f32 * 250.0 + k as f32 * 76.0;
                let p = Paint { ink: *ink, tone: *t, screen: *screen, mode: proof_core::draw::Mode::Over };
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
    b.save(&out.join(format!("riso_swatches_{}.png", name(ed))));
}

/// Printer's registration mark: a crosshair in a circle, printed once per drum.
fn reg_mark(d: &mut DrawList, c: V2, r: f32) {
    for ink in INKS {
        d.stroke_p(Paint::solid(ink, 1.0).add(), 1.6, &circle(c, r * 0.62), true);
        d.fill_p(Paint::solid(ink, 1.0).add(), &rect_poly(rect(c.x - r, c.y - 0.8, r * 2.0, 1.6)));
        d.fill_p(Paint::solid(ink, 1.0).add(), &rect_poly(rect(c.x - 0.8, c.y - r, 1.6, r * 2.0)));
    }
}

/// A little outlined badge: colour fills under a key contour, so registration shows.
fn badge(d: &mut DrawList, c: V2) {
    let disc = circle(c, 46.0);
    d.backing(&disc);
    d.fill(Ink::Yellow, 0.9, &disc);
    d.ht(Ink::Pink, 0.35, &circle(c + v2(-14.0, 12.0), 22.0));
    d.fill(Ink::Pink, 1.0, &heart(c + v2(0.0, -2.0), 30.0));
    d.fill(Ink::Blue, 0.8, &rounded_rect(rect(c.x - 30.0, c.y + 24.0, 60.0, 10.0), 5.0));
    d.outline(Ink::Key, 5.0, &disc);
    d.outline(Ink::Key, 2.2, &heart(c + v2(0.0, -2.0), 30.0));
}

fn press(out: &Path, scale: f32, ed: Edition) {
    let (w, h) = (1200.0, 1500.0);
    let mut b = Board::new(w, h, scale * 0.6, ed);
    // 1. Ink film: big solids and overprints (mottling, velvet, pinholes, feed streaks,
    //    starvation, trapping).
    for (i, ink) in INKS.iter().enumerate() {
        b.put(V2::ZERO, |d| d.fill(*ink, 1.0, &rect_poly(rect(60.0 + i as f32 * 275.0, 60.0, 250.0, 200.0))));
    }
    let over: [&[Ink]; 4] = [
        &[Ink::Yellow, Ink::Pink],
        &[Ink::Yellow, Ink::Blue],
        &[Ink::Pink, Ink::Blue],
        &[Ink::Yellow, Ink::Pink, Ink::Blue],
    ];
    for (i, set) in over.iter().enumerate() {
        b.put(V2::ZERO, |d| {
            for ink in set.iter() {
                d.fill(*ink, 1.0, &rect_poly(rect(60.0 + i as f32 * 275.0, 290.0, 250.0, 150.0)));
            }
        });
    }
    // 2. Registration: marks near the corners and centre show each drum's offset changing
    //    across the page (a hair of rotation), badges show fills against the key line.
    for (x, y) in [(40.0, 500.0), (w - 40.0, 500.0), (w * 0.5, 500.0), (40.0, h - 40.0), (w - 40.0, h - 40.0)]
    {
        b.put(V2::ZERO, |d| reg_mark(d, v2(x, y), 22.0));
    }
    for (i, x) in [150.0, 450.0, 750.0, 1050.0].iter().enumerate() {
        b.put(V2::ZERO, |d| badge(d, v2(*x, 600.0 + (i % 2) as f32 * 20.0)));
    }
    // 3. The print-in kick: the same badge as the page slams into the press and settles.
    for (i, k) in [1.0, 0.6, 0.3, 0.1, 0.0].iter().enumerate() {
        b.style.kick = *k;
        b.put(V2::ZERO, |d| badge(d, v2(140.0 + i as f32 * 230.0, 800.0)));
    }
    b.style.kick = 0.0;
    // 4. Printed props at game size.
    b.put(v2(170.0, 1180.0), |d| jar(d, &JarView::default()));
    b.put(v2(450.0, 1040.0), |d| loaf_top(d, &LoafView { r: 110.0, crust: 0.7, ..LoafView::default() }));
    b.put(v2(450.0, 1300.0), |d| {
        loaf_top(d, &LoafView { r: 90.0, bake: 0.0, spring: 0.0, ..LoafView::default() })
    });
    b.put(v2(820.0, 1020.0), |d| props::ticket(d, 330.0, 150.0, Ink::Pink));
    b.put(v2(820.0, 1160.0), |d| props::button(d, 300.0, 80.0, Ink::Pink, false));
    b.put(v2(820.0, 1260.0), |d| props::button(d, 300.0, 80.0, Ink::Blue, true));
    b.put(v2(1080.0, 1380.0), |d| props::stamp(d, 70.0, 3, Ink::Pink, 4));
    b.put(v2(700.0, 1380.0), |d| {
        d.fill(Ink::Yellow, 0.42, &circle(V2::ZERO, 70.0));
        face(d, v2(0.0, -4.0), 82.0, Expr::Happy, V2::ZERO);
    });
    b.save(&out.join(format!("riso_press_{}.png", name(ed))));
}

fn zoom(out: &Path, ed: Edition) {
    // A loupe: 4 device px per unit (a phone is 1.5–2.5).
    let mut b = Board::new(330.0, 200.0, 4.0, ed);
    b.put(V2::ZERO, |d| {
        let r = rect_poly(rect(10.0, 10.0, 150.0, 80.0));
        d.fill(Ink::Yellow, 0.3, &r);
        d.ht(Ink::Pink, 0.25, &rect_poly(rect(90.0, 10.0, 70.0, 80.0)));
        d.fill(Ink::Blue, 0.6, &rect_poly(rect(10.0, 100.0, 70.0, 90.0)));
        d.fill(Ink::Key, 1.0, &rect_poly(rect(90.0, 100.0, 70.0, 90.0)));
        d.knock(&circle(v2(125.0, 145.0), 16.0));
        d.fill(Ink::Pink, 0.85, &circle(v2(125.0, 145.0), 10.0));
        for (i, wd) in [1.0, 2.0, 3.2, 5.0].iter().enumerate() {
            d.line(Ink::Key, *wd, &[v2(20.0 + i as f32 * 14.0, 118.0), v2(26.0 + i as f32 * 14.0, 178.0)]);
        }
    });
    b.put(v2(245.0, 58.0), |d| loaf_top(d, &LoafView { r: 48.0, crust: 0.6, ..LoafView::default() }));
    b.put(v2(245.0, 150.0), |d| {
        d.fill(Ink::Yellow, 0.42, &circle(V2::ZERO, 42.0));
        face(d, v2(0.0, -2.0), 50.0, Expr::Content, V2::ZERO);
    });
    b.save(&out.join(format!("riso_zoom_{}.png", name(ed))));
}
