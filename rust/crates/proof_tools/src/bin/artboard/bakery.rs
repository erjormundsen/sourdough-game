//! Bakery sheets: dough, loaves, crumb, starter jars, Toasty and treats.
//! Owned by the bakery-objects area (`proof_core::art::{bread, jar, oven, treats}`).
//!
//! * `bakery_loaves`      — raw doughs and a showcase of finished loaves (hero size)
//! * `bakery_states`      — oven bake 0→1, reveal bloom 0→1, crust levels, in-game sizes, crumb
//! * `bakery_jars`        — moods & microbiomes at 1×, plus the 0.42× and 0.8× shelf sizes
//! * `bakery_oven_treats` — Toasty's states and the treat tray (raw → finished, all sizes)

use crate::Board;
use proof_core::art::Expr;
use proof_core::art::bread::{CutView, LoafView, crumb_slice, loaf_top};
use proof_core::art::jar::{JarView, TAPE_CENTER, jar};
use proof_core::art::oven::{OvenView, oven};
use proof_core::art::treats::{treat, treat_raw};
use proof_core::content::{Pattern, Recipe, Shape, Stencil, Topping, Treat};
use proof_core::draw::{DrawList, Paint};
use proof_core::geom::{V2, Xf, rect, rect_poly, v2};
use proof_core::ink::{Edition, Ink};
use proof_core::scoring::template;
use std::path::Path;

/// (recipe, pattern, shape, crust, stencil, topping) for a showcase loaf.
type Look = (Recipe, Option<Pattern>, Shape, f32, Option<Stencil>, Option<Topping>);

pub fn cuts(p: Pattern, shape: Shape, bloom: f32, ear: f32) -> Vec<CutView> {
    template(p, shape).into_iter().map(|pts| CutView { pts, bloom, ear }).collect()
}

/// A player's freehand scoring: a wobbly swoosh and two quick nicks.
fn freehand(bloom: f32, ear: f32) -> Vec<CutView> {
    let swoosh: Vec<V2> = (0..14)
        .map(|i| {
            let t = i as f32 / 13.0;
            v2(-0.62 + 1.2 * t, 0.18 - 0.42 * t + 0.05 * (t * 9.0).sin())
        })
        .collect();
    let nick1 = vec![v2(-0.42, 0.46), v2(-0.2, 0.52), v2(-0.02, 0.5)];
    let nick2 = vec![v2(0.2, 0.36), v2(0.38, 0.4), v2(0.5, 0.33)];
    vec![
        CutView { pts: swoosh, bloom, ear },
        CutView { pts: nick1, bloom: bloom * 0.7, ear: ear * 0.2 },
        CutView { pts: nick2, bloom: bloom * 0.7, ear: ear * 0.2 },
    ]
}

fn look_view(l: &Look, r: f32, bake: f32, bloom: f32, seed: u32) -> LoafView {
    let (recipe, p, shape, crust, stencil, topping) = *l;
    let cuts = match p {
        Some(p) => cuts(p, shape, bloom, if p == Pattern::Ear { 0.9 } else { 0.35 }),
        None if crust < 0.0 => Vec::new(),
        None => freehand(bloom, 0.8),
    };
    LoafView { shape, recipe, r, bake, spring: 0.85, crust: crust.abs(), cuts, stencil, topping, seed }
}

pub fn render(out: &Path, scale: f32) {
    if std::env::var("BAKERY_BENCH").is_ok() {
        bench();
        return;
    }
    loaves(out, scale);
    states(out, scale);
    jars(out, scale);
    oven_and_treats(out, scale);
}

