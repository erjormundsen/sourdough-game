//! Evening prep: feed the starters, read tomorrow's board, mix doughs into the fridge.
//!
//! Composition: the pantry wall with the moonlit window (left) and the cork "Tomorrow"
//! board (right); the starter jars stand on the lamp-lit shelf with paper status tags
//! hanging from its edge; the prep counter below holds the fridge's bannetons on a couche
//! (and the flour sacks while feeding); the action bar sits on the cabinet doors.

use super::{Ctx, Nav, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list, list_at};
use crate::sfx::Sfx;
use crate::ui::{self, Button, Floater, Hud, Panel};
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::Spring;
use proof_core::art::critters::{CritterView, HEAD_C, critter};
use proof_core::art::icons::{Icon, icon};
use proof_core::art::jar::{TAPE_CENTER, jar};
use proof_core::art::scenes::{
    Backdrop, Dressing, backdrop_dressed, counter_front, counter_top, jar_scale, jar_shelf, jar_slots,
};
use proof_core::art::{Expr, props};
use proof_core::content::{Flour, Recipe, Shape, Species};
use proof_core::customer::Want;
use proof_core::draw::{DrawList, Paint};
use proof_core::geom::{V2, Xf, arc, blob, circle, rect, v2};
use proof_core::gesture::{FOLD_ORDER, compass_vec, fold_quality};
use proof_core::ink::Ink;
use proof_core::state::{Action, Event, Visit};

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
    tag: Art,
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
    floaters: Vec<Floater>,
    /// Where objects stand on the prep counter.
    surface: f32,
    shelf_y: f32,
    bar: f32,
    t: f32,
}

/// Jar centre used for the stir gesture.
const STIR_UP: f32 = 95.0;
const STIR_R: f32 = 150.0;

/// A round portrait of a regular for the board (head centred in a circle of radius `r`).
fn portrait(d: &mut DrawList, species: Species, c: V2, r: f32) {
    // TODO(B): switch to `critters::critter_head(d, species, Expr::Content)` once it lands.
    let frame = circle(c, r);
    d.backing(&frame);
    d.fill(Ink::Blue, 0.2, &frame);
    let k = r / 92.0;
    d.clipped(&frame, |d| {
        d.with(Xf::at(c - HEAD_C * k + v2(0.0, r * 0.18)).scaled(k), |d| {
            critter(d, &CritterView { species, expr: Expr::Content, t: 0.0 })
        });
    });
    d.outline(Ink::Key, 2.6, &frame);
}

