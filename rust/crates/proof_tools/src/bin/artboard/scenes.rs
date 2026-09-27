//! Scene sheets: full backdrops with counters (and the gameplay objects standing where the
//! screens put them), the printed UI kit, every icon, and the app icon.
//! Owned by the scenes/props/UI area (`proof_core::art::{scenes, props, icons}`).

use crate::Board;
use proof_core::art::bread::{LoafView, loaf_top};
use proof_core::art::critters::{CritterView, critter};
use proof_core::art::icons::{Icon, icon};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::props::{self, Look};
use proof_core::art::scenes::{
    self, Backdrop, Dressing, backdrop_dressed, case_cols, case_rows, counter_front, counter_top, jar_scale,
    jar_shelf, jar_slots,
};
use proof_core::art::treats::treat;
use proof_core::art::{Expr, twinkle};
use proof_core::bake::CrustLevel;
use proof_core::content::{Flour, Pattern, Recipe, Shape, Species, Stencil, Topping, Treat};
use proof_core::customer::Want;
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, rect, rounded_rect, v2};
use proof_core::ink::{Edition, Ink};
use std::path::Path;

/// One backdrop with its counter and stand-in gameplay objects, drawn at the origin.
fn stage(d: &mut DrawList, bd: Backdrop, h: f32, jars: usize) {
    let slots = jar_slots(bd, jars);
    let dress = Dressing { jars: if bd == Backdrop::Shopfront { Vec::new() } else { slots.clone() } };
    backdrop_dressed(d, bd, h, &dress);
    match bd {
        Backdrop::Bakehouse => {
            let y = jar_shelf(bd, h);
            for (i, x) in slots.iter().enumerate() {
                let v = JarView { seed: i as u32 + 1, rise: 0.7, ..JarView::default() };
                d.with(Xf::at(v2(*x, y - 7.0)).scaled(jar_scale(bd)), |d| jar(d, &v));
            }
            let lay = scenes::bake_layout(h);
            d.with(Xf::at(lay.dough), |d| props::bread_board(d, lay.board_r));
            let v = LoafView { bake: 0.0, spring: 0.0, r: 165.0, ..LoafView::default() };
            d.with(Xf::at(lay.dough), |d| loaf_top(d, &v));
            let (bs, br) = scenes::banneton_slots(h, 2);
            let span = bs.last().map(|p| p.y).unwrap_or(0.0) - bs.first().map(|p| p.y).unwrap_or(0.0);
            d.with(Xf::at(v2(84.0, lay.dough.y)), |d| props::couche(d, 96.0, span + 110.0));
            for (i, p) in bs.iter().enumerate() {
                d.with(Xf::at(*p), |d| props::banneton(d, Shape::Boule, br, i > 0));
            }
            let dress = vec![Icon::None, Icon::Stencil(Stencil::Heart), Icon::Topping(Topping::Sesame)];
            let score =
                vec![Icon::Freehand, Icon::Pattern(Pattern::Ear), Icon::Pattern(Pattern::Cross), Icon::Undo];
            for (row, icons) in [dress, score].iter().enumerate() {
                let y = lay.trays[row];
                let (tw, slots) = props::tray_layout(icons.len(), 32.0, if row == 1 { 1 } else { 0 });
                d.with(Xf::at(v2(360.0, y)), |d| {
                    d.with(Xf::at(v2(-tw * 0.5 + 14.0, -44.0)), |d| props::tab(d, 100.0, 28.0, Ink::Pink));
                    props::tray(d, tw, 88.0, &slots, 32.0);
                    for (i, ic) in icons.iter().enumerate() {
                        let look = Look { selected: i == 1, ..Look::default() };
                        let ink = if *ic == Icon::Undo { Ink::Blue } else { Ink::Yellow };
                        d.with(Xf::at(slots[i]), |d| props::round_button_ex(d, 32.0, ink, look));
                        icon(d, *ic, slots[i] + v2(0.0, -1.0), 19.0);
                    }
                });
            }
        }
        Backdrop::Shopfront => {
            let stand = v2(360.0, counter_top(bd, h) + 40.0);
            d.with(Xf::at(stand).scaled(1.3), |d| {
                critter(d, &CritterView { species: Species::Bunny, expr: Expr::Happy, t: 0.0 })
            });
        }
        Backdrop::Pantry => {
            let y = jar_shelf(bd, h);
            for (i, x) in slots.iter().enumerate() {
                let v = JarView { seed: i as u32 + 1, ..JarView::default() };
                d.with(Xf::at(v2(*x, y - 7.0)).scaled(jar_scale(bd)), |d| jar(d, &v));
            }
        }
    }
    counter_front(d, bd, h);
    if bd == Backdrop::Shopfront {
        let rows = case_rows(h);
        for (i, x) in case_cols().iter().enumerate() {
            let v = LoafView { r: 62.0, seed: i as u32 + 3, ..LoafView::default() };
            d.with(Xf::at(v2(*x, rows[0])), |d| loaf_top(d, &v));
            let t = Treat::ALL[i % 3];
            d.with(Xf::at(v2(*x, rows[1])), |d| treat(d, t, 120.0, 1));
        }
        d.with(Xf::at(v2(606.0, counter_top(bd, h) + 6.0)), |d| props::sign(d, 158.0, 64.0, Look::default()));
    }
    if bd == Backdrop::Bakehouse {
        let lay = scenes::bake_layout(h);
        d.with(Xf::at(v2(360.0, lay.bar)), |d| props::button(d, 340.0, 80.0, Ink::Pink, false));
    }
    props::masthead(d, 720.0, 96.0, 0.0);
    if bd != Backdrop::Shopfront {
        d.with(Xf::at(v2(360.0, 121.0)), |d| props::ribbon(d, 560.0, 42.0, Ink::Pink));
    }
}

