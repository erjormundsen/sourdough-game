//! Bakery sheets: dough, loaves, crumb, starter jars, Toasty and treats.
//! Owned by the bakery-objects area (`proof_core::art::{bread, jar, oven, treats}`).

use crate::Board;
use proof_core::art::Expr;
use proof_core::art::bread::{CutView, LoafView, crumb_slice, loaf_top};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::oven::{OvenView, oven};
use proof_core::art::treats::{treat, treat_raw};
use proof_core::content::{Pattern, Recipe, Shape, Stencil, Topping, Treat};
use proof_core::geom::{Xf, v2};
use proof_core::ink::{Edition, Ink};
use proof_core::scoring::template;
use std::path::Path;

/// (recipe, pattern, shape, crust, stencil, topping) for a showcase loaf.
type Look = (Recipe, Pattern, Shape, f32, Option<Stencil>, Option<Topping>);

pub fn cuts(p: Pattern, shape: Shape, bloom: f32, ear: f32) -> Vec<CutView> {
    template(p, shape).into_iter().map(|pts| CutView { pts, bloom, ear }).collect()
}

pub fn render(out: &Path, scale: f32) {
    // 1. Loaves: raw → baked, every recipe, pattern, crust level, stencil and topping.
    let mut b = Board::new(1440.0, 1560.0, scale * 0.6, Edition::Dawn);
    let raw =
        |p: Pattern, shape: Shape, stencil: Option<Stencil>, topping: Option<Topping>, seed: u32| LoafView {
            shape,
            bake: 0.0,
            cuts: cuts(p, shape, 0.0, 0.0),
            stencil,
            topping,
            seed,
            ..LoafView::default()
        };
    b.put(v2(190.0, 190.0), |d| loaf_top(d, &raw(Pattern::Wheat, Shape::Boule, None, None, 7)));
    b.put(v2(540.0, 190.0), |d| {
        loaf_top(d, &raw(Pattern::Ear, Shape::Batard, Some(Stencil::Heart), None, 3))
    });
    b.put(v2(900.0, 190.0), |d| {
        loaf_top(d, &raw(Pattern::Leaf, Shape::Boule, None, Some(Topping::Sesame), 9))
    });
    b.put(v2(1250.0, 190.0), |d| {
        loaf_top(d, &raw(Pattern::Cross, Shape::Boule, Some(Stencil::Star), None, 5))
    });
    let looks: [Look; 8] = [
        (Recipe::Country, Pattern::Wheat, Shape::Boule, 0.55, None, Some(Topping::Sesame)),
        (Recipe::DarkRye, Pattern::Ear, Shape::Batard, 0.85, None, None),
        (Recipe::CranberryWalnut, Pattern::Cross, Shape::Boule, 0.2, Some(Stencil::Heart), None),
        (Recipe::WholeWheat, Pattern::Leaf, Shape::Boule, 0.6, None, Some(Topping::Oats)),
        (Recipe::Olive, Pattern::Ear, Shape::Boule, 0.65, Some(Stencil::Star), None),
        (Recipe::Cheddar, Pattern::Wheat, Shape::Batard, 0.45, None, Some(Topping::Poppy)),
        (Recipe::Country, Pattern::Cross, Shape::Boule, 0.3, Some(Stencil::Sun), None),
        (Recipe::WholeWheat, Pattern::Leaf, Shape::Batard, 0.75, Some(Stencil::Bunny), None),
    ];
    for (i, (recipe, p, shape, crust, stencil, topping)) in looks.iter().enumerate() {
        let at = v2(190.0 + (i % 4) as f32 * 355.0, 560.0 + (i / 4) as f32 * 370.0);
        let v = LoafView {
            shape: *shape,
            recipe: *recipe,
            crust: *crust,
            cuts: cuts(*p, *shape, 0.9, if *p == Pattern::Ear { 1.0 } else { 0.35 }),
            stencil: *stencil,
            topping: *topping,
            seed: 11 + i as u32,
            ..LoafView::default()
        };
        b.put(at, |d| loaf_top(d, &v));
    }
    // Crumb shots: tight → open.
    for (i, open) in [0.2f32, 0.55, 0.95].iter().enumerate() {
        b.put(v2(240.0 + i as f32 * 480.0, 1400.0), |d| {
            crumb_slice(d, 360.0, 230.0, *open, 0.55, 3 + i as u32)
        });
    }
    b.save(&out.join("bakery_loaves.png"));

    // 2. Starter jars: moods and microbiomes, plus the small shelf size.
    let mut b = Board::new(1440.0, 760.0, scale * 0.6, Edition::Dusk);
    let jars = [
        JarView { expr: Expr::Content, ..JarView::default() },
        JarView {
            pep: 1.0,
            tang: 0.15,
            rise: 0.95,
            band: 0.35,
            expr: Expr::Excited,
            seed: 4,
            cloth: Ink::Blue,
            ..JarView::default()
        },
        JarView {
            pep: 0.9,
            tang: 0.9,
            rise: 0.8,
            band: 0.3,
            expr: Expr::Happy,
            seed: 6,
            cloth: Ink::Yellow,
            ..JarView::default()
        },
        JarView {
            pep: 0.3,
            tang: 0.5,
            rise: 0.35,
            band: 0.45,
            expr: Expr::Hungry,
            seed: 8,
            ..JarView::default()
        },
        JarView {
            pep: 0.1,
            tang: 0.6,
            rise: 0.2,
            band: 0.5,
            hooch: true,
            expr: Expr::Sleepy,
            seed: 9,
            cloth: Ink::Blue,
            ..JarView::default()
        },
    ];
    for (i, v) in jars.iter().enumerate() {
        b.put(v2(150.0 + i as f32 * 285.0, 340.0), |d| jar(d, v));
    }
    for (i, v) in jars.iter().take(3).enumerate() {
        b.put(v2(120.0 + i as f32 * 90.0, 700.0), |d| d.with(Xf::IDENTITY.scaled(0.42), |d| jar(d, v)));
    }
    for (i, v) in jars.iter().take(3).enumerate() {
        b.put(v2(560.0 + i as f32 * 170.0, 720.0), |d| d.with(Xf::IDENTITY.scaled(0.8), |d| jar(d, v)));
    }
    b.save(&out.join("bakery_jars.png"));

    // 3. Toasty (closed, baking with loaves, door open) and treats (raw → finished).
    let mut b = Board::new(1440.0, 1000.0, scale * 0.6, Edition::Dawn);
    let loaves_inside = |d: &mut proof_core::draw::DrawList, bake: f32| {
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
                ..LoafView::default()
            };
            d.with(Xf::at(v2(*x, -150.0)), |d| loaf_top(d, &v));
        }
    };
    b.put(v2(250.0, 470.0), |d| oven(d, &OvenView::default(), |_| {}));
    b.put(v2(720.0, 470.0), |d| {
        oven(d, &OvenView { glow: 0.9, steam: 1.0, expr: Expr::Happy, t: 0.4, ..OvenView::default() }, |d| {
            loaves_inside(d, 0.6)
        })
    });
    b.put(v2(1190.0, 470.0), |d| {
        oven(d, &OvenView { glow: 0.4, open: 1.0, expr: Expr::Wow, ..OvenView::default() }, |_| {})
    });
    for (i, t) in Treat::ALL.iter().enumerate() {
        let x = 170.0 + i as f32 * 460.0;
        b.put(v2(x, 800.0), |d| treat_raw(d, *t, 150.0, 1));
        b.put(v2(x + 200.0, 800.0), |d| treat(d, *t, 150.0, 2));
        b.put(v2(x + 100.0, 930.0), |d| treat(d, *t, 96.0, 3));
    }
    b.save(&out.join("bakery_oven_treats.png"));
}