/// Raw doughs, then a showcase of finished loaves at hero size.
fn loaves(out: &Path, scale: f32) {
    let mut b = Board::new(1600.0, 1560.0, scale * 0.6, Edition::Dawn);
    let raw: [Look; 5] = [
        (Recipe::Country, Some(Pattern::Wheat), Shape::Boule, 0.5, None, None),
        (Recipe::Country, Some(Pattern::Ear), Shape::Batard, 0.5, Some(Stencil::Heart), None),
        (Recipe::WholeWheat, Some(Pattern::Leaf), Shape::Boule, 0.5, None, Some(Topping::Sesame)),
        (Recipe::Country, Some(Pattern::Cross), Shape::Boule, 0.5, Some(Stencil::Star), Some(Topping::Poppy)),
        (Recipe::DarkRye, None, Shape::Boule, 0.5, None, Some(Topping::Oats)),
    ];
    // Lay a row out by each loaf's width so bâtards never collide.
    let row_xs = |row: &[Look], r: f32| -> Vec<f32> {
        let widths: Vec<f32> = row.iter().map(|l| 2.0 * r * 1.1 * l.2.radii().0 + 40.0).collect();
        let total: f32 = widths.iter().sum();
        let mut x = (1600.0 - total) * 0.5;
        widths
            .iter()
            .map(|w| {
                x += w;
                x - w * 0.5
            })
            .collect()
    };
    let xs = row_xs(&raw, 120.0);
    for (i, l) in raw.iter().enumerate() {
        b.put(v2(xs[i], 180.0), |d| loaf_top(d, &look_view(l, 120.0, 0.0, 0.0, 3 + i as u32)));
    }
    let baked: [Look; 10] = [
        (Recipe::Country, Some(Pattern::Wheat), Shape::Boule, 0.55, None, Some(Topping::Sesame)),
        (Recipe::DarkRye, Some(Pattern::Ear), Shape::Batard, 0.85, None, None),
        (Recipe::CranberryWalnut, Some(Pattern::Cross), Shape::Boule, 0.3, Some(Stencil::Heart), None),
        (Recipe::WholeWheat, Some(Pattern::Leaf), Shape::Boule, 0.6, None, Some(Topping::Oats)),
        (Recipe::Olive, Some(Pattern::Ear), Shape::Boule, 0.65, Some(Stencil::Star), None),
        (Recipe::Cheddar, Some(Pattern::Wheat), Shape::Batard, 0.45, None, Some(Topping::Poppy)),
        (Recipe::Country, Some(Pattern::Cross), Shape::Boule, 0.15, Some(Stencil::Sun), None),
        (Recipe::WholeWheat, Some(Pattern::Leaf), Shape::Batard, 0.75, Some(Stencil::Bunny), None),
        (Recipe::Country, None, Shape::Boule, 0.95, None, None),
        (Recipe::Country, None, Shape::Boule, -0.5, Some(Stencil::Heart), Some(Topping::Sesame)),
    ];
    for (row, looks) in baked.chunks(5).enumerate() {
        let xs = row_xs(looks, 112.0);
        for (i, l) in looks.iter().enumerate() {
            let at = v2(xs[i], 530.0 + row as f32 * 330.0);
            b.put(at, |d| loaf_top(d, &look_view(l, 112.0, 1.0, 0.9, 11 + (row * 5 + i) as u32)));
        }
    }
    // Crumb shots at the reveal card's size and blown up.
    for (i, open) in [0.2f32, 0.55, 0.95].iter().enumerate() {
        b.put(v2(170.0 + i as f32 * 250.0, 1400.0), |d| {
            crumb_slice(d, 200.0, 130.0, *open, 0.55, 3 + i as u32)
        });
    }
    b.put(v2(1180.0, 1430.0), |d| crumb_slice(d, 440.0, 286.0, 0.8, 0.7, 5));
    b.save(&out.join("bakery_loaves.png"));
}