/// Print how long each backdrop and counter takes to rasterise at phone scales (they print
/// on every screen change). Run with `PROOF_TIMINGS=1`.
fn timings() {
    for scale in [1.5f32, 2.0] {
        let cfg = proof_raster::RasterConfig { scale, ..proof_raster::RasterConfig::default() };
        let mut base = DrawList::new();
        base.fill(Ink::Yellow, 0.2, &proof_core::geom::rect_poly(rect(-12.0, -40.0, 744.0, 1600.0)));
        let t0 = std::time::Instant::now();
        let _ = proof_raster::rasterize(&base, &cfg, None);
        println!("raster x{scale}: one full-screen rect {:?}", t0.elapsed());
        for bd in [Backdrop::Bakehouse, Backdrop::Shopfront, Backdrop::Pantry] {
            let mut d = DrawList::new();
            backdrop_dressed(&mut d, bd, 1560.0, &Dressing { jars: jar_slots(bd, 1) });
            let mut f = DrawList::new();
            counter_front(&mut f, bd, 1560.0);
            let t0 = std::time::Instant::now();
            let _ = proof_raster::rasterize(&d, &cfg, None);
            let t1 = t0.elapsed();
            let _ = proof_raster::rasterize(&f, &cfg, None);
            println!(
                "raster x{scale} {bd:?}: backdrop {t1:?} ({} cmds), counter {:?} ({} cmds)",
                d.cmds.len(),
                t0.elapsed() - t1,
                f.cmds.len()
            );
        }
    }
}

