//! Evening prep: feed the starters, read tomorrow's board, mix doughs into the fridge.

use super::{Ctx, Nav, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list, list_at};
use crate::sfx::Sfx;
use crate::ui::{Button, Floater, Hud, Panel};
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::Spring;
use proof_core::art::critters::{CritterView, critter};
use proof_core::art::icons::{Icon, icon};
use proof_core::art::jar::{TAPE_CENTER, jar};
use proof_core::art::scenes::{Backdrop, backdrop, counter_front, counter_top};
use proof_core::art::{Expr, props};
use proof_core::content::{Flour, Recipe, Shape};
use proof_core::customer::Want;
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, arc, blob, rect, rounded_rect, v2};
use proof_core::gesture::{FOLD_ORDER, compass_vec, fold_quality};
use proof_core::ink::Ink;
use proof_core::state::{Action, Event};

const JAR_SCALE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Btn {
    Mix,
    Night,
    Flour(Flour),
    CancelFeed,
    Recipe(Recipe),
    Starter(usize),
    Shape(Shape),
    CloseMix,
}

#[derive(Clone, Debug, PartialEq)]
enum Mode {
    Idle,
    /// Picking a flour for jar `jar`.
    PickFlour(usize),
    /// Stirring jar `jar` after pouring `flour`.
    Stir {
        jar: usize,
        flour: Flour,
        turns: f32,
        last: Option<f32>,
    },
    /// The dough station overlay.
    Mixing {
        folds: Vec<V2>,
    },
}

struct JarSlot {
    art: Art,
    name: Gd<Label>,
    status: Gd<Label>,
    home: V2,
    bounce: Spring,
}

pub struct Evening {
    root: Gd<Node2D>,
    rn: Gd<Node>,
    hud: Hud,
    _bg: Art,
    _front: Art,
    board: Art,
    board_labels: Vec<Gd<Label>>,
    jars: Vec<JarSlot>,
    fridge: Art,
    fridge_label: Gd<Label>,
    btns: Panel<Btn>,
    feed_btns: Panel<Btn>,
    ring: Art,
    mode: Mode,
    // dough station
    station: Vec<Art>,
    station_labels: Vec<Gd<Label>>,
    station_btns: Panel<Btn>,
    dough: Art,
    dough_squash: Spring,
    arrow: Art,
    recipe: Recipe,
    starter: usize,
    shape: Shape,
    swipe_start: Option<V2>,
    prompt: Gd<Label>,
    floaters: Vec<Floater>,
    counter_y: f32,
    t: f32,
}

fn jar_centers(n: usize) -> Vec<f32> {
    match n {
        0 | 1 => vec![200.0],
        2 => vec![140.0, 360.0],
        _ => vec![130.0, 360.0, 590.0],
    }
}

