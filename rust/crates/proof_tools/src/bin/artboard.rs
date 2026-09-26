//! Render Proof's procedural art to PNG sheets for review (no Godot needed).
//!
//! Usage: `cargo run -p proof_tools --bin artboard -- [out_dir] [scale]`

use proof_core::art::bread::{CutView, LoafView, crumb_slice, loaf_top};
use proof_core::art::critters::{CritterView, critter};
use proof_core::art::jar::{JarView, jar};
use proof_core::art::oven::{OvenView, oven};
use proof_core::art::props;
use proof_core::art::scenes::{Backdrop, backdrop};
use proof_core::art::treats::treat;
use proof_core::art::{Expr, twinkle};
use proof_core::content::{Flour, Pattern, Recipe, Shape, Species, Stencil, Topping, Treat};
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, v2};
use proof_core::ink::{Edition, Ink};
use proof_core::scoring::template;
use proof_raster::{Canvas, CompositeStyle, RasterConfig, rasterize};
use std::path::PathBuf;

struct Board {
    canvas: Canvas,
    cfg: RasterConfig,
    palette: proof_core::ink::Palette,
    style: CompositeStyle,
}

impl Board {
    fn new(w: f32, h: f32, scale: f32, edition: Edition) -> Board {
        let palette = edition.palette();
        Board {
            canvas: Canvas::paper(w, h, scale, &palette),
            cfg: RasterConfig {
                scale,
                ..RasterConfig::default()
            },
            palette,
            style: CompositeStyle::default(),
        }
    }

    /// Draw art (authored around its own origin) at `at`.
    fn put(&mut self, at: V2, f: impl FnOnce(&mut DrawList)) {
        let mut d = DrawList::new();
        d.with(Xf::at(at), f);
        let pl = rasterize(&d, &self.cfg, None);
        self.canvas.draw(&pl, V2::ZERO, &self.palette, &self.style);
    }

    fn save(&self, path: &PathBuf) {
        proof_tools::write_png(
            path,
            self.canvas.width,
            self.canvas.height,
            &self.canvas.to_rgba8(),
        )
        .unwrap();
        println!("wrote {}", path.display());
    }
}