pub fn render(out: &Path, scale: f32) {
    if std::env::var("PROOF_TIMINGS").is_ok() {
        timings();
    }
    // 1. Backdrops (each in its own edition) at phone height and at a tall-phone height.
    for (name, h) in [("scenes_backdrops", 1280.0f32), ("scenes_backdrops_tall", 1560.0)] {
        let mut b = Board::new(2160.0, h, scale * 0.5, Edition::Daylight);
        for (i, (bd, ed, jars)) in [
            (Backdrop::Bakehouse, Edition::Dawn, 1),
            (Backdrop::Shopfront, Edition::Daylight, 0),
            (Backdrop::Pantry, Edition::Dusk, if h > 1300.0 { 3 } else { 1 }),
        ]
        .into_iter()
        .enumerate()
        {
            b.palette = ed.palette();
            let mut d = DrawList::new();
            d.with(Xf::at(v2(i as f32 * 720.0, 0.0)), |d| stage(d, bd, h, jars));
            // Each panel is clipped to its own column so walls never bleed.
            let mut clipped = DrawList::new();
            clipped.clipped(&proof_core::geom::rect_poly(rect(i as f32 * 720.0, 0.0, 720.0, h)), |c| {
                c.append(&d)
            });
            b.put_list(&clipped);
        }
        b.save(&out.join(format!("{name}.png")));
    }

    // 2. The printed UI kit.
    for (name, ed) in [("scenes_props", Edition::Daylight), ("scenes_props_dusk", Edition::Dusk)] {
        let mut b = Board::new(1440.0, 1500.0, scale * 0.6, ed);
        b.put(v2(190.0, 150.0), |d| props::ticket(d, 300.0, 200.0, Ink::Pink));
        b.put(v2(540.0, 150.0), |d| {
            d.with(Xf::IDENTITY.rotated(-0.25), |d| props::stamp(d, 90.0, 3, Ink::Pink, 4))
        });
        b.put(v2(890.0, 100.0), |d| props::button(d, 300.0, 84.0, Ink::Pink, false));
        b.put(v2(890.0, 210.0), |d| {
            props::button_ex(d, 300.0, 84.0, Ink::Pink, Look { pressed: true, ..Look::default() })
        });
        b.put(v2(890.0, 320.0), |d| props::button_ex(d, 240.0, 72.0, Ink::Blue, Look::default()));
        b.put(v2(1230.0, 100.0), |d| props::button_ex(d, 240.0, 72.0, Ink::Yellow, Look::default()));
        b.put(v2(1230.0, 210.0), |d| {
            props::button_ex(d, 240.0, 72.0, Ink::Blue, Look { selected: true, ..Look::default() })
        });
        b.put(v2(1230.0, 320.0), |d| {
            props::button_ex(d, 240.0, 72.0, Ink::Pink, Look { disabled: true, ..Look::default() })
        });
        // A tool tray with round buttons.
        let slots: Vec<V2> = (0..5).map(|i| v2(-160.0 + i as f32 * 80.0, 0.0)).collect();
        b.put(v2(360.0, 470.0), |d| {
            d.with(Xf::at(v2(-296.0, -42.0)), |d| props::tab(d, 96.0, 30.0, Ink::Pink));
            props::tray(d, 600.0, 92.0, &slots, 32.0);
            for (i, s) in slots.iter().enumerate() {
                let look = Look { selected: i == 1, pressed: i == 3, ..Look::default() };
                d.with(Xf::at(*s), |d| props::round_button_ex(d, 32.0, Ink::Yellow, look));
                let lift = props::round_lift(32.0, look.pressed);
                let ic = [
                    Icon::None,
                    Icon::Stencil(Stencil::Heart),
                    Icon::Topping(Topping::Sesame),
                    Icon::Pattern(Pattern::Ear),
                    Icon::Undo,
                ][i];
                icon(d, ic, *s + v2(0.0, lift - 1.0), 19.0);
            }
        });
        b.put(v2(360.0, 720.0), |d| props::bubble(d, 440.0, 230.0, v2(-40.0, 180.0)));
        b.put(v2(360.0, 750.0), |d| {
            icon(d, Icon::Want(Want::Tangy), v2(-60.0, 0.0), 44.0);
            icon(d, Icon::Want(Want::Pattern(Pattern::Leaf)), v2(60.0, 0.0), 44.0);
        });
        b.put(v2(1000.0, 480.0), |d| props::ribbon(d, 420.0, 46.0, Ink::Pink));
        b.put(v2(1000.0, 580.0), |d| props::slip(d, 520.0, 62.0, Ink::Yellow));
        b.put(v2(1000.0, 680.0), |d| props::tape(d, 300.0, 44.0, Ink::Yellow));
        for (i, f) in Flour::ALL.iter().enumerate() {
            b.put(v2(830.0 + i as f32 * 150.0, 940.0), |d| props::flour_bag(d, *f));
        }
        b.put(v2(1320.0, 800.0), |d| props::banneton(d, Shape::Boule, 50.0, true));
        b.put(v2(1320.0, 920.0), |d| props::banneton(d, Shape::Batard, 40.0, false));
        b.put(v2(190.0, 1000.0), |d| props::sign(d, 160.0, 72.0, Look::default()));
        b.put(v2(430.0, 1000.0), |d| props::cooling_rack(d, 220.0, 90.0));
        b.put(v2(190.0, 1130.0), |d| props::sheet_pan(d, 300.0, 120.0));
        b.put(v2(500.0, 1120.0), |d| props::couche(d, 200.0, 70.0));
        b.put(v2(820.0, 1100.0), |d| {
            props::cork_board(d, 300.0, 160.0);
            d.with(Xf::at(v2(-70.0, 4.0)).rotated(-0.04), |d| props::pinned_card(d, 110.0, 80.0, None, 1));
            d.with(Xf::at(v2(60.0, 10.0)).rotated(0.05), |d| {
                props::pinned_card(d, 110.0, 80.0, Some(Ink::Pink), 2)
            });
        });
        b.put(v2(1120.0, 1060.0), |d| props::hanging_tag(d, 120.0, 44.0, 20.0, Ink::Pink));
        b.put(v2(1300.0, 1080.0), |d| props::grandma(d, V2::ZERO, 60.0));
        b.put(v2(190.0, 1300.0), |d| props::photo(d, 170.0, 190.0, Ink::Yellow));
        b.put(v2(360.0, 1300.0), |d| props::rosette(d, V2::ZERO, 28.0));
        b.put(v2(420.0, 1300.0), |d| props::meter(d, 140.0, 16.0, 0.6, Ink::Pink));
        b.put(v2(700.0, 1300.0), |d| props::chip(d, 150.0, 48.0));
        b.put(v2(640.0, 1300.0), |d| props::coin(d, V2::ZERO, 18.0));
        b.put(v2(1000.0, 1330.0), |d| props::crust_gauge(d, 420.0, 0.62));
        b.put(v2(1300.0, 1400.0), |d| {
            props::bowl_back(d, 100.0);
            props::bowl_front(d, 100.0);
        });
        b.put(v2(820.0, 1440.0), |d| {
            props::heart_icon(d, v2(0.0, 0.0), 50.0);
            props::coin(d, v2(90.0, 0.0), 26.0);
            twinkle(d, v2(170.0, 0.0), 20.0, Ink::Yellow);
        });
        b.save(&out.join(format!("{name}.png")));
    }

    // 3. Paper cards: Grandma's note, receipt, listing, masthead.
    let mut b = Board::new(1440.0, 900.0, scale * 0.6, Edition::Dusk);
    b.put(v2(340.0, 330.0), |d| {
        props::note_card(d, 600.0, 540.0);
        props::grandma(d, v2(-230.0, -250.0), 56.0);
    });
    b.put(v2(1000.0, 360.0), |d| props::receipt(d, 420.0, 600.0));
    b.put(v2(40.0, 700.0), |d| props::listing(d, 660.0, 88.0, true));
    b.put(v2(40.0, 800.0), |d| props::listing(d, 660.0, 88.0, false));
    b.put(v2(720.0, 740.0), |d| props::masthead(d, 720.0, 96.0, 0.0));
    b.save(&out.join("scenes_cards.png"));

    // 4. Every icon on a grid (as buttons and order bubbles show them), at two sizes.
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
    let mut b =
        Board::new(cols as f32 * 110.0 + 20.0 + 460.0, rows as f32 * 110.0 + 20.0, scale, Edition::Daylight);
    for (i, ic) in icons.iter().enumerate() {
        let c = v2(65.0 + (i % cols) as f32 * 110.0, 65.0 + (i / cols) as f32 * 110.0);
        b.put(c, |d| {
            let plate = rounded_rect(rect(-48.0, -48.0, 96.0, 96.0), 18.0);
            d.stroke_p(proof_core::draw::Paint::solid(Ink::Key, 0.25), 1.5, &plate, true);
            icon(d, *ic, V2::ZERO, 30.0);
        });
        // The same icon at button size, on a round button.
        let c2 = v2(cols as f32 * 110.0 + 60.0 + (i % 6) as f32 * 72.0, 50.0 + (i / 6) as f32 * 76.0);
        b.put(c2, |d| {
            props::round_button(d, 30.0, Ink::Yellow, false);
            icon(d, *ic, v2(0.0, -1.0), 18.0);
        });
    }
    b.save(&out.join("scenes_icons.png"));

    // 5. App icon: Bubbles on a halftone tile.
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