impl Evening {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>) -> Evening {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let h = ctx.lay.h;
        let bg = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| backdrop(d, Backdrop::Pantry, h)));
        let counter_y = counter_top(Backdrop::Pantry, h);
        let board = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let mut jars = Vec::new();
        let xs = jar_centers(ctx.state.starters.len());
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let home = v2(xs.get(i).copied().unwrap_or(360.0), 700.0);
            let art = ctx.riso.art(
                &mut rn,
                home,
                &list_at(Xf::IDENTITY.scaled(JAR_SCALE), |d| jar(d, &s.view(0.0))),
            );
            let mut an = art.as_node();
            let tc = TAPE_CENTER * JAR_SCALE;
            let name = ctx.riso.text(
                &mut an,
                &TextSpec::new(s.name.clone(), rect(tc.x - 60.0, tc.y - 16.0, 120.0, 32.0), 21.0)
                    .bold()
                    .plain(),
            );
            let status = ctx
                .riso
                .text(&mut rn, &TextSpec::new("", rect(home.x - 110.0, 728.0, 220.0, 60.0), 20.0).wrap());
            jars.push(JarSlot { art, name, status, home, bounce: Spring::new(1.0) });
        }
        let ring = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let front = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| counter_front(d, Backdrop::Pantry, h)));
        let fridge = ctx.riso.art(&mut rn, v2(360.0, counter_y + 72.0), &DrawList::new());
        let fridge_label = ctx.riso.text(
            &mut rn,
            &TextSpec::new("", rect(210.0, counter_y + 84.0, 300.0, 36.0), 22.0).bold().plain(),
        );
        let dough = ctx.riso.art(&mut rn, v2(360.0, 820.0), &DrawList::new());
        let arrow = ctx.riso.art(&mut rn, v2(360.0, 820.0), &DrawList::new());
        let prompt = ctx.riso.text(&mut rn, &TextSpec::new("", rect(30.0, 800.0, 660.0, 50.0), 28.0).bold());
        let hud = Hud::new(ctx, &mut rn, "Dusk edition · prep");
        let mut e = Evening {
            root,
            rn,
            hud,
            _bg: bg,
            _front: front,
            board,
            board_labels: Vec::new(),
            jars,
            fridge,
            fridge_label,
            btns: Panel::default(),
            feed_btns: Panel::default(),
            ring,
            mode: Mode::Idle,
            station: Vec::new(),
            station_labels: Vec::new(),
            station_btns: Panel::default(),
            dough,
            dough_squash: Spring::new(1.0),
            arrow,
            recipe: Recipe::Country,
            starter: 0,
            shape: Shape::Boule,
            swipe_start: None,
            prompt,
            floaters: Vec::new(),
            counter_y,
            t: 0.0,
        };
        e.build_board(ctx);
        e.refresh_jars(ctx);
        e.refresh_fridge(ctx);
        let mut rn2 = e.rn.clone();
        let mix = Button::pill(ctx, &mut rn2, rect(40.0, h - 104.0, 300.0, 82.0), "Mix dough", Ink::Yellow);
        e.btns.add(Btn::Mix, mix);
        let night = Button::pill(ctx, &mut rn2, rect(380.0, h - 104.0, 300.0, 82.0), "Night page", Ink::Pink);
        e.btns.add(Btn::Night, night);
        e.set_idle_prompt(ctx);
        ctx.note(
            "evening",
            "Evening is for getting ready! Tap a jar to feed it, then mix doughs for tomorrow — they rest in the fridge overnight.",
        );
        e
    }

    fn set_idle_prompt(&mut self, ctx: &Ctx) {
        let hungry = ctx.state.starters.iter().any(|s| !s.fed_today);
        let t = if hungry {
            "Tap a jar to feed it"
        } else if ctx.state.fridge_free() > 0 {
            "Mix doughs for tomorrow"
        } else {
            "All set! Read the night page"
        };
        self.prompt.set_text(t);
    }

    fn build_board(&mut self, ctx: &mut Ctx) {
        for mut l in self.board_labels.drain(..) {
            l.queue_free();
        }
        let visitors = ctx.state.tomorrow.clone();
        let d = list(|d| {
            let r = rect(372.0, 108.0, 330.0, 372.0);
            let board = rounded_rect(r, 18.0);
            d.backing(&board);
            d.fill(Ink::Yellow, 0.55, &board);
            d.ht(Ink::Pink, 0.28, &board);
            d.outline(Ink::Key, 5.0, &board);
            for (i, v) in visitors.iter().enumerate().take(6) {
                let cx = r.x + 84.0 + (i % 2) as f32 * 164.0;
                let cy = r.y + 118.0 + (i / 2) as f32 * 112.0;
                let card = rounded_rect(rect(cx - 76.0, cy - 50.0, 152.0, 100.0), 12.0);
                d.backing(&card);
                if v.preorder {
                    d.fill(Ink::Pink, 0.18, &card);
                }
                d.outline(Ink::Key, 2.5, &card);
                d.clipped(&card, |d| {
                    d.with(Xf::at(v2(cx - 36.0, cy + 70.0)).scaled(0.36), |d| {
                        critter(d, &CritterView { species: v.species, expr: Expr::Content, t: 0.0 })
                    });
                });
                for (k, w) in v.order.wants.iter().enumerate().take(2) {
                    icon(d, Icon::Want(*w), v2(cx + 30.0 + k as f32 * 8.0, cy - 8.0 + k as f32 * 30.0), 24.0);
                }
                if v.preorder {
                    d.with(Xf::at(v2(cx + 36.0, cy - 50.0)).rotated(0.2), |d| {
                        d.fill(Ink::Pink, 1.0, &proof_core::geom::circle(V2::ZERO, 8.0));
                        d.outline(Ink::Key, 2.5, &proof_core::geom::circle(V2::ZERO, 8.0));
                    });
                }
            }
        });
        self.board.set(ctx.riso, &d);
        let mut rn = self.rn.clone();
        self.board_labels.push(
            ctx.riso.text(&mut rn, &TextSpec::new("Tomorrow", rect(372.0, 116.0, 330.0, 44.0), 30.0).bold()),
        );
        if visitors.is_empty() {
            self.board_labels.push(
                ctx.riso.text(
                    &mut rn,
                    &TextSpec::new("A quiet day ahead", rect(372.0, 250.0, 330.0, 60.0), 24.0),
                ),
            );
        }
    }

    fn refresh_jars(&mut self, ctx: &mut Ctx) {
        let t = (self.t * 3.0).floor() / 3.0;
        for (i, slot) in self.jars.iter_mut().enumerate() {
            let Some(s) = ctx.state.starters.get(i) else { continue };
            let view = s.view(t);
            slot.art.set(ctx.riso, &list_at(Xf::IDENTITY.scaled(JAR_SCALE), |d| jar(d, &view)));
            slot.name.set_text(&s.name);
            let st = if s.fed_today {
                format!("Fed ✓ · {}", s.tang_word())
            } else {
                format!("{} · {}", s.status(), s.tang_word())
            };
            slot.status.set_text(&st);
        }
    }

    fn refresh_fridge(&mut self, ctx: &mut Ctx) {
        let (n, slots) = (ctx.state.fridge.len() as u32, ctx.state.fridge_slots);
        let shapes: Vec<Shape> = ctx.state.fridge.iter().map(|d| d.shape).collect();
        let d = list(|d| {
            let w = slots as f32 * 64.0;
            for i in 0..slots {
                let x = -w * 0.5 + 32.0 + i as f32 * 64.0;
                let shape = shapes.get(i as usize).copied().unwrap_or(Shape::Boule);
                d.with(Xf::at(v2(x, -20.0)), |d| props::banneton(d, shape, 26.0, i < n));
            }
        });
        self.fridge.set(ctx.riso, &d);
        self.fridge_label.set_text(&format!("Fridge {n}/{slots}"));
    }

    fn jar_at(&self, p: V2) -> Option<usize> {
        self.jars.iter().position(|j| {
            let c = j.home + v2(0.0, -100.0);
            (p.x - c.x).abs() < 75.0 && (p.y - c.y).abs() < 110.0
        })
    }

    // --- feeding -----------------------------------------------------------------------------

    fn start_feed(&mut self, ctx: &mut Ctx, jar: usize) {
        if ctx.state.starters.get(jar).is_some_and(|s| s.fed_today) {
            ctx.toast("Already fed — it's full and happy!");
            return;
        }
        self.mode = Mode::PickFlour(jar);
        self.jars[jar].bounce.pos = 1.12;
        self.feed_btns.clear();
        let flours = ctx.state.unlocked_flours();
        let n = flours.len() as f32;
        let mut rn = self.rn.clone();
        for (i, f) in flours.iter().enumerate() {
            let x = 360.0 + (i as f32 - (n - 1.0) * 0.5) * 170.0;
            let b = Button::bare(ctx, &mut rn, v2(x, self.counter_y + 40.0), 115.0, Icon::Flour(*f));
            self.feed_btns.add(Btn::Flour(*f), b);
        }
        let h = ctx.lay.h;
        let cancel = Button::pill(ctx, &mut rn, rect(260.0, h - 100.0, 200.0, 70.0), "Back", Ink::Blue);
        self.feed_btns.add(Btn::CancelFeed, cancel);
        for (_, b) in &mut self.btns.items {
            b.set_visible(false);
        }
        self.fridge.set_visible(false);
        self.fridge_label.set_visible(false);
        let name = ctx.state.starters[jar].name.clone();
        self.prompt.set_text(&format!("What's for dinner, {name}?"));
        ctx.sfx(Sfx::Bubble);
        ctx.note("feed", "White flour feeds the pink Yeasties (mild). Rye feeds the blue Lactos (tangy). Whole wheat is a bit of both!");
    }

    fn pick_flour(&mut self, ctx: &mut Ctx, flour: Flour) {
        let Mode::PickFlour(jar) = self.mode else { return };
        self.feed_btns.clear();
        self.mode = Mode::Stir { jar, flour, turns: 0.0, last: None };
        self.prompt.set_text("Stir! Draw circles around the jar");
        ctx.sfx(Sfx::Pour);
        self.draw_ring(ctx, jar, 0.0);
    }

    fn draw_ring(&mut self, ctx: &mut Ctx, jar: usize, progress: f32) {
        let c = self.jars[jar].home + v2(0.0, -95.0);
        let d = list(|d| {
            let r = 150.0;
            d.stroke_p(
                proof_core::draw::Paint::ht(Ink::Blue, 0.5),
                14.0,
                &arc(c, r, 0.0, std::f32::consts::TAU),
                true,
            );
            if progress > 0.01 {
                let a0 = -std::f32::consts::FRAC_PI_2;
                d.line(Ink::Pink, 14.0, &arc(c, r, a0, a0 + std::f32::consts::TAU * progress.min(1.0)));
            }
            let tip = c + V2::from_angle(-0.4) * 150.0;
            d.fill(Ink::Key, 1.0, &[tip + v2(-14.0, -10.0), tip + v2(16.0, -4.0), tip + v2(-2.0, 18.0)]);
        });
        self.ring.set(ctx.riso, &d);
        self.ring.set_visible(true);
    }

    fn finish_feed(&mut self, ctx: &mut Ctx) {
        if let Mode::Stir { jar, flour, turns, .. } = self.mode.clone() {
            let stir = proof_core::gesture::stir_quality(turns);
            ctx.act(Action::Feed { jar, flour, stir });
        }
    }

    fn end_feed_mode(&mut self, ctx: &mut Ctx) {
        self.mode = Mode::Idle;
        self.feed_btns.clear();
        self.ring.set_visible(false);
        for (_, b) in &mut self.btns.items {
            b.set_visible(true);
        }
        self.fridge.set_visible(true);
        self.fridge_label.set_visible(true);
        self.set_idle_prompt(ctx);
    }

    // --- dough station -----------------------------------------------------------------------

    fn open_station(&mut self, ctx: &mut Ctx) {
        if ctx.state.fridge_free() == 0 {
            ctx.toast("The fridge is full of bannetons!");
            return;
        }
        self.close_station();
        for (_, b) in &mut self.btns.items {
            b.set_visible(false);
        }
        self.fridge.set_visible(false);
        self.fridge_label.set_visible(false);
        let mut rn = self.rn.clone();
        let h = ctx.lay.h;
        let card = list(|d| props::ticket(d, 680.0, h - 150.0, Ink::Yellow));
        let mut card_art = ctx.riso.art(&mut rn, v2(360.0, 90.0 + (h - 150.0) * 0.5), &card);
        card_art.node.set_z_index(5);
        self.station.push(card_art);
        fn add_label(e: &mut Evening, ctx: &mut Ctx, rn: &mut Gd<Node>, spec: TextSpec) {
            let mut l = ctx.riso.text(rn, &spec);
            l.set_z_index(6);
            e.station_labels.push(l);
        }
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new("Mix a batch (makes 2)", rect(40.0, 120.0, 640.0, 50.0), 30.0).bold(),
        );
        // Recipes: default to one somebody asked for.
        let wanted: Vec<Recipe> = ctx
            .state
            .tomorrow
            .iter()
            .flat_map(|v| v.order.wants.clone())
            .filter_map(|w| if let Want::Recipe(r) = w { Some(r) } else { None })
            .collect();
        let recipes = ctx.state.unlocked_recipes();
        self.recipe = wanted.iter().copied().find(|r| recipes.contains(r)).unwrap_or(recipes[0]);
        self.starter = (0..ctx.state.starters.len())
            .max_by(|a, b| ctx.state.starters[*a].pep.total_cmp(&ctx.state.starters[*b].pep))
            .unwrap_or(0);
        self.shape = if ctx.state.tomorrow.iter().any(|v| v.order.wants.contains(&Want::Shape(Shape::Batard)))
        {
            Shape::Batard
        } else {
            Shape::Boule
        };
        let per_row = 3;
        for (i, r) in recipes.iter().enumerate() {
            let x = 150.0 + (i % per_row) as f32 * 210.0;
            let y = 245.0 + (i / per_row) as f32 * 150.0;
            let mut b = Button::round(ctx, &mut rn, v2(x, y), 50.0, Ink::Yellow, Icon::Recipe(*r));
            b_z(&mut b);
            self.station_btns.add(Btn::Recipe(*r), b);
            add_label(
                self,
                ctx,
                &mut rn,
                TextSpec::new(r.name(), rect(x - 100.0, y + 50.0, 200.0, 34.0), 20.0).plain(),
            );
        }
        let rows = recipes.len().div_ceil(per_row) as f32;
        let y2 = 245.0 + rows * 150.0 - 10.0;
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new("Starter", rect(40.0, y2 - 20.0, 130.0, 40.0), 22.0).bold().left(),
        );
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let mut b = Button::pill(
                ctx,
                &mut rn,
                rect(160.0 + i as f32 * 170.0, y2 - 28.0, 160.0, 56.0),
                &s.name,
                Ink::Blue,
            );
            b_z(&mut b);
            self.station_btns.add(Btn::Starter(i), b);
        }
        let y3 = y2 + 78.0;
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new("Shape", rect(40.0, y3 - 20.0, 130.0, 40.0), 22.0).bold().left(),
        );
        for (i, s) in [Shape::Boule, Shape::Batard].iter().enumerate() {
            let mut b = Button::round(
                ctx,
                &mut rn,
                v2(210.0 + i as f32 * 100.0, y3),
                34.0,
                Ink::Yellow,
                Icon::Shape(*s),
            );
            b_z(&mut b);
            self.station_btns.add(Btn::Shape(*s), b);
        }
        let mut close = Button::round(ctx, &mut rn, v2(640.0, 140.0), 30.0, Ink::Blue, Icon::Back);
        b_z(&mut close);
        self.station_btns.add(Btn::CloseMix, close);
        self.dough.set_pos(v2(360.0, y3 + 190.0));
        self.arrow.set_pos(v2(360.0, y3 + 190.0));
        self.dough.node.set_z_index(6);
        self.arrow.node.set_z_index(7);
        self.dough.set_visible(true);
        self.arrow.set_visible(true);
        self.mode = Mode::Mixing { folds: Vec::new() };
        self.sync_station(ctx);
        self.draw_station_dough(ctx, 0);
        ctx.sfx(Sfx::Whoosh);
        ctx.note("mix", "Pick a recipe, then stretch & fold: swipe the dough the way the arrow points. Four folds make it strong!");
    }

    fn sync_station(&mut self, ctx: &mut Ctx) {
        let (r, s, sh) = (self.recipe, self.starter, self.shape);
        for (id, b) in &mut self.station_btns.items {
            let on = match id {
                Btn::Recipe(x) => *x == r,
                Btn::Starter(x) => *x == s,
                Btn::Shape(x) => *x == sh,
                _ => false,
            };
            if b.selected != on {
                b.set_selected(ctx, on);
            }
        }
        let st = ctx
            .state
            .starters
            .get(s)
            .map(|x| {
                format!(
                    "{} is {} and {}",
                    x.name,
                    if x.pep > 70.0 {
                        "peppy"
                    } else if x.pep > 40.0 {
                        "lively"
                    } else {
                        "sleepy"
                    },
                    x.tang_word()
                )
            })
            .unwrap_or_default();
        self.prompt.set_text(&st);
    }

    fn draw_station_dough(&mut self, ctx: &mut Ctx, folds: usize) {
        let shape = self.shape;
        let (rx, ry) = shape.radii();
        let d = list(|d| {
            let bowl = {
                let mut b = arc(v2(0.0, 10.0), 190.0, 0.0, std::f32::consts::PI);
                b.push(v2(-190.0, 10.0));
                b
            };
            let body = blob(v2(0.0, -10.0), 120.0 * rx, 100.0 * ry, 0.06, folds as u32 + 1);
            d.backing(&body);
            d.fill(Ink::Yellow, 0.3 + 0.05 * folds as f32, &body);
            for i in 0..folds {
                let a = i as f32 * 0.9;
                d.stroke_p(
                    proof_core::draw::Paint::solid(Ink::Key, 0.5),
                    3.0,
                    &arc(v2(0.0, -10.0), 40.0 + i as f32 * 18.0, a, a + 1.6),
                    false,
                );
            }
            d.outline(Ink::Key, 4.5, &body);
            d.backing(&bowl);
            d.fill(Ink::Blue, 0.62, &bowl);
            d.ht(Ink::Pink, 0.25, &bowl);
            d.outline(Ink::Key, 5.0, &bowl);
            for i in 0..4 {
                let c = v2(-45.0 + i as f32 * 30.0, 120.0);
                let dot = proof_core::geom::circle(c, 9.0);
                d.backing(&dot);
                if i < folds {
                    d.fill(Ink::Pink, 1.0, &dot);
                }
                d.outline(Ink::Key, 2.5, &dot);
            }
        });
        self.dough.set(ctx.riso, &d);
        let next = FOLD_ORDER.get(folds).map(|i| compass_vec(*i));
        let a = list(|d| {
            if let Some(dir) = next {
                let base = dir * 150.0;
                let tip = dir * 215.0;
                let n = dir.perp();
                d.line(Ink::Pink, 16.0, &[base, tip - dir * 10.0]);
                let head = vec![tip + dir * 12.0, tip - dir * 26.0 + n * 28.0, tip - dir * 26.0 - n * 28.0];
                d.fill(Ink::Pink, 1.0, &head);
                d.outline(Ink::Key, 4.0, &head);
            }
        });
        self.arrow.set(ctx.riso, &a);
    }

    fn fold(&mut self, ctx: &mut Ctx, dir: V2) {
        let Mode::Mixing { folds } = &mut self.mode else { return };
        folds.push(dir);
        let n = folds.len();
        let q = fold_quality(folds, &FOLD_ORDER[..n]);
        let good = q > 0.5;
        self.dough_squash.pos = 1.25;
        ctx.sfx_pitch(Sfx::Squish, 0.9 + n as f32 * 0.08);
        ctx.buzz(10);
        if !good && n == 1 {
            ctx.toast("Follow the arrow for a stronger dough!");
        }
        if n >= 4 {
            let fold = q;
            ctx.act(Action::Mix { recipe: self.recipe, jar: self.starter, shape: self.shape, fold });
        } else {
            self.draw_station_dough(ctx, n);
        }
    }

    fn close_station(&mut self) {
        for a in self.station.drain(..) {
            a.free();
        }
        for mut l in self.station_labels.drain(..) {
            l.queue_free();
        }
        self.station_btns.clear();
        self.dough.set_visible(false);
        self.arrow.set_visible(false);
        for (_, b) in &mut self.btns.items {
            b.set_visible(true);
        }
        self.fridge.set_visible(true);
        self.fridge_label.set_visible(true);
        if matches!(self.mode, Mode::Mixing { .. }) {
            self.mode = Mode::Idle;
        }
    }

    fn click(&mut self, ctx: &mut Ctx, b: Btn) {
        match b {
            Btn::Mix => self.open_station(ctx),
            Btn::Night => ctx.nav(Nav::Night),
            Btn::Flour(f) => self.pick_flour(ctx, f),
            Btn::CancelFeed => self.end_feed_mode(ctx),
            Btn::Recipe(r) => {
                self.recipe = r;
                self.sync_station(ctx);
            }
            Btn::Starter(i) => {
                self.starter = i;
                self.sync_station(ctx);
            }
            Btn::Shape(s) => {
                self.shape = s;
                self.sync_station(ctx);
                let n = if let Mode::Mixing { folds } = &self.mode { folds.len() } else { 0 };
                self.draw_station_dough(ctx, n);
            }
            Btn::CloseMix => {
                self.close_station();
                self.set_idle_prompt(ctx);
            }
        }
    }
}