fn cuts_for(p: Pattern, shape: Shape, bloom: f32, ear: f32) -> Vec<CutView> {
    template(p, shape)
        .into_iter()
        .map(|pts| CutView { pts, bloom, ear })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(
        args.get(1)
            .cloned()
            .unwrap_or_else(|| "../out/artboard".into()),
    );
    let scale: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    // --- Sheet 1: pets & bread -------------------------------------------------
    for edition in [Edition::Dawn, Edition::Daylight, Edition::Dusk] {
        let mut b = Board::new(1440.0, 1500.0, scale, edition);
        b.put(v2(180.0, 330.0), |d| {
            jar(
                d,
                &JarView {
                    expr: Expr::Content,
                    ..JarView::default()
                },
            )
        });
        b.put(v2(470.0, 330.0), |d| {
            jar(
                d,
                &JarView {
                    pep: 1.0,
                    tang: 0.8,
                    rise: 0.95,
                    band: 0.4,
                    expr: Expr::Excited,
                    seed: 4,
                    cloth: Ink::Blue,
                    ..JarView::default()
                },
            )
        });
        b.put(v2(760.0, 330.0), |d| {
            jar(
                d,
                &JarView {
                    pep: 0.15,
                    tang: 0.6,
                    rise: 0.2,
                    band: 0.5,
                    hooch: true,
                    expr: Expr::Sleepy,
                    seed: 9,
                    cloth: Ink::Yellow,
                    ..JarView::default()
                },
            )
        });
        b.put(v2(1090.0, 250.0), |d| {
            let mut v = LoafView {
                bake: 0.0,
                ..LoafView::default()
            };
            v.cuts = cuts_for(Pattern::Wheat, Shape::Boule, 0.0, 0.0);
            loaf_top(d, &v);
        });
        b.put(v2(220.0, 620.0), |d| {
            let v = LoafView {
                cuts: cuts_for(Pattern::Wheat, Shape::Boule, 0.8, 0.3),
                topping: Some(Topping::Sesame),
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        b.put(v2(620.0, 620.0), |d| {
            let v = LoafView {
                shape: Shape::Batard,
                recipe: Recipe::DarkRye,
                crust: 0.85,
                cuts: cuts_for(Pattern::Ear, Shape::Batard, 1.0, 1.0),
                seed: 3,
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        b.put(v2(1060.0, 620.0), |d| {
            let v = LoafView {
                recipe: Recipe::CranberryWalnut,
                crust: 0.2,
                stencil: Some(Stencil::Heart),
                cuts: cuts_for(Pattern::Cross, Shape::Boule, 0.9, 0.2),
                seed: 11,
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        b.put(v2(220.0, 960.0), |d| {
            let v = LoafView {
                recipe: Recipe::Olive,
                crust: 0.6,
                stencil: Some(Stencil::Star),
                cuts: vec![],
                topping: None,
                seed: 5,
                r: 130.0,
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        b.put(v2(560.0, 960.0), |d| {
            let v = LoafView {
                recipe: Recipe::Cheddar,
                crust: 0.5,
                cuts: cuts_for(Pattern::Leaf, Shape::Boule, 0.7, 0.2),
                topping: Some(Topping::Oats),
                seed: 8,
                r: 130.0,
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        b.put(v2(930.0, 980.0), |d| {
            crumb_slice(d, 300.0, 200.0, 0.85, 0.55, 3)
        });
        b.put(v2(1260.0, 960.0), |d| {
            let v = LoafView {
                recipe: Recipe::WholeWheat,
                stencil: Some(Stencil::Bunny),
                topping: Some(Topping::Poppy),
                r: 110.0,
                seed: 21,
                ..LoafView::default()
            };
            loaf_top(d, &v)
        });
        // Treats + bags.
        b.put(v2(150.0, 1310.0), |d| treat(d, Treat::Muffin, 150.0, 1));
        b.put(v2(330.0, 1310.0), |d| {
            treat(d, Treat::CinnamonBun, 150.0, 2)
        });
        b.put(v2(510.0, 1310.0), |d| treat(d, Treat::Bagel, 150.0, 3));
        for (i, f) in Flour::ALL.iter().enumerate() {
            b.put(v2(680.0 + i as f32 * 150.0, 1400.0), |d| {
                props::flour_bag(d, *f)
            });
        }
        b.put(v2(1250.0, 1300.0), |d| {
            props::banneton(d, Shape::Boule, 110.0, true)
        });
        b.save(&out.join(format!("sheet_bread_{edition:?}.png").to_lowercase()));
    }

    // --- Sheet 2: customers, Toasty, props -----------------------------------------
    let mut b = Board::new(1440.0, 1500.0, scale, Edition::Daylight);
    let exprs = [
        Expr::Content,
        Expr::Happy,
        Expr::Excited,
        Expr::Hmm,
        Expr::Wow,
        Expr::Proud,
        Expr::Hungry,
        Expr::Sleepy,
    ];
    for (i, sp) in Species::ALL.iter().enumerate() {
        let x = 180.0 + (i % 4) as f32 * 360.0;
        let y = 330.0 + (i / 4) as f32 * 360.0;
        b.put(v2(x, y), |d| {
            critter(
                d,
                &CritterView {
                    species: *sp,
                    expr: exprs[i],
                    t: 0.0,
                },
            )
        });
    }
    b.put(v2(260.0, 1180.0), |d| {
        oven(
            d,
            &OvenView {
                glow: 0.8,
                steam: 1.0,
                expr: Expr::Happy,
                ..OvenView::default()
            },
            |d| {
                let v = LoafView {
                    r: 44.0,
                    bake: 0.6,
                    cuts: cuts_for(Pattern::Ear, Shape::Boule, 0.8, 0.8),
                    ..LoafView::default()
                };
                d.with(Xf::at(v2(0.0, -140.0)), |d| loaf_top(d, &v));
            },
        )
    });
    b.put(v2(640.0, 1000.0), |d| {
        props::ticket(d, 220.0, 170.0, Ink::Pink)
    });
    b.put(v2(640.0, 1000.0), |d| {
        props::pattern_icon(d, v2(-50.0, 20.0), 30.0, Pattern::Wheat);
        props::lemon_icon(d, v2(40.0, 20.0), 30.0);
    });
    b.put(v2(920.0, 1000.0), |d| {
        d.with(Xf::IDENTITY.rotated(-0.25), |d| {
            props::stamp(d, 90.0, 3, Ink::Pink, 4)
        })
    });
    b.put(v2(1210.0, 980.0), |d| {
        props::button(d, 240.0, 84.0, Ink::Yellow, false)
    });
    b.put(v2(1210.0, 1090.0), |d| {
        props::button(d, 240.0, 84.0, Ink::Pink, true)
    });
    b.put(v2(640.0, 1260.0), |d| {
        props::bubble(d, 260.0, 140.0, v2(-40.0, 110.0))
    });
    b.put(v2(640.0, 1250.0), |d| {
        props::cloud_icon(d, v2(-70.0, 0.0), 32.0);
        props::crust_icon(d, v2(0.0, 0.0), 30.0, 0.9);
        props::gift_icon(d, v2(75.0, 0.0), 30.0);
    });
    b.put(v2(900.0, 1250.0), |d| {
        for (i, st) in Stencil::ALL.iter().enumerate() {
            props::stencil_icon(
                d,
                v2((i % 2) as f32 * 80.0, (i / 2) as f32 * 80.0 - 40.0),
                32.0,
                *st,
            );
        }
    });
    b.put(v2(1120.0, 1250.0), |d| {
        for (i, t) in Topping::ALL.iter().enumerate() {
            props::topping_icon(d, v2(i as f32 * 80.0, 0.0), 30.0, *t);
        }
        props::heart_icon(d, v2(0.0, 90.0), 44.0);
        props::coin(d, v2(80.0, 90.0), 24.0);
        twinkle(d, v2(160.0, 90.0), 18.0, Ink::Yellow);
        props::burst(d, 30.0, 8, 3);
    });
    b.save(&out.join("sheet_cast.png"));

    // --- Backdrops (side by side, each in its own edition) -------------------------------
    let mut b = Board::new(2160.0, 1280.0, scale * 0.5, Edition::Daylight);
    for (i, (bd, ed)) in [
        (Backdrop::Bakehouse, Edition::Dawn),
        (Backdrop::Shopfront, Edition::Daylight),
        (Backdrop::Pantry, Edition::Dusk),
    ]
    .into_iter()
    .enumerate()
    {
        b.palette = ed.palette();
        b.put(v2(i as f32 * 720.0, 0.0), |d| backdrop(d, bd));
    }
    b.save(&out.join("backdrops.png"));
}