/// Intermediate states the screens animate, crust levels and the in-game sizes.
fn states(out: &Path, scale: f32) {
    let mut b = Board::new(1600.0, 1240.0, scale * 0.6, Edition::Dawn);
    let big_ear: Look = (Recipe::Country, Some(Pattern::Ear), Shape::Boule, 0.6, None, None);
    let cross: Look = (Recipe::Country, Some(Pattern::Cross), Shape::Boule, 0.6, Some(Stencil::Heart), None);
    // Toasty bakes 0 → 1 (loaves are r=40 at 1.25× in the oven).
    for i in 0..7 {
        let t = i as f32 / 6.0;
        let at = v2(110.0 + i as f32 * 125.0, 90.0);
        b.put(at, |d| {
            let mut v = look_view(&cross, 50.0, t, 0.8, 21);
            v.crust = t;
            v.spring = t;
            for c in &mut v.cuts {
                c.bloom = t * 0.8;
                c.ear = t * 0.5;
            }
            loaf_top(d, &v)
        });
    }
    // The reveal: bloom and ear open 0 → 1 (r=118 on the card).
    for i in 0..6 {
        let q = i as f32 / 5.0;
        let at = v2(135.0 + i as f32 * 265.0, 330.0);
        b.put(at, |d| {
            let mut v = look_view(&big_ear, 112.0, 1.0, 1.0, 5);
            v.spring = 0.85 * q;
            for c in &mut v.cuts {
                c.bloom *= q;
                c.ear *= q;
            }
            loaf_top(d, &v)
        });
    }
    // Crust levels: blonde, golden, bold.
    for (i, c) in [0.15f32, 0.55, 0.92].iter().enumerate() {
        let wheat: Look = (Recipe::Country, Some(Pattern::Wheat), Shape::Boule, *c, None, None);
        b.put(v2(130.0 + i as f32 * 250.0, 640.0), |d| loaf_top(d, &look_view(&wheat, 105.0, 1.0, 0.85, 8)));
    }
    // Every recipe at the shop shelf size (r=62) and the zine size (r=80).
    for (i, r) in Recipe::ALL.iter().enumerate() {
        let p =
            [Pattern::Ear, Pattern::Cross, Pattern::Wheat, Pattern::Leaf, Pattern::Ear, Pattern::Cross][i];
        let l: Look = (*r, Some(p), Shape::Boule, 0.55, None, None);
        b.put(v2(860.0 + (i % 3) as f32 * 150.0, 580.0 + (i / 3) as f32 * 150.0), |d| {
            loaf_top(d, &look_view(&l, 62.0, 1.0, 0.85, 30 + i as u32))
        });
    }
    for (i, t) in [Topping::Sesame, Topping::Poppy, Topping::Oats].iter().enumerate() {
        let l: Look = (Recipe::Country, Some(Pattern::Ear), Shape::Batard, 0.6, None, Some(*t));
        b.put(v2(1340.0 + (i % 2) as f32 * 0.0, 560.0 + i as f32 * 120.0), |d| {
            loaf_top(d, &look_view(&l, 62.0, 1.0, 0.85, 40 + i as u32))
        });
    }
    // Icon sizes (recipe icons draw loaves at r ≈ 27–50).
    for (i, r) in Recipe::ALL.iter().enumerate() {
        let l: Look = (*r, Some(Pattern::Ear), Shape::Boule, 0.5, None, None);
        b.put(v2(100.0 + i as f32 * 90.0, 950.0), |d| {
            loaf_top(d, &look_view(&l, 36.0, 1.0, 0.8, 50 + i as u32))
        });
        b.put(v2(100.0 + i as f32 * 90.0, 1060.0), |d| {
            loaf_top(d, &look_view(&l, 27.0, 1.0, 0.8, 50 + i as u32))
        });
    }
    // A night page trio at r=64 and the stocked-shelf preview size r=80.
    for (i, s) in [Some(Stencil::Heart), None, Some(Stencil::Sun)].iter().enumerate() {
        let l: Look = (Recipe::Country, Some(Pattern::Cross), Shape::Boule, 0.5, *s, None);
        b.put(v2(700.0 + i as f32 * 190.0, 1010.0), |d| {
            loaf_top(d, &look_view(&l, 80.0, 1.0, 0.85, 60 + i as u32))
        });
    }
    let l: Look = (Recipe::Country, Some(Pattern::Wheat), Shape::Batard, 0.6, None, Some(Topping::Sesame));
    b.put(v2(1360.0, 1010.0), |d| loaf_top(d, &look_view(&l, 80.0, 1.0, 0.85, 70)));
    b.save(&out.join("bakery_states.png"));
}