fn b_z(b: &mut Button) {
    b.set_z(6);
}

impl Screen for Evening {
    fn root(&self) -> Gd<Node2D> {
        self.root.clone()
    }

    fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        self.t += dt;
        self.hud.refresh(ctx.riso, ctx.state);
        self.btns.update(ctx, dt);
        self.feed_btns.update(ctx, dt);
        self.station_btns.update(ctx, dt);
        self.floaters.retain_mut(|f| f.update(dt));
        let q = (self.t * 3.0).floor();
        if q != ((self.t - dt) * 3.0).floor() {
            self.refresh_jars(ctx);
        }
        for j in &mut self.jars {
            let s = j.bounce.step(dt, 260.0, 10.0);
            j.art.set_scale_xy(1.0 / s.sqrt(), s);
        }
        let s = self.dough_squash.step(dt, 240.0, 11.0);
        self.dough.set_scale_xy(s, 1.0 / s.sqrt());
        let pulse = 1.0 + 0.06 * (self.t * 5.0).sin();
        self.arrow.set_scale(pulse);
    }

    fn pointer(&mut self, ctx: &mut Ctx, p: Ptr) {
        match p {
            Ptr::Down(q) => {
                if matches!(self.mode, Mode::Mixing { .. }) {
                    if self.station_btns.down(ctx, q) {
                        return;
                    }
                    if self.dough.pos().dist(q) < 220.0 {
                        self.swipe_start = Some(q);
                    }
                    return;
                }
                if self.feed_btns.down(ctx, q) || self.btns.down(ctx, q) {
                    return;
                }
                match self.mode {
                    Mode::Idle => {
                        if let Some(j) = self.jar_at(q) {
                            self.start_feed(ctx, j);
                        }
                    }
                    Mode::Stir { jar, .. } => {
                        let c = self.jars[jar].home + v2(0.0, -95.0);
                        if let Mode::Stir { last, .. } = &mut self.mode {
                            *last = Some((q - c).angle());
                        }
                    }
                    _ => {}
                }
            }
            Ptr::Move(q) => {
                if let Mode::Stir { jar, turns, last, .. } = &mut self.mode {
                    let c = self.jars[*jar].home + v2(0.0, -95.0);
                    let a = (q - c).angle();
                    if let Some(l) = *last {
                        let mut d = a - l;
                        while d > std::f32::consts::PI {
                            d -= std::f32::consts::TAU;
                        }
                        while d < -std::f32::consts::PI {
                            d += std::f32::consts::TAU;
                        }
                        let before = (*turns).abs();
                        *turns += d / std::f32::consts::TAU;
                        if (before * 4.0).floor() != ((*turns).abs() * 4.0).floor() {
                            ctx.sfx_pitch(Sfx::Bubble, 0.8 + (*turns).abs() * 0.3);
                        }
                    }
                    *last = Some(a);
                    let (j, t) = (*jar, (*turns).abs());
                    self.jars[j].bounce.pos = 1.0 + 0.05 * (t * 12.0).sin();
                    self.draw_ring(ctx, j, t / 2.0);
                    if t >= 2.0 {
                        self.finish_feed(ctx);
                    }
                }
            }
            Ptr::Up(q) => {
                if matches!(self.mode, Mode::Mixing { .. }) {
                    if let Some(b) = self.station_btns.up(ctx, q) {
                        self.click(ctx, b);
                        return;
                    }
                    if let Some(s) = self.swipe_start.take() {
                        let d = q - s;
                        if d.len() > 60.0 {
                            self.fold(ctx, d.norm());
                        }
                    }
                    return;
                }
                if let Some(b) = self.feed_btns.up(ctx, q) {
                    self.click(ctx, b);
                    return;
                }
                if let Some(b) = self.btns.up(ctx, q) {
                    self.click(ctx, b);
                    return;
                }
                if let Mode::Stir { turns, .. } = self.mode
                    && turns.abs() >= 0.35
                {
                    self.finish_feed(ctx);
                }
            }
        }
    }

    fn events(&mut self, ctx: &mut Ctx, events: &[Event]) {
        for e in events {
            match e {
                Event::Fed { jar, result } => {
                    self.end_feed_mode(ctx);
                    self.refresh_jars(ctx);
                    if let Some(j) = self.jars.get_mut(*jar) {
                        j.bounce.pos = 1.3;
                    }
                    let mut rn = self.rn.clone();
                    let home = self.jars[*jar].home;
                    let msg = if result.revived {
                        "Revived! ♥".to_string()
                    } else if result.tang_after > result.tang_before + 1.0 {
                        "Tangier!".to_string()
                    } else if result.tang_after < result.tang_before - 1.0 {
                        "Milder!".to_string()
                    } else {
                        "Yum!".to_string()
                    };
                    self.floaters.push(Floater::new(
                        ctx.riso,
                        &mut rn,
                        &msg,
                        home + v2(0.0, -250.0),
                        32.0,
                        Ink::Key,
                    ));
                    ctx.sfx(Sfx::Bubble);
                    ctx.sfx(Sfx::Sparkle);
                    ctx.kick(0.35);
                    self.set_idle_prompt(ctx);
                }
                Event::Mixed { doughs } => {
                    self.close_station();
                    self.refresh_fridge(ctx);
                    ctx.toast(format!("{} doughs resting in the fridge", doughs.len()));
                    ctx.sfx(Sfx::Plop);
                    self.set_idle_prompt(ctx);
                }
                _ => {}
            }
        }
    }

    fn rejected(&mut self, ctx: &mut Ctx, r: proof_core::state::Reject) {
        ctx.toast(r.message());
        self.end_feed_mode(ctx);
        self.close_station();
    }

    fn auto(&mut self, ctx: &mut Ctx) -> Option<String> {
        match self.mode.clone() {
            Mode::Idle => {
                if let Some(j) = ctx.state.starters.iter().position(|s| !s.fed_today) {
                    self.start_feed(ctx, j);
                    return Some("feed".into());
                }
                if ctx.state.fridge_free() > 0 {
                    self.open_station(ctx);
                    return Some("station".into());
                }
                ctx.nav(Nav::Night);
                Some("ready".into())
            }
            Mode::PickFlour(_) => {
                let wants: Vec<Want> =
                    ctx.state.tomorrow.iter().flat_map(|v| v.order.wants.clone()).collect();
                let flours = ctx.state.unlocked_flours();
                let f = if wants.contains(&Want::Tangy) && flours.contains(&Flour::Rye) {
                    Flour::Rye
                } else if wants.contains(&Want::Mild) {
                    Flour::White
                } else {
                    *flours.last().unwrap_or(&Flour::White)
                };
                self.pick_flour(ctx, f);
                None
            }
            Mode::Stir { jar, .. } => {
                let c = self.jars[jar].home + v2(0.0, -95.0);
                self.pointer(ctx, Ptr::Down(c + v2(150.0, 0.0)));
                for i in 1..=36 {
                    let a = i as f32 / 36.0 * std::f32::consts::TAU * 2.05;
                    self.pointer(ctx, Ptr::Move(c + V2::from_angle(a) * 150.0));
                }
                Some("stirred".into())
            }
            Mode::Mixing { folds } => {
                let n = folds.len();
                if n < 4 {
                    let c = self.dough.pos();
                    let dir = compass_vec(FOLD_ORDER[n]);
                    self.pointer(ctx, Ptr::Down(c));
                    self.pointer(ctx, Ptr::Up(c + dir * 140.0));
                    if n == 1 { Some("fold".into()) } else { None }
                } else {
                    None
                }
            }
        }
    }
}