impl Evening {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>) -> Evening {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let top = ctx.lay.top;
        let hh = ctx.lay.h - top - ctx.lay.bottom;
        let slots = jar_slots(Backdrop::Pantry, ctx.state.starters.len());
        let dress = Dressing { jars: slots.clone() };
        let bg =
            ctx.riso.art(&mut rn, v2(0.0, top), &list(|d| backdrop_dressed(d, Backdrop::Pantry, hh, &dress)));
        let shelf_y = top + jar_shelf(Backdrop::Pantry, hh);
        let counter_y = top + counter_top(Backdrop::Pantry, hh);
        let surface = counter_y + 52.0;
        let board = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let scale = jar_scale(Backdrop::Pantry);
        let mut jars = Vec::new();
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let home = v2(slots.get(i).copied().unwrap_or(360.0), shelf_y - 4.0);
            let art =
                ctx.riso.art(&mut rn, home, &list_at(Xf::IDENTITY.scaled(scale), |d| jar(d, &s.view(0.0))));
            let mut an = art.as_node();
            let tc = TAPE_CENTER * scale;
            let name = ctx.riso.text(
                &mut an,
                &TextSpec::new(s.name.clone(), rect(tc.x - 60.0, tc.y - 16.0, 120.0, 32.0), 21.0)
                    .bold()
                    .plain(),
            );
            let tag_at = v2(home.x, shelf_y + 15.0);
            let tag =
                ctx.riso.art(&mut rn, tag_at, &list(|d| props::hanging_tag(d, 204.0, 44.0, 10.0, Ink::Pink)));
            let status = ctx.riso.text(
                &mut rn,
                &TextSpec::new("", rect(tag_at.x - 102.0, tag_at.y + 14.0, 204.0, 40.0), 18.0).bold().plain(),
            );
            jars.push(JarSlot { art, name, tag, status, home, bounce: Spring::new(1.0) });
        }
        let ring = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let front = ctx.riso.art(&mut rn, v2(0.0, top), &list(|d| counter_front(d, Backdrop::Pantry, hh)));
        let fridge = ctx.riso.art(&mut rn, v2(360.0, counter_y + 38.0), &DrawList::new());
        let fridge_label = ctx.riso.text(
            &mut rn,
            &TextSpec::new("", rect(360.0 - 90.0, counter_y + 77.0, 180.0, 26.0), 18.0).bold().plain(),
        );
        let dough = ctx.riso.art(&mut rn, v2(360.0, 820.0), &DrawList::new());
        let arrow = ctx.riso.art(&mut rn, v2(360.0, 820.0), &DrawList::new());
        let hud = Hud::new(ctx, &mut rn, "Dusk edition · prep");
        let bar = ui::bar_y(ctx);
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
            floaters: Vec::new(),
            surface,
            shelf_y,
            bar,
            t: 0.0,
        };
        e.build_board(ctx);
        e.refresh_jars(ctx);
        e.refresh_fridge(ctx);
        let mut rn2 = e.rn.clone();
        let (sec, pri) = ui::bar_rects(bar, true);
        let mix = Button::pill(ctx, &mut rn2, sec, "Mix dough", Ink::Yellow);
        e.btns.add(Btn::Mix, mix);
        let night = Button::pill(ctx, &mut rn2, pri, "Night page", Ink::Pink);
        e.btns.add(Btn::Night, night);
        e.set_idle_prompt(ctx);
        ctx.note(
            "evening",
            "Evening is for getting ready! Tap a jar to feed it, then mix doughs for tomorrow — they rest in the fridge overnight.",
        );
        e
    }

    fn say(&mut self, ctx: &mut Ctx, t: &str) {
        self.hud.say(ctx.riso, t);
    }

    fn set_idle_prompt(&mut self, ctx: &mut Ctx) {
        let hungry = ctx.state.starters.iter().any(|s| !s.fed_today);
        let t = if hungry {
            "Tap a jar to feed it"
        } else if ctx.state.fridge_free() > 0 {
            "Mix doughs for tomorrow"
        } else {
            "All set! Read the night page"
        };
        self.say(ctx, t);
    }

    /// The cork board's rectangle (upper right, above the jars).
    /// The cork board: sized to tomorrow's cards and centred in the wall space between the
    /// headline ribbon and the jars' cloth caps.
    fn board_rect(&self, ctx: &Ctx) -> proof_core::geom::Rect {
        let y0 = ctx.lay.top + ui::RIBBON_Y + 34.0;
        let y1 = self.shelf_y - 200.0 * jar_scale(Backdrop::Pantry) / 0.8 - 20.0;
        let rows = ctx.state.tomorrow.len().clamp(1, 6).div_ceil(2) as f32;
        let card = if rows <= 2.0 { 118.0 } else { 100.0 };
        let want = 50.0 + 44.0 + rows * (card + 12.0) + 12.0;
        let h = want.min(y1 - y0).max(300.0);
        rect(332.0, y0 + ((y1 - y0) - h).max(0.0) * 0.4, 368.0, h)
    }

    fn build_board(&mut self, ctx: &mut Ctx) {
        for mut l in self.board_labels.drain(..) {
            l.queue_free();
        }
        let visitors: Vec<Visit> = ctx.state.tomorrow.clone();
        let r = self.board_rect(ctx);
        let c = r.center();
        let n = visitors.len().min(6);
        let rows = n.div_ceil(2).max(1);
        let title_h = 50.0;
        let avail = r.h - title_h - 44.0;
        let (cw, ch) = (156.0, (avail / rows as f32 - 12.0).min(if rows <= 2 { 118.0 } else { 100.0 }));
        let grid_top = r.y + title_h + 26.0 + (avail - rows as f32 * (ch + 12.0)).max(0.0) * 0.5;
        let card_c = |i: usize| -> V2 {
            let col = i % 2;
            let row = i / 2;
            let last_single = n % 2 == 1 && i == n - 1;
            let x = if last_single { c.x } else { c.x + (col as f32 - 0.5) * (cw + 16.0) };
            v2(x, grid_top + row as f32 * (ch + 12.0) + ch * 0.5)
        };
        let d = list(|d| {
            d.with(Xf::at(c), |d| props::cork_board(d, r.w, r.h));
            // Title slip pinned at the top.
            d.with(Xf::at(v2(c.x, r.y + 30.0)).rotated(-0.02), |d| {
                props::pinned_card(d, 190.0, 40.0, None, 2)
            });
            for (i, v) in visitors.iter().enumerate().take(6) {
                let p = card_c(i);
                let rot = (i as f32 * 1.7).sin() * 0.03;
                d.with(Xf::at(p).rotated(rot), |d| {
                    props::pinned_card(d, cw, ch, if v.preorder { Some(Ink::Pink) } else { None }, i as u32);
                    let pr = (ch * 0.34).min(38.0);
                    portrait(d, v.species, v2(-cw * 0.5 + pr + 12.0, 7.0), pr);
                    let nw = v.order.wants.len().min(2);
                    let is = (ch * 0.22).clamp(19.0, 25.0);
                    for (k, w) in v.order.wants.iter().enumerate().take(2) {
                        let x = if nw == 1 { cw * 0.22 } else { cw * 0.06 + k as f32 * 46.0 };
                        icon(d, Icon::Want(*w), v2(x, 8.0), is);
                    }
                    if v.preorder {
                        d.with(Xf::at(v2(cw * 0.5 - 16.0, -ch * 0.5 + 4.0)).rotated(0.6), |d| {
                            props::tape(d, 58.0, 16.0, Ink::Pink)
                        });
                    }
                });
            }
        });
        self.board.set(ctx.riso, &d);
        let mut rn = self.rn.clone();
        self.board_labels.push(ctx.riso.text(
            &mut rn,
            &TextSpec::new("Tomorrow", rect(c.x - 95.0, r.y + 10.0, 190.0, 44.0), 25.0).bold(),
        ));
        if visitors.is_empty() {
            self.board_labels.push(
                ctx.riso.text(
                    &mut rn,
                    &TextSpec::new("A quiet day ahead", rect(r.x + 20.0, c.y - 20.0, r.w - 40.0, 60.0), 24.0)
                        .plain(),
                ),
            );
        }
    }

    fn refresh_jars(&mut self, ctx: &mut Ctx) {
        let t = (self.t * 3.0).floor() / 3.0;
        let scale = jar_scale(Backdrop::Pantry);
        for (i, slot) in self.jars.iter_mut().enumerate() {
            let Some(s) = ctx.state.starters.get(i) else { continue };
            let view = s.view(t);
            slot.art.set(ctx.riso, &list_at(Xf::IDENTITY.scaled(scale), |d| jar(d, &view)));
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
            let pitch = 70.0;
            let w = slots as f32 * pitch + 30.0;
            props::couche(d, w, 58.0);
            for i in 0..slots {
                let x = -w * 0.5 + 15.0 + pitch * 0.5 + i as f32 * pitch;
                let shape = shapes.get(i as usize).copied().unwrap_or(Shape::Boule);
                if i < n {
                    d.with(Xf::at(v2(x, -2.0)), |d| props::banneton(d, shape, 25.0, true));
                } else {
                    // An empty spot: a dashed ring waiting for a banneton.
                    for k in 0..12 {
                        let a0 = k as f32 / 12.0 * std::f32::consts::TAU;
                        let seg = arc(v2(x, -2.0), 24.0, a0, a0 + 0.3);
                        d.stroke_p(Paint::solid(Ink::Key, 0.5), 2.0, &seg, false);
                    }
                }
            }
            // A masking-tape label on the counter's edge.
            d.with(Xf::at(v2(0.0, 52.0)).rotated(-0.015), |d| props::tape(d, 180.0, 28.0, Ink::Yellow));
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

    fn set_idle_widgets(&mut self, visible: bool) {
        for (_, b) in &mut self.btns.items {
            b.set_visible(visible);
        }
        self.fridge.set_visible(visible);
        self.fridge_label.set_visible(visible);
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
            let x = 360.0 + (i as f32 - (n - 1.0) * 0.5) * 196.0;
            let b = Button::sack(ctx, &mut rn, v2(x, self.surface + 6.0), *f);
            self.feed_btns.add(Btn::Flour(*f), b);
        }
        let (_, pri) = ui::bar_rects(self.bar, false);
        let cancel = Button::pill(ctx, &mut rn, pri, "Back", Ink::Blue);
        self.feed_btns.add(Btn::CancelFeed, cancel);
        self.set_idle_widgets(false);
        let name = ctx.state.starters[jar].name.clone();
        self.say(ctx, &format!("What's for dinner, {name}?"));
        ctx.sfx(Sfx::Bubble);
        ctx.note("feed", "White flour feeds the pink Yeasties (mild). Rye feeds the blue Lactos (tangy). Whole wheat is a bit of both!");
    }

    fn pick_flour(&mut self, ctx: &mut Ctx, flour: Flour) {
        let Mode::PickFlour(jar) = self.mode else { return };
        self.feed_btns.clear();
        self.mode = Mode::Stir { jar, flour, turns: 0.0, last: None };
        self.say(ctx, "Stir! Draw circles around the jar");
        ctx.sfx(Sfx::Pour);
        self.draw_ring(ctx, jar, 0.0);
    }

    fn draw_ring(&mut self, ctx: &mut Ctx, jar: usize, progress: f32) {
        let c = self.jars[jar].home + v2(0.0, -STIR_UP);
        let d = list(|d| {
            let r = STIR_R;
            // Dashed track.
            let n = 28;
            for k in 0..n {
                let a0 = k as f32 / n as f32 * std::f32::consts::TAU;
                d.stroke_p(Paint::solid(Ink::Blue, 0.75), 9.0, &arc(c, r, a0, a0 + 0.12), false);
            }
            let a0 = -std::f32::consts::FRAC_PI_2;
            if progress > 0.01 {
                let a1 = a0 + std::f32::consts::TAU * progress.min(1.0);
                d.line(Ink::Key, 18.0, &arc(c, r, a0, a1));
                d.line(Ink::Pink, 12.0, &arc(c, r, a0, a1));
            }
            // A wooden spoon riding the ring at the progress tip, pointing the way.
            let a = a0 + std::f32::consts::TAU * progress.min(1.0) + 0.25;
            let tip = c + V2::from_angle(a) * r;
            let dir = V2::from_angle(a + std::f32::consts::FRAC_PI_2);
            let head = vec![
                tip + dir * 20.0,
                tip - dir * 8.0 + dir.perp() * 16.0,
                tip - dir * 8.0 - dir.perp() * 16.0,
            ];
            d.backing(&head);
            d.fill(Ink::Pink, 1.0, &head);
            d.outline(Ink::Key, 3.5, &head);
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
        self.set_idle_widgets(true);
        self.set_idle_prompt(ctx);
    }

    // --- dough station -----------------------------------------------------------------------

    /// The dough-station card: under the ribbon, above the bar; on tall phones it keeps its
    /// proportions and sits centred with the pantry showing around it.
    fn station_rect(&self, ctx: &Ctx) -> proof_core::geom::Rect {
        let y0 = ctx.lay.top + ui::RIBBON_Y + 32.0;
        let y1 = self.bar - ui::PILL_H * 0.5 - 18.0;
        let h = (y1 - y0).min(1040.0);
        rect(22.0, y0 + ((y1 - y0) - h) * 0.5, 676.0, h)
    }

    fn open_station(&mut self, ctx: &mut Ctx) {
        if ctx.state.fridge_free() == 0 {
            ctx.toast("The fridge is full of bannetons!");
            return;
        }
        self.close_station();
        self.set_idle_widgets(false);
        let mut rn = self.rn.clone();
        let r = self.station_rect(ctx);
        let card = list(|d| {
            props::ticket(d, r.w, r.h, Ink::Yellow);
        });
        let mut card_art = ctx.riso.art(&mut rn, r.center(), &card);
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
            TextSpec::new("Mix a batch", rect(r.x + 60.0, r.y + 40.0, r.w - 120.0, 46.0), 32.0).bold(),
        );
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new(
                "makes two doughs for tomorrow",
                rect(r.x + 60.0, r.y + 82.0, r.w - 120.0, 30.0),
                19.0,
            )
            .plain(),
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
        let rows = recipes.len().div_ceil(per_row);
        let y_rec = r.y + 172.0;
        for (i, rc) in recipes.iter().enumerate() {
            let in_row = (recipes.len() - (i / per_row) * per_row).min(per_row) as f32;
            let x = 360.0 + ((i % per_row) as f32 - (in_row - 1.0) * 0.5) * 200.0;
            let y = y_rec + (i / per_row) as f32 * 142.0;
            let mut b = Button::round(ctx, &mut rn, v2(x, y), 46.0, Ink::Yellow, Icon::Recipe(*rc));
            b.set_z(6);
            self.station_btns.add(Btn::Recipe(*rc), b);
            add_label(
                self,
                ctx,
                &mut rn,
                TextSpec::new(rc.name(), rect(x - 100.0, y + 50.0, 200.0, 30.0), 19.0).bold().plain(),
            );
        }
        let y2 = y_rec + (rows as f32 - 1.0) * 142.0 + 124.0;
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new("Starter", rect(r.x + 36.0, y2 - 20.0, 140.0, 40.0), 23.0).bold().left(),
        );
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let mut b = Button::pill(
                ctx,
                &mut rn,
                rect(r.x + 170.0 + i as f32 * 164.0, y2 - 29.0, 152.0, 58.0),
                &s.name,
                Ink::Blue,
            );
            b.set_z(6);
            self.station_btns.add(Btn::Starter(i), b);
        }
        let y3 = y2 + 84.0;
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new("Shape", rect(r.x + 36.0, y3 - 20.0, 140.0, 40.0), 23.0).bold().left(),
        );
        for (i, s) in [Shape::Boule, Shape::Batard].iter().enumerate() {
            let x = r.x + 206.0 + i as f32 * 96.0;
            let mut b = Button::round(ctx, &mut rn, v2(x, y3), 34.0, Ink::Yellow, Icon::Shape(*s));
            b.set_z(6);
            self.station_btns.add(Btn::Shape(*s), b);
            add_label(
                self,
                ctx,
                &mut rn,
                TextSpec::new(s.name(), rect(x - 50.0, y3 + 38.0, 100.0, 26.0), 17.0).plain(),
            );
        }
        let mut close =
            Button::round(ctx, &mut rn, v2(r.x + r.w - 44.0, r.y + 58.0), 30.0, Ink::Blue, Icon::Back);
        close.set_z(6);
        self.station_btns.add(Btn::CloseMix, close);
        // The bowl sits in the space left under the choices, with room for the fold arrows,
        // a caption and the fold dots above it.
        let lo = y3 + 66.0 + 250.0;
        let hi = r.y + r.h - 12.0 - 250.0;
        let bowl_c = v2(360.0, if lo <= hi { (lo + hi) * 0.5 } else { hi });
        add_label(
            self,
            ctx,
            &mut rn,
            TextSpec::new(
                "Swipe the way the arrow points",
                rect(r.x + 40.0, bowl_c.y - 262.0, r.w - 80.0, 32.0),
                21.0,
            )
            .plain(),
        );
        self.dough.set_pos(bowl_c);
        self.arrow.set_pos(bowl_c);
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
        self.say(ctx, &st);
    }

    fn draw_station_dough(&mut self, ctx: &mut Ctx, folds: usize) {
        let shape = self.shape;
        let (rx, ry) = shape.radii();
        let d = list(|d| {
            let br = 150.0;
            d.with(Xf::at(v2(0.0, 10.0)), |d| props::bowl_back(d, br));
            let body = blob(v2(0.0, -8.0), 104.0 * rx, 76.0 * ry, 0.06, folds as u32 + 1);
            d.backing(&body);
            d.fill(Ink::Yellow, 0.3 + 0.05 * folds as f32, &body);
            props::shade(d, &body, v2(-16.0, -18.0), Ink::Pink, 0.0, 0.1);
            for i in 0..folds {
                let a = i as f32 * 0.9;
                d.stroke_p(
                    Paint::solid(Ink::Key, 0.5),
                    3.0,
                    &arc(v2(0.0, -8.0), 30.0 + i as f32 * 16.0, a, a + 1.6),
                    false,
                );
            }
            d.outline(Ink::Key, 4.5, &body);
            d.with(Xf::at(v2(0.0, 10.0)), |d| props::bowl_front(d, br));
            for i in 0..4 {
                let c = v2(-45.0 + i as f32 * 30.0, -218.0);
                let dot = circle(c, 9.0);
                d.backing(&dot);
                if i < folds {
                    d.fill(Ink::Pink, 1.0, &dot);
                } else {
                    d.fill(Ink::Yellow, 0.2, &dot);
                }
                d.outline(Ink::Key, 2.5, &dot);
            }
        });
        self.dough.set(ctx.riso, &d);
        let next = FOLD_ORDER.get(folds).map(|i| compass_vec(*i));
        let a = list(|d| {
            if let Some(dir) = next {
                let reach = if dir.y < -0.5 {
                    118.0
                } else if dir.y > 0.5 {
                    172.0
                } else {
                    178.0
                };
                let base = dir * reach;
                let tip = dir * (reach + 62.0);
                let n = dir.perp();
                let shaft = proof_core::geom::capsule(base, tip - dir * 16.0, 9.0);
                d.backing(&shaft);
                d.fill(Ink::Pink, 1.0, &shaft);
                d.outline(Ink::Key, 3.5, &shaft);
                let head = vec![tip + dir * 14.0, tip - dir * 24.0 + n * 28.0, tip - dir * 24.0 - n * 28.0];
                let head = proof_core::geom::chaikin(&head, 1, true);
                d.backing(&head);
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
        self.set_idle_widgets(true);
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
            let sway = (self.t * 1.3 + j.home.x * 0.01).sin() * 0.02;
            j.tag.node.set_rotation(sway);
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
                        let c = self.jars[jar].home + v2(0.0, -STIR_UP);
                        if let Mode::Stir { last, .. } = &mut self.mode {
                            *last = Some((q - c).angle());
                        }
                    }
                    _ => {}
                }
            }
            Ptr::Move(q) => {
                if let Mode::Stir { jar, turns, last, .. } = &mut self.mode {
                    let c = self.jars[*jar].home + v2(0.0, -STIR_UP);
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
                let c = self.jars[jar].home + v2(0.0, -STIR_UP);
                self.pointer(ctx, Ptr::Down(c + v2(STIR_R, 0.0)));
                for i in 1..=36 {
                    let a = i as f32 / 36.0 * std::f32::consts::TAU * 2.05;
                    self.pointer(ctx, Ptr::Move(c + V2::from_angle(a) * STIR_R));
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