/// Where the engine prints a starter's name (text rect at 0.8× in the evening).
fn name_guide(d: &mut DrawList, s: f32) {
    let tc = TAPE_CENTER;
    let (w, h) = (120.0 / s, 32.0 / s);
    let r = rect(tc.x - w * 0.5, tc.y - h * 0.5, w, h);
    d.stroke_p(Paint::solid(Ink::Blue, 0.8), 1.2, &rect_poly(r), true);
    // A stand-in word: short key strokes where the name's letters sit.
    for i in 0..7 {
        let x = tc.x - 44.0 + i as f32 * 14.5;
        d.line(Ink::Key, 2.6, &[v2(x, tc.y - 8.0), v2(x + 3.0, tc.y + 8.0)]);
    }
}

fn jars(out: &Path, scale: f32) {
    let mut b = Board::new(1600.0, 900.0, scale * 0.6, Edition::Dusk);
    let jars = [
        JarView { expr: Expr::Content, ..JarView::default() },
        JarView {
            pep: 1.0,
            tang: 0.15,
            rise: 0.98,
            band: 0.28,
            expr: Expr::Excited,
            seed: 4,
            cloth: Ink::Blue,
            ..JarView::default()
        },
        JarView {
            pep: 0.9,
            tang: 0.9,
            rise: 0.8,
            band: 0.28,
            expr: Expr::Happy,
            seed: 6,
            cloth: Ink::Yellow,
            ..JarView::default()
        },
        JarView {
            pep: 0.72,
            tang: 0.4,
            rise: 0.28,
            band: 0.28,
            expr: Expr::Happy,
            seed: 7,
            ..JarView::default()
        },
        JarView {
            pep: 0.3,
            tang: 0.5,
            rise: 0.35,
            band: 0.45,
            expr: Expr::Hungry,
            seed: 8,
            cloth: Ink::Yellow,
            ..JarView::default()
        },
        JarView {
            pep: 0.1,
            tang: 0.6,
            rise: 0.15,
            band: 0.3,
            hooch: true,
            expr: Expr::Sleepy,
            seed: 9,
            cloth: Ink::Blue,
            ..JarView::default()
        },
    ];
    for (i, v) in jars.iter().enumerate() {
        b.put(v2(140.0 + i as f32 * 264.0, 380.0), |d| jar(d, v));
    }
    // Morning shelf (0.42×) and evening shelf (0.8×, with the name guide).
    for (i, v) in jars.iter().enumerate() {
        b.put(v2(90.0 + i as f32 * 80.0, 800.0), |d| d.with(Xf::IDENTITY.scaled(0.42), |d| jar(d, v)));
    }
    for (i, v) in jars.iter().take(4).enumerate() {
        b.put(v2(700.0 + i as f32 * 225.0, 830.0), |d| {
            d.with(Xf::IDENTITY.scaled(0.8), |d| {
                jar(d, v);
                name_guide(d, 0.8);
            })
        });
    }
    b.save(&out.join("bakery_jars.png"));
}

