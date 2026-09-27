//! Cast sheets: the regulars behind a counter, every expression, and board-sized heads.
//! Owned by the characters area (`proof_core::art::{critters, face}`).

use crate::Board;
use proof_core::art::Expr;
use proof_core::art::critters::{BUST_W, CritterView, critter};
use proof_core::content::Species;
use proof_core::geom::{Xf, rect, rect_poly, rounded_rect, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

pub const EXPRS: [Expr; 8] = [
    Expr::Content,
    Expr::Happy,
    Expr::Excited,
    Expr::Wow,
    Expr::Proud,
    Expr::Hungry,
    Expr::Sleepy,
    Expr::Hmm,
];

/// A strip of shop counter so we can judge how busts sit behind it.
fn counter(b: &mut Board, y: f32, w: f32) {
    b.put(v2(0.0, y), |d| {
        let slab = rounded_rect(rect(-20.0, 0.0, w + 40.0, 40.0), 12.0);
        let front = rect_poly(rect(-20.0, 30.0, w + 40.0, 60.0));
        d.backing(&front);
        d.fill(Ink::Blue, 0.16, &front);
        d.backing(&slab);
        d.fill(Ink::Pink, 0.75, &slab);
        d.ht(Ink::Yellow, 0.4, &slab);
        d.outline(Ink::Key, 6.0, &slab);
    });
}

pub fn render(out: &Path, scale: f32) {
    // 1. Line-up: all eight at game scale (1.3×) standing behind the counter.
    let mut b = Board::new(1440.0, 1000.0, scale * 0.5, Edition::Daylight);
    for (i, sp) in Species::ALL.iter().enumerate() {
        let x = 180.0 + (i % 4) as f32 * 360.0;
        let y = 440.0 + (i / 4) as f32 * 480.0;
        b.put(v2(x, y), |d| {
            d.with(Xf::IDENTITY.scaled(1.3), |d| {
                critter(d, &CritterView { species: *sp, expr: Expr::Content, t: 0.0 })
            })
        });
    }
    for row in 0..2 {
        counter(&mut b, 430.0 + row as f32 * 480.0, 1440.0);
    }
    b.save(&out.join("cast_lineup.png"));

    // 2. Every expression for every regular at bubble-reading size (0.5×).
    let cell = BUST_W * 0.6;
    let mut b = Board::new(cell * 8.0 + 40.0, 8.0 * 200.0 + 40.0, scale * 0.5, Edition::Daylight);
    for (r, sp) in Species::ALL.iter().enumerate() {
        for (c, e) in EXPRS.iter().enumerate() {
            let at = v2(20.0 + cell * (c as f32 + 0.5), 20.0 + 200.0 * (r as f32 + 1.0) - 10.0);
            b.put(at, |d| {
                d.with(Xf::IDENTITY.scaled(0.5), |d| {
                    critter(d, &CritterView { species: *sp, expr: *e, t: 0.0 })
                })
            });
        }
    }
    b.save(&out.join("cast_expressions.png"));

    // 3. Board-sized (≈0.35×) regulars as the evening "Tomorrow" board shows them.
    let mut b = Board::new(8.0 * 130.0 + 40.0, 260.0, scale, Edition::Dusk);
    for (i, sp) in Species::ALL.iter().enumerate() {
        let at = v2(85.0 + i as f32 * 130.0, 220.0);
        b.put(at, |d| {
            d.with(Xf::IDENTITY.scaled(0.36), |d| {
                critter(d, &CritterView { species: *sp, expr: Expr::Happy, t: 0.0 })
            })
        });
    }
    b.save(&out.join("cast_small.png"));
}
