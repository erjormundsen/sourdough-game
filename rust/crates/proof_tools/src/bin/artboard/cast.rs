//! Cast sheets: the regulars behind the shop counter, every expression, board-sized cards and
//! head portraits. Owned by the characters area (`proof_core::art::{critters, face}`).

use crate::Board;
use proof_core::art::critters::{BUST_W, CritterView, HEAD_R, critter, critter_head};
use proof_core::art::{Expr, face};
use proof_core::content::Species;
use proof_core::geom::{V2, Xf, circle, rect, rect_poly, rounded_rect, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

pub const EXPRS: [Expr; 8] = Expr::ALL;

/// Shop scale and anchor: busts print at 1.3× with their origin 40 units under the counter top.
const SHOP_SCALE: f32 = 1.3;
const SHOP_DROP: f32 = 40.0;

/// The shop's display-case counter (same proportions as `scenes::counter_front`), top at `y`.
fn counter(b: &mut Board, y: f32, w: f32) {
    b.put(v2(0.0, y), |d| {
        let body = rect_poly(rect(-30.0, 30.0, w + 60.0, 150.0));
        d.backing(&body);
        d.fill(Ink::Blue, 0.16, &body);
        d.ht(Ink::Yellow, 0.18, &body);
        let slab = rounded_rect(rect(-30.0, 0.0, w + 60.0, 44.0), 14.0);
        d.backing(&slab);
        d.fill(Ink::Pink, 0.75, &slab);
        d.ht(Ink::Yellow, 0.4, &slab);
        d.outline(Ink::Key, 5.5, &slab);
    });
}

fn wall(b: &mut Board, y0: f32, h: f32, w: f32) {
    b.put(V2::ZERO, |d| d.fill(Ink::Pink, 0.12, &rect_poly(rect(0.0, y0, w, h))));
}

pub fn render(out: &Path, scale: f32) {
    // 1. Line-up: all eight at shop scale standing behind the counter, as in the shop.
    let (cw, rh) = (360.0, 520.0);
    let mut b = Board::new(cw * 4.0, rh * 2.0, scale * 0.5, Edition::Daylight);
    wall(&mut b, 0.0, rh * 2.0, cw * 4.0);
    for (i, sp) in Species::ALL.iter().enumerate() {
        let x = cw * 0.5 + (i % 4) as f32 * cw;
        let y = 470.0 + (i / 4) as f32 * rh;
        b.put(v2(x, y), |d| {
            d.with(Xf::IDENTITY.scaled(SHOP_SCALE), |d| {
                critter(d, &CritterView { species: *sp, expr: Expr::Content, t: 0.0 })
            })
        });
    }
    for row in 0..2 {
        counter(&mut b, 470.0 - SHOP_DROP + row as f32 * rh, cw * 4.0);
    }
    b.save(&out.join("cast_lineup.png"));

    // 2. Every expression for every regular at bubble-reading size (0.5×).
    let cell = BUST_W * 0.62;
    let mut b = Board::new(cell * 8.0 + 40.0, 8.0 * 200.0 + 40.0, scale * 0.5, Edition::Daylight);
    for (r, sp) in Species::ALL.iter().enumerate() {
        for (c, e) in EXPRS.iter().enumerate() {
            let at = v2(20.0 + cell * (c as f32 + 0.5), 20.0 + 200.0 * (r as f32 + 1.0) - 14.0);
            b.put(at, |d| {
                d.with(Xf::IDENTITY.scaled(0.5), |d| {
                    critter(d, &CritterView { species: *sp, expr: *e, t: 0.0 })
                })
            });
        }
    }
    b.save(&out.join("cast_expressions.png"));

    // 3. Board cards (≈0.36×) exactly as the evening Tomorrow board clips them, then the
    //    head portraits at the same print size.
    let mut b = Board::new(8.0 * 170.0 + 40.0, 360.0, scale, Edition::Dusk);
    for (i, sp) in Species::ALL.iter().enumerate() {
        let cx = 100.0 + i as f32 * 170.0;
        let cy = 90.0;
        b.put(V2::ZERO, |d| {
            let card = rounded_rect(rect(cx - 76.0, cy - 50.0, 152.0, 100.0), 12.0);
            d.backing(&card);
            d.outline(Ink::Key, 2.5, &card);
            d.clipped(&card, |d| {
                d.with(Xf::at(v2(cx - 36.0, cy + 70.0)).scaled(0.36), |d| {
                    critter(d, &CritterView { species: *sp, expr: Expr::Content, t: 0.0 })
                });
            });
            let card2 = rounded_rect(rect(cx - 76.0, cy + 120.0, 152.0, 100.0), 12.0);
            d.backing(&card2);
            d.outline(Ink::Key, 2.5, &card2);
            d.with(Xf::at(v2(cx - 34.0, cy + 170.0)).scaled(0.42), |d| critter_head(d, *sp, Expr::Happy));
            d.outline(Ink::Blue, 1.0, &circle(v2(cx - 34.0, cy + 170.0), HEAD_R * 0.42));
        });
    }
    b.save(&out.join("cast_small.png"));

    // 4. Head portraits: every regular and expression at 0.6×, plus face grammar swatches.
    let cell = 130.0;
    let mut b = Board::new(cell * 8.0 + 40.0, cell * 9.0 + 40.0, scale * 0.75, Edition::Daylight);
    for (r, sp) in Species::ALL.iter().enumerate() {
        for (c, e) in EXPRS.iter().enumerate() {
            let at = v2(20.0 + cell * (c as f32 + 0.5), 20.0 + cell * (r as f32 + 0.5));
            b.put(at, |d| d.with(Xf::IDENTITY.scaled(0.6), |d| critter_head(d, *sp, *e)));
        }
    }
    for (c, e) in EXPRS.iter().enumerate() {
        let at = v2(20.0 + cell * (c as f32 + 0.5), 20.0 + cell * 8.5);
        b.put(at, |d| {
            let disc = circle(V2::ZERO, 52.0);
            d.backing(&disc);
            d.fill(Ink::Yellow, 0.35, &disc);
            d.outline(Ink::Key, 4.0, &disc);
            face::face(d, v2(0.0, 4.0), 84.0, *e, V2::ZERO);
        });
    }
    b.save(&out.join("cast_heads.png"));

    // 5. The idle loop (12 frames at the shop's 3 redraws a second) for a few regulars.
    let cell = 150.0;
    let who = [Species::Bunny, Species::Cat, Species::Otter];
    let mut b =
        Board::new(cell * 12.0 + 40.0, 220.0 * who.len() as f32 + 20.0, scale * 0.75, Edition::Daylight);
    for (r, sp) in who.iter().enumerate() {
        for f in 0..12 {
            let at = v2(20.0 + cell * (f as f32 + 0.5), 210.0 + 220.0 * r as f32);
            b.put(at, |d| {
                d.with(Xf::IDENTITY.scaled(0.6), |d| {
                    critter(d, &CritterView { species: *sp, expr: Expr::Content, t: f as f32 / 3.0 })
                })
            });
        }
    }
    b.save(&out.join("cast_idle.png"));
}
