//! Scene sheets: full backdrops with counters, printed props, every icon, and the app icon.
//! Owned by the scenes/props/UI area (`proof_core::art::{scenes, props, icons}`).

use crate::Board;
use proof_core::art::icons::{Icon, icon};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::props;
use proof_core::art::scenes::{Backdrop, backdrop, counter_front};
use proof_core::art::{Expr, twinkle};
use proof_core::bake::CrustLevel;
use proof_core::content::{Flour, Pattern, Recipe, Shape, Stencil, Topping, Treat};
use proof_core::customer::Want;
use proof_core::geom::{V2, Xf, rect, rounded_rect, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

pub fn render(out: &Path, scale: f32) {
    // 1. Backdrops (each in its own edition) at phone height and at a tall-phone height.
    for (name, h) in [("scenes_backdrops", 1280.0f32), ("scenes_backdrops_tall", 1560.0)] {
        let mut b = Board::new(2160.0, h, scale * 0.5, Edition::Daylight);
        for (i, (bd, ed)) in [
            (Backdrop::Bakehouse, Edition::Dawn),
            (Backdrop::Shopfront, Edition::Daylight),
            (Backdrop::Pantry, Edition::Dusk),
        ]
        .into_iter()
        .enumerate()
        {
            b.palette = ed.palette();
            b.put(v2(i as f32 * 720.0, 0.0), |d| {
                backdrop(d, bd, h);
                counter_front(d, bd, h);
            });
        }
        b.save(&out.join(format!("{name}.png")));
    }

    // 2. Printed props and UI pieces.
    let mut b = Board::new(1440.0, 1100.0, scale * 0.6, Edition::Daylight);
    b.put(v2(200.0, 150.0), |d| props::ticket(d, 300.0, 200.0, Ink::Pink));
    b.put(v2(560.0, 150.0), |d| {
        d.with(Xf::IDENTITY.rotated(-0.25), |d| props::stamp(d, 90.0, 3, Ink::Pink, 4))
    });
    b.put(v2(900.0, 110.0), |d| props::button(d, 300.0, 84.0, Ink::Pink, false));
    b.put(v2(900.0, 220.0), |d| props::button(d, 300.0, 84.0, Ink::Yellow, true));
    b.put(v2(1250.0, 160.0), |d| props::round_button(d, 40.0, Ink::Yellow, true));
    b.put(v2(1250.0, 160.0), |d| icon(d, Icon::Pattern(Pattern::Wheat), V2::ZERO, 12.0));
    b.put(v2(360.0, 460.0), |d| props::bubble(d, 460.0, 250.0, v2(-20.0, 190.0)));
    b.put(v2(360.0, 500.0), |d| {
        icon(d, Icon::Want(Want::Tangy), v2(-60.0, 0.0), 44.0);
        icon(d, Icon::Want(Want::Pattern(Pattern::Leaf)), v2(60.0, 0.0), 44.0);
    });
    b.put(v2(900.0, 430.0), |d| props::tape(d, 400.0, 64.0, Ink::Yellow));
    for (i, f) in Flour::ALL.iter().enumerate() {
        b.put(v2(820.0 + i as f32 * 170.0, 700.0), |d| props::flour_bag(d, *f));
    }
    b.put(v2(1350.0, 620.0), |d| props::banneton(d, Shape::Boule, 60.0, true));
    b.put(v2(1320.0, 760.0), |d| props::banneton(d, Shape::Batard, 50.0, false));
    b.put(v2(360.0, 900.0), |d| props::ticket(d, 560.0, 70.0, Ink::Yellow));
    b.put(v2(900.0, 930.0), |d| {
        props::heart_icon(d, v2(0.0, 0.0), 50.0);
        props::coin(d, v2(90.0, 0.0), 26.0);
        twinkle(d, v2(170.0, 0.0), 20.0, Ink::Yellow);
        d.with(Xf::at(v2(320.0, 0.0)), |d| props::burst(d, 60.0, 8, 3));
    });
    b.save(&out.join("scenes_props.png"));

    // 3. Every icon on a grid (as buttons and order bubbles show them).
    let mut icons: Vec<Icon> = Vec::new();
    icons.extend(Stencil::ALL.map(Icon::Stencil));
    icons.extend(Topping::ALL.map(Icon::Topping));
    icons.extend(Pattern::ALL.map(Icon::Pattern));
    icons.extend(Flour::ALL.map(Icon::Flour));
    icons.extend(Recipe::ALL.map(Icon::Recipe));
    icons.extend(Treat::ALL.map(Icon::Treat));
    icons.extend([Shape::Boule, Shape::Batard].map(Icon::Shape));
    icons.extend([Want::Tangy, Want::Mild, Want::BigEar, Want::Surprise, Want::AnyLoaf].map(Icon::Want));
    icons.extend(CrustLevel::ALL.map(|c| Icon::Want(Want::Crust(c))));
    icons.extend([
        Icon::Freehand,
        Icon::None,
        Icon::Undo,
        Icon::Check,
        Icon::Coin,
        Icon::Heart,
        Icon::Star,
        Icon::Moon,
        Icon::Book,
        Icon::Back,
        Icon::Bowl,
        Icon::Oven,
        Icon::Shop,
        Icon::Jar,
        Icon::Fridge,
    ]);
    let cols = 8;
    let rows = icons.len().div_ceil(cols);
    let mut b = Board::new(cols as f32 * 110.0 + 20.0, rows as f32 * 110.0 + 20.0, scale, Edition::Daylight);
    for (i, ic) in icons.iter().enumerate() {
        let c = v2(65.0 + (i % cols) as f32 * 110.0, 65.0 + (i / cols) as f32 * 110.0);
        b.put(c, |d| {
            let plate = rounded_rect(rect(-48.0, -48.0, 96.0, 96.0), 18.0);
            d.stroke_p(proof_core::draw::Paint::solid(Ink::Key, 0.25), 1.5, &plate, true);
            icon(d, *ic, V2::ZERO, 30.0);
        });
    }
    b.save(&out.join("scenes_icons.png"));

    // 4. App icon: Bubbles on a halftone tile.
    let mut b = Board::new(512.0, 512.0, 1.0, Edition::Daylight);
    b.style.offsets = [v2(1.6, -1.2), v2(-1.2, 0.9), v2(0.9, 1.4), V2::ZERO];
    b.put(V2::ZERO, |d| {
        let tile = rounded_rect(rect(0.0, 0.0, 512.0, 512.0), 96.0);
        d.fill(Ink::Yellow, 0.55, &tile);
        d.fill_p(proof_core::draw::Paint::coarse(Ink::Pink, 0.3).add(), &tile);
        d.with(Xf::at(v2(256.0, 470.0)).scaled(1.62), |d| {
            jar(
                d,
                &JarView {
                    pep: 0.95,
                    tang: 0.45,
                    rise: 0.85,
                    band: 0.35,
                    expr: Expr::Excited,
                    seed: 4,
                    ..JarView::default()
                },
            )
        });
        twinkle(d, v2(92.0, 118.0), 28.0, Ink::Yellow);
        twinkle(d, v2(420.0, 170.0), 20.0, Ink::Pink);
    });
    b.save(&out.join("icon.png"));
}