fn oven_and_treats(out: &Path, scale: f32) {
    let mut b = Board::new(1600.0, 1240.0, scale * 0.6, Edition::Dawn);
    let loaves_inside = |d: &mut DrawList, bake: f32| {
        for (i, x) in [-46.0f32, 46.0].iter().enumerate() {
            let v = LoafView {
                r: 40.0,
                bake,
                crust: bake * 0.6,
                spring: bake,
                cuts: cuts(
                    if i == 0 { Pattern::Cross } else { Pattern::Ear },
                    Shape::Boule,
                    bake * 0.8,
                    bake * 0.5,
                ),
                stencil: if i == 0 { Some(Stencil::Heart) } else { None },
                ..LoafView::default()
            };
            d.with(Xf::at(v2(*x, -150.0)), |d| loaf_top(d, &v));
        }
    };
    let ovens: [(OvenView, f32); 4] = [
        (OvenView::default(), -1.0),
        (OvenView { glow: 0.9, steam: 0.5, expr: Expr::Happy, t: 0.4, ..OvenView::default() }, 0.2),
        (OvenView { glow: 0.9, steam: 1.0, expr: Expr::Wow, t: 2.2, ..OvenView::default() }, 0.9),
        (OvenView { glow: 0.3, open: 1.0, expr: Expr::Excited, t: 1.0, ..OvenView::default() }, -1.0),
    ];
    for (i, (v, bake)) in ovens.iter().enumerate() {
        b.put(v2(210.0 + i as f32 * 395.0, 520.0), |d| {
            oven(d, v, |d| {
                if *bake >= 0.0 {
                    loaves_inside(d, *bake)
                }
            })
        });
    }
    // A half-open door.
    b.put(v2(210.0, 1040.0), |d| {
        d.with(Xf::IDENTITY.scaled(0.8), |d| {
            oven(d, &OvenView { glow: 0.6, open: 0.5, expr: Expr::Content, ..OvenView::default() }, |_| {})
        })
    });
    // Treat tray: raw → finished at the tray size, then shop (120), preview (96) and icon (~51).
    for (i, t) in Treat::ALL.iter().enumerate() {
        let x = 540.0 + i as f32 * 360.0;
        b.put(v2(x - 80.0, 870.0), |d| treat_raw(d, *t, 130.0, 1));
        b.put(v2(x + 80.0, 870.0), |d| treat(d, *t, 130.0, 2));
        b.put(v2(x - 110.0, 1080.0), |d| treat(d, *t, 120.0, 3));
        b.put(v2(x + 20.0, 1090.0), |d| treat(d, *t, 96.0, 4));
        b.put(v2(x + 115.0, 1100.0), |d| treat(d, *t, 51.0, 5));
    }
    b.save(&out.join("bakery_oven_treats.png"));
}

/// `BAKERY_BENCH=1 artboard --sheet bakery`: time the rasteriser on the heaviest bakery art
/// at a phone-ish scale (the screens rasterise on the main thread, so this is frame time).
fn bench() {
    use proof_raster::{RasterConfig, rasterize};
    let cfg = RasterConfig { scale: 1.5, ..RasterConfig::default() };
    let hero: Look = (
        Recipe::Cheddar,
        Some(Pattern::Wheat),
        Shape::Boule,
        0.6,
        Some(Stencil::Heart),
        Some(Topping::Sesame),
    );
    let one = |f: &dyn Fn(&mut DrawList)| {
        let mut d = DrawList::new();
        f(&mut d);
        d
    };
    let cases: Vec<(&str, DrawList)> = vec![
        ("loaf r165 (score/reveal)", one(&|d| loaf_top(d, &look_view(&hero, 165.0, 1.0, 0.9, 3)))),
        ("dough r165 (scoring)", one(&|d| loaf_top(d, &look_view(&hero, 165.0, 0.0, 0.0, 3)))),
        ("crumb 200x130", one(&|d| crumb_slice(d, 200.0, 130.0, 0.9, 0.6, 3))),
        ("jar x0.8", one(&|d| d.with(Xf::IDENTITY.scaled(0.8), |d| jar(d, &JarView::default())))),
        (
            "oven x1.25 baking two loaves",
            one(&|d| {
                d.with(Xf::IDENTITY.scaled(1.25), |d| {
                    let v = OvenView { glow: 0.9, steam: 1.0, t: 0.3, ..OvenView::default() };
                    oven(d, &v, |d| {
                        for x in [-46.0f32, 46.0] {
                            d.with(Xf::at(v2(x, -150.0)), |d| {
                                loaf_top(d, &look_view(&hero, 40.0, 0.6, 0.5, 4))
                            });
                        }
                    })
                })
            }),
        ),
        ("bun 130", one(&|d| treat(d, Treat::CinnamonBun, 130.0, 1))),
    ];
    for (name, d) in &cases {
        let n = 5;
        let t0 = std::time::Instant::now();
        for _ in 0..n {
            let _ = rasterize(d, &cfg, None);
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!("{name:30} {:5} cmds {:8.2} ms", d.cmds.len(), ms);
    }
}
