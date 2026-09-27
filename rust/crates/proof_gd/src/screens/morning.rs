//! Morning: wake-up peek → dress & score each dough → Toasty's oven → the reveal → treats.
//!
//! Composition (see `scenes::bake_layout`): the Bakehouse wall up top with the starter jars
//! on their shelf; below, the marble bench where the dough rests on a floured board, the
//! waiting bannetons sit on a linen couche, and the Dress / Score tools live in two trays.
//! The action bar sits on the bench's wooden apron.

use super::{Ctx, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list, list_at, vec2};
use crate::sfx::Sfx;
use crate::ui::{self, Button, Floater, Hud, Panel};
use godot::classes::line_2d::{LineCapMode, LineJointMode};
use godot::classes::{Label, Line2D, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::{Spring, ease_out_back, ease_out_cubic, ease_out_elastic};
use proof_core::art::bread::{LoafView, crumb_slice, loaf_top};
use proof_core::art::icons::Icon;
use proof_core::art::jar::jar;
use proof_core::art::oven::{OvenView, WINDOW, oven};
use proof_core::art::scenes::{
    self, Backdrop, BakeLayout, Dressing, backdrop_dressed, counter_front, jar_scale, jar_shelf, jar_slots,
};
use proof_core::art::treats::{treat, treat_raw};
use proof_core::art::{Expr, props};
use proof_core::bake::Loaf;
use proof_core::content::{Pattern, Shape, Stencil, Topping, Treat};
use proof_core::customer::Want;
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, polyline_len, rect, resample, v2};
use proof_core::ink::Ink;
use proof_core::scoring::template;
use proof_core::state::{Event, TRAY_COST};

const R: f32 = 165.0;
const OVEN_SECS: f32 = 6.5;
/// Toasty is drawn a little larger than life on the bench.
const OVEN_SCALE: f32 = 1.15;
/// Round tool buttons in the trays.
const TOOL_R: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tool {
    Stencil(Option<Stencil>),
    Topping(Option<Topping>),
    Guide(Option<Pattern>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Btn {
    Undo,
    ToOven,
    BakeNow,
    Pull,
    Next,
    Treat(Treat),
    SkipTreats,
    OpenShop,
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    Peek(f32),
    Work,
    Oven { ids: Vec<u32>, t: f32, pulled: bool },
    Reveal { loaves: Vec<Loaf>, t: f32, stamped: bool },
    Treats { kind: Option<Treat>, done: [bool; 4] },
    Done,
}

pub struct Morning {
    root: Gd<Node2D>,
    rn: Gd<Node>,
    hud: Hud,
    _bg: Art,
    _front: Art,
    jars: Vec<Art>,
    jar_scale: f32,
    step: Step,
    sel: Option<u32>,
    cuts: Vec<Vec<V2>>,
    stencil: Option<Stencil>,
    topping: Option<Topping>,
    guide: Option<Pattern>,
    board: Art,
    dough: Art,
    dough_bounce: Spring,
    guide_art: Art,
    stroke: Vec<V2>,
    stroke_t0: f32,
    line: Gd<Line2D>,
    cutting: bool,
    tools: Panel<Tool>,
    btns: Panel<Btn>,
    trays: Vec<Art>,
    bannetons: Vec<(u32, Art)>,
    couche: Option<Art>,
    ready: Vec<u32>,
    oven: Art,
    meter: Art,
    oven_q: i32,
    reveal: Vec<Art>,
    stamps: Vec<Art>,
    reveal_labels: Vec<Gd<Label>>,
    tray: Vec<Art>,
    tray_props: Vec<Art>,
    work_labels: Vec<Gd<Label>>,
    oven_shot: bool,
    reveal_shot: bool,
    done_shot: bool,
    floaters: Vec<Floater>,
    /// The bench layout, already offset by the safe-area inset.
    lay: BakeLayout,
    dough_c: V2,
    bar: f32,
}

/// The bake layout for this screen, in screen coordinates.
fn stage(ctx: &Ctx) -> BakeLayout {
    let top = ctx.lay.top;
    let hh = ctx.lay.h - top - ctx.lay.bottom;
    let mut l = scenes::bake_layout(hh);
    let off = v2(0.0, top);
    l.dough += off;
    l.trays = [l.trays[0] + top, l.trays[1] + top];
    l.bannetons += top;
    l.oven += off;
    l.gauge += off;
    l.bar += top;
    l
}

impl Morning {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>) -> Morning {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let top = ctx.lay.top;
        let hh = ctx.lay.h - top - ctx.lay.bottom;
        let lay = stage(ctx);
        let slots = jar_slots(Backdrop::Bakehouse, ctx.state.starters.len());
        let dress = Dressing { jars: slots.clone() };
        let bg = ctx.riso.art(
            &mut rn,
            v2(0.0, top),
            &list(|d| backdrop_dressed(d, Backdrop::Bakehouse, hh, &dress)),
        );
        // Starter jars stand on the shelf for the wake-up peek.
        let js = top + jar_shelf(Backdrop::Bakehouse, hh) - 4.0;
        let jscale = jar_scale(Backdrop::Bakehouse);
        let mut jars = Vec::new();
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let mut view = s.view(0.0);
            view.rise = view.band;
            let x = slots.get(i).copied().unwrap_or(56.0 + i as f32 * 74.0);
            let a =
                ctx.riso.art(&mut rn, v2(x, js), &list_at(Xf::IDENTITY.scaled(jscale), |d| jar(d, &view)));
            jars.push(a);
        }
        let board = ctx.riso.art(&mut rn, lay.dough, &list(|d| props::bread_board(d, lay.board_r)));
        let dough = ctx.riso.art(&mut rn, lay.dough, &DrawList::new());
        let guide_art = ctx.riso.art(&mut rn, lay.dough, &DrawList::new());
        let oven = ctx.riso.art(&mut rn, lay.oven, &DrawList::new());
        let meter = ctx.riso.art(&mut rn, lay.gauge, &DrawList::new());
        let front = ctx.riso.art(&mut rn, v2(0.0, top), &list(|d| counter_front(d, Backdrop::Bakehouse, hh)));
        let mut line = Line2D::new_alloc();
        line.set_width(5.0);
        line.set_default_color(ctx.riso.ink(Ink::Key));
        line.set_joint_mode(LineJointMode::ROUND);
        line.set_begin_cap_mode(LineCapMode::ROUND);
        line.set_end_cap_mode(LineCapMode::ROUND);
        line.set_material(&ctx.riso.text_mat);
        rn.add_child(&line);
        let hud = Hud::new(ctx, &mut rn, "Dawn edition · bake");
        let bar = ui::bar_y(ctx);
        let mut m = Morning {
            root,
            rn,
            hud,
            _bg: bg,
            _front: front,
            jars,
            jar_scale: jscale,
            step: Step::Peek(0.0),
            sel: None,
            cuts: Vec::new(),
            stencil: None,
            topping: None,
            guide: None,
            board,
            dough,
            dough_bounce: Spring::new(1.0),
            guide_art,
            stroke: Vec::new(),
            stroke_t0: 0.0,
            line,
            cutting: false,
            tools: Panel::default(),
            btns: Panel::default(),
            trays: Vec::new(),
            bannetons: Vec::new(),
            couche: None,
            ready: Vec::new(),
            oven,
            meter,
            oven_q: -1,
            reveal: Vec::new(),
            stamps: Vec::new(),
            reveal_labels: Vec::new(),
            tray: Vec::new(),
            tray_props: Vec::new(),
            work_labels: Vec::new(),
            oven_shot: false,
            reveal_shot: false,
            done_shot: false,
            floaters: Vec::new(),
            dough_c: lay.dough,
            lay,
            bar,
        };
        m.set_prompt(ctx, "Good morning! The jars woke up bubbly.");
        ctx.sfx(Sfx::Bubble);
        m
    }

    fn set_prompt(&mut self, ctx: &mut Ctx, t: &str) {
        self.hud.say(ctx.riso, t);
    }

    fn dough_shape(&self, ctx: &Ctx) -> Shape {
        self.sel.and_then(|id| ctx.state.fridge_dough(id)).map(|d| d.shape).unwrap_or(Shape::Boule)
    }

    // --- Work step -----------------------------------------------------------------------

    fn enter_work(&mut self, ctx: &mut Ctx) {
        self.step = Step::Work;
        self.clear_reveal();
        self.oven.set_visible(false);
        self.meter.set_visible(false);
        self.board.set_visible(true);
        self.dough.set_visible(true);
        self.guide_art.set_visible(true);
        let remaining: Vec<u32> =
            ctx.state.fridge.iter().map(|d| d.id).filter(|id| !self.ready.contains(id)).collect();
        if remaining.is_empty() {
            if self.ready.is_empty() {
                self.after_baking(ctx);
            } else {
                let ids = std::mem::take(&mut self.ready);
                self.enter_oven(ctx, ids);
            }
            return;
        }
        self.sel = remaining.first().copied();
        let d = self.sel.and_then(|id| ctx.state.fridge_dough(id)).cloned();
        if let Some(d) = d {
            self.cuts = d.cuts.clone();
            self.stencil = d.stencil;
            self.topping = d.topping;
            self.guide = d.guide;
        }
        self.dough_bounce.pos = 0.6;
        self.build_work_ui(ctx);
        self.redraw_dough(ctx);
        self.set_prompt(ctx, "Dress it, then swipe to score!");
        ctx.note(
            "score",
            "Swipe across the dough with your blade to score it. One bold slash makes a big ear. Try a pattern if you like!",
        );
    }

    fn clear_trays(&mut self) {
        self.tools.clear();
        for a in self.trays.drain(..) {
            a.free();
        }
        for mut l in self.work_labels.drain(..) {
            l.queue_free();
        }
    }

    fn build_work_ui(&mut self, ctx: &mut Ctx) {
        self.clear_trays();
        self.btns.clear();
        let mut dress: Vec<(Tool, Icon)> = vec![(Tool::Stencil(None), Icon::None)];
        dress.extend(
            ctx.state.unlocked_stencils().into_iter().map(|s| (Tool::Stencil(Some(s)), Icon::Stencil(s))),
        );
        dress.extend(
            ctx.state.unlocked_toppings().into_iter().map(|t| (Tool::Topping(Some(t)), Icon::Topping(t))),
        );
        let mut score: Vec<(Tool, Icon)> = vec![(Tool::Guide(None), Icon::Freehand)];
        score.extend(
            ctx.state.unlocked_patterns().into_iter().map(|p| (Tool::Guide(Some(p)), Icon::Pattern(p))),
        );
        let mut rn = self.rn.clone();
        let tw = props::tray_layout(dress.len(), TOOL_R, 0)
            .0
            .max(props::tray_layout(score.len() + 1, TOOL_R, 1).0);
        for (row, (name, items)) in [("Dress", dress), ("Score", score)].into_iter().enumerate() {
            let undo = row == 1;
            let n = items.len() + usize::from(undo);
            let (_, slots) = props::tray_layout(n, TOOL_R, usize::from(undo));
            let c = v2(360.0, self.lay.trays[row]);
            let tab_w = 104.0;
            let art = list(|d| {
                d.with(Xf::at(v2(-tw * 0.5 + 14.0, -44.0)), |d| props::tab(d, tab_w, 30.0, Ink::Pink));
                props::tray(d, tw, 88.0, &slots, TOOL_R);
            });
            self.trays.push(ctx.riso.art(&mut rn, c, &art));
            self.work_labels.push(
                ctx.riso.text(
                    &mut rn,
                    &TextSpec::new(name, rect(c.x - tw * 0.5 + 14.0, c.y - 77.0, tab_w, 30.0), 20.0)
                        .bold()
                        .plain(),
                ),
            );
            for (i, (tool, ic)) in items.into_iter().enumerate() {
                let b = Button::round(ctx, &mut rn, c + slots[i], TOOL_R, Ink::Yellow, ic);
                self.tools.add(tool, b);
            }
            if undo {
                let b = Button::round(ctx, &mut rn, c + slots[n - 1], TOOL_R, Ink::Blue, Icon::Undo);
                self.btns.add(Btn::Undo, b);
            }
        }
        let with_now = !self.ready.is_empty();
        let (sec, pri) = ui::bar_rects(self.bar, with_now);
        let to = Button::pill(ctx, &mut rn, pri, "Into the oven!", Ink::Pink);
        self.btns.add(Btn::ToOven, to);
        if with_now {
            let now = Button::pill(ctx, &mut rn, sec, "Bake 1 now", Ink::Yellow);
            self.btns.add(Btn::BakeNow, now);
        }
        self.sync_tools(ctx);
        self.build_bannetons(ctx);
    }

    fn sync_tools(&mut self, ctx: &mut Ctx) {
        let (s, t, g) = (self.stencil, self.topping, self.guide);
        for (tool, b) in &mut self.tools.items {
            let on = match tool {
                Tool::Stencil(x) => *x == s && (x.is_some() || t.is_none()),
                Tool::Topping(x) => *x == t,
                Tool::Guide(x) => *x == g,
            };
            if b.selected != on {
                b.set_selected(ctx, on);
            }
        }
    }

    fn clear_bannetons(&mut self) {
        for (_, a) in self.bannetons.drain(..) {
            a.free();
        }
        if let Some(c) = self.couche.take() {
            c.free();
        }
    }

    fn build_bannetons(&mut self, ctx: &mut Ctx) {
        self.clear_bannetons();
        let top = ctx.lay.top;
        let hh = ctx.lay.h - top - ctx.lay.bottom;
        let n = ctx.state.fridge.len();
        let (slots, r) = scenes::banneton_slots(hh, n);
        let slots: Vec<V2> = slots.into_iter().map(|p| p + v2(0.0, top)).collect();
        let mut rn = self.rn.clone();
        if let (Some(first), Some(last)) = (slots.first(), slots.last()) {
            let cx = slots.iter().map(|p| p.x).sum::<f32>() / slots.len() as f32;
            let w = if n > 3 { 160.0 } else { 100.0 };
            let hgt = last.y - first.y + r * 2.0 + 44.0;
            let c = v2(cx, (first.y + last.y) * 0.5);
            self.couche = Some(ctx.riso.art(&mut rn, c, &list(|d| props::couche(d, w, hgt))));
        }
        for (i, d) in ctx.state.fridge.iter().enumerate() {
            let full = Some(d.id) != self.sel;
            let ready = self.ready.contains(&d.id);
            let shape = d.shape;
            let art = list(|dl| {
                props::banneton(dl, shape, r, full);
                if ready {
                    proof_core::art::twinkle(dl, v2(r * 0.9, -r * 0.8), 11.0, Ink::Yellow);
                }
            });
            let p = slots.get(i).copied().unwrap_or(v2(84.0, self.dough_c.y));
            let a = ctx.riso.art(&mut rn, p, &art);
            self.bannetons.push((d.id, a));
        }
    }

    fn redraw_dough(&mut self, ctx: &mut Ctx) {
        let Some(d) = self.sel.and_then(|id| ctx.state.fridge_dough(id)) else {
            self.dough.set(ctx.riso, &DrawList::new());
            return;
        };
        let mut v = d.view(R);
        v.cuts = self
            .cuts
            .iter()
            .map(|p| proof_core::art::bread::CutView { pts: p.clone(), bloom: 0.0, ear: 0.0 })
            .collect();
        v.stencil = self.stencil;
        v.topping = self.topping;
        self.dough.set(ctx.riso, &list(|dl| loaf_top(dl, &v)));
        let shape = d.shape;
        let g = self.guide.map(|p| template(p, shape)).unwrap_or_default();
        self.guide_art.set(
            ctx.riso,
            &list(|dl| {
                for s in &g {
                    let pts = resample(s, 40);
                    for w in pts.chunks(4) {
                        if w.len() >= 2 {
                            let seg: Vec<V2> = w.iter().map(|p| *p * R).collect();
                            dl.stroke_p(
                                proof_core::draw::Paint::solid(Ink::Blue, 0.75),
                                5.0,
                                &seg[..2],
                                false,
                            );
                        }
                    }
                }
            }),
        );
    }

    fn norm(&self, p: V2) -> V2 {
        (p - self.dough_c) / R
    }

    fn inside_dough(&self, ctx: &Ctx, p: V2) -> bool {
        let (rx, ry) = self.dough_shape(ctx).radii();
        let q = self.norm(p);
        (q.x / (rx * 1.1)).powi(2) + (q.y / (ry * 1.1)).powi(2) <= 1.0
    }

    fn finish_cut(&mut self, ctx: &mut Ctx, pts: Vec<V2>, secs: f32) {
        let norm: Vec<V2> = pts.iter().map(|p| self.norm(*p)).collect();
        let len = polyline_len(&norm);
        if len < 0.18 || self.cuts.len() >= 12 {
            return;
        }
        self.cuts.push(resample(&norm, (norm.len()).clamp(4, 24)));
        if let Some(id) = self.sel {
            ctx.act(proof_core::state::Action::Score {
                dough: id,
                cuts: self.cuts.clone(),
                guide: self.guide,
            });
        }
        let speed = (len / secs.max(0.05)).clamp(0.5, 8.0);
        ctx.sfx_pitch(Sfx::Blade, 0.8 + speed * 0.08);
        ctx.buzz(12);
        self.dough_bounce.pos = 0.96;
        self.redraw_dough(ctx);
    }

    fn mark_ready(&mut self, ctx: &mut Ctx, bake_now: bool) {
        let Some(id) = self.sel else { return };
        if !self.ready.contains(&id) {
            self.ready.push(id);
        }
        ctx.sfx(Sfx::Plop);
        let more = ctx.state.fridge.iter().any(|d| !self.ready.contains(&d.id));
        if bake_now || self.ready.len() >= proof_core::state::OVEN_CAPACITY || !more {
            let ids = std::mem::take(&mut self.ready);
            self.enter_oven(ctx, ids);
        } else {
            self.enter_work(ctx);
            ctx.toast("One more fits in Toasty!");
        }
    }

    // --- Oven step -----------------------------------------------------------------------

    fn enter_oven(&mut self, ctx: &mut Ctx, ids: Vec<u32>) {
        self.clear_trays();
        self.btns.clear();
        self.clear_bannetons();
        self.board.set_visible(false);
        self.dough.set_visible(false);
        self.guide_art.set_visible(false);
        self.oven.set_visible(true);
        self.meter.set_visible(true);
        self.oven.set_scale(OVEN_SCALE);
        self.oven_q = -1;
        let mut rn = self.rn.clone();
        let (_, pri) = ui::bar_rects(self.bar, false);
        let pull = Button::pill(ctx, &mut rn, pri, "Pull!", Ink::Pink);
        self.btns.add(Btn::Pull, pull);
        self.step = Step::Oven { ids, t: 0.0, pulled: false };
        self.set_prompt(ctx, "Pull when the crust looks just right");
        ctx.sfx(Sfx::Whoosh);
        ctx.note("oven", "Blonde, golden or bold — every crust is lovely. Some customers have favourites!");
    }

    fn draw_oven(&mut self, ctx: &mut Ctx, t: f32, ids: &[u32], open: f32, expr: Expr) {
        let q = (t * 16.0) as i32 + (open * 10.0) as i32 * 100;
        if q == self.oven_q {
            return;
        }
        self.oven_q = q;
        let doughs: Vec<_> = ids.iter().filter_map(|id| ctx.state.fridge_dough(*id)).cloned().collect();
        let crust = (t * 16.0).floor() / 16.0;
        let view = OvenView {
            glow: if open > 0.5 { 0.3 } else { 0.9 },
            open,
            expr,
            steam: (t * 3.0).min(1.0),
            t: crust * 6.0,
        };
        let wc = WINDOW.center();
        let d = list(|dl| {
            oven(dl, &view, |dl| {
                let n = doughs.len().max(1) as f32;
                let r = (WINDOW.h * 0.36).min(WINDOW.w / (2.2 * n));
                for (i, dough) in doughs.iter().enumerate() {
                    let x = wc.x + (i as f32 - (n - 1.0) * 0.5) * (WINDOW.w / n).min(r * 2.3);
                    let mut v = dough.view(r);
                    v.bake = crust.min(1.0);
                    v.crust = crust;
                    v.spring = crust;
                    for c in &mut v.cuts {
                        c.bloom = crust * 0.8;
                        c.ear = crust * 0.5;
                    }
                    dl.with(Xf::at(v2(x, wc.y + 4.0)), |dl| loaf_top(dl, &v));
                }
            })
        });
        self.oven.set(ctx.riso, &d);
        let meter = list(|dl| props::crust_gauge(dl, 500.0, t));
        self.meter.set(ctx.riso, &meter);
    }

    // --- Reveal ------------------------------------------------------------------------------

    fn clear_reveal(&mut self) {
        for a in self.reveal.drain(..) {
            a.free();
        }
        for a in self.stamps.drain(..) {
            a.free();
        }
        for mut l in self.reveal_labels.drain(..) {
            l.queue_free();
        }
    }

    /// The reveal card's rectangle: centred between the ribbon and the action bar.
    fn card_rect(&self, ctx: &Ctx) -> proof_core::geom::Rect {
        let y0 = ctx.lay.top + ui::RIBBON_Y + 30.0;
        let y1 = self.bar - ui::PILL_H * 0.5 - 22.0;
        let h = (y1 - y0).min(830.0);
        rect(40.0, y0 + (y1 - y0 - h) * 0.5, 640.0, h)
    }

    fn loaf_spot(&self, ctx: &Ctx, i: usize, n: usize) -> (V2, f32) {
        let c = self.card_rect(ctx);
        let y = c.y + 64.0 + (c.h - 64.0) * 0.28;
        if n == 1 { (v2(360.0, y), 150.0) } else { (v2(360.0 + (i as f32 - 0.5) * 300.0, y), 112.0) }
    }

    fn enter_reveal(&mut self, ctx: &mut Ctx, loaves: Vec<Loaf>) {
        self.btns.clear();
        self.meter.set_visible(false);
        self.clear_reveal();
        let mut rn = self.rn.clone();
        let c = self.card_rect(ctx);
        let card = list(|dl| {
            props::ticket(dl, c.w, c.h, Ink::Yellow);
            let y = -c.h * 0.5 + 64.0 + (c.h - 64.0) * 0.62;
            props::dashed(dl, v2(-c.w * 0.5 + 34.0, y), v2(c.w * 0.5 - 34.0, y), 8.0, 7.0, 2.0, 0.55);
            proof_core::art::twinkle(dl, v2(-c.w * 0.5 + 44.0, -c.h * 0.5 + 60.0), 12.0, Ink::Pink);
            proof_core::art::twinkle(dl, v2(c.w * 0.5 - 44.0, -c.h * 0.5 + 60.0), 12.0, Ink::Yellow);
        });
        self.reveal.push(ctx.riso.art(&mut rn, c.center(), &card));
        self.reveal_labels.push(
            ctx.riso.text(
                &mut rn,
                &TextSpec::new("Fresh from Toasty!", rect(c.x + 60.0, c.y + 34.0, c.w - 120.0, 48.0), 32.0)
                    .bold(),
            ),
        );
        let n = loaves.len();
        for (i, l) in loaves.iter().enumerate() {
            let (p, r) = self.loaf_spot(ctx, i, n);
            let mut a = ctx.riso.art(&mut rn, p, &DrawList::new());
            a.set_scale(0.2);
            self.reveal.push(a);
            let (tw, size) = if n == 1 { (540.0, 28.0) } else { (290.0, 23.0) };
            let title = TextSpec::new(l.title(), rect(p.x - tw * 0.5, p.y + r * 1.12 + 10.0, tw, 70.0), size)
                .bold()
                .wrap();
            self.reveal_labels.push(ctx.riso.text(&mut rn, &title));
        }
        let best = loaves.iter().max_by(|a, b| a.quality.total_cmp(&b.quality)).cloned();
        if let Some(b) = best {
            let yd = c.y + 64.0 + (c.h - 64.0) * 0.62;
            let ph = (c.y + c.h - 30.0 - yd - 24.0).clamp(150.0, 210.0);
            let pw = ph * 1.2;
            let pc = v2(c.x + 34.0 + pw * 0.5 + 10.0, yd + 24.0 + ph * 0.5);
            let photo = list(|dl| {
                dl.with(Xf::IDENTITY.rotated(-0.04), |dl| {
                    props::photo(dl, pw, ph, Ink::Blue);
                    let k = (pw - 44.0) / 230.0;
                    dl.with(Xf::at(v2(0.0, -16.0 + 35.0 * k)).scaled(k), |dl| {
                        crumb_slice(dl, 200.0, 130.0, b.openness, b.crust, b.seed)
                    });
                })
            });
            self.reveal.push(ctx.riso.art(&mut rn, pc, &photo));
            let word = if b.openness > 0.7 {
                "open & airy!"
            } else if b.openness > 0.45 {
                "soft & even"
            } else {
                "a bit tight"
            };
            let tx = pc.x + pw * 0.5 + 26.0;
            self.reveal_labels.push(
                ctx.riso.text(
                    &mut rn,
                    &TextSpec::new(
                        format!("Crumb from {}", b.starter),
                        rect(tx, pc.y - 52.0, c.x + c.w - 30.0 - tx, 40.0),
                        22.0,
                    )
                    .left()
                    .plain(),
                ),
            );
            self.reveal_labels.push(ctx.riso.text(
                &mut rn,
                &TextSpec::new(word, rect(tx, pc.y - 14.0, c.x + c.w - 30.0 - tx, 48.0), 30.0).bold().left(),
            ));
        }
        let (_, pri) = ui::bar_rects(self.bar, false);
        let next = Button::pill(ctx, &mut rn, pri, "Yay!", Ink::Pink);
        self.btns.add(Btn::Next, next);
        self.step = Step::Reveal { loaves, t: 0.0, stamped: false };
        self.reveal_shot = false;
        self.set_prompt(ctx, "");
        ctx.sfx(Sfx::Ding);
        ctx.kick(0.8);
    }

    fn update_reveal(&mut self, ctx: &mut Ctx, loaves: &[Loaf], t: f32, stamped: &mut bool) {
        let n = loaves.len();
        let bloom = ease_out_cubic((t / 1.0).min(1.0));
        let q = (bloom * 10.0).round() / 10.0;
        for (i, l) in loaves.iter().enumerate() {
            let (_, r) = self.loaf_spot(ctx, i, n);
            let mut v: LoafView = l.view(r);
            v.spring = l.spring * q;
            for c in &mut v.cuts {
                c.bloom *= q;
                c.ear *= q;
            }
            let a = &mut self.reveal[1 + i];
            a.set(ctx.riso, &list(|dl| loaf_top(dl, &v)));
            a.set_scale(0.2 + 0.8 * ease_out_elastic((t / 1.1).min(1.0)));
        }
        if !*stamped && t > 1.05 {
            *stamped = true;
            let mut rn = self.rn.clone();
            for (i, l) in loaves.iter().enumerate() {
                let (p, r) = self.loaf_spot(ctx, i, n);
                let stars = l.stars as u32;
                let seed = l.seed;
                let sr = if n == 1 { 64.0 } else { 52.0 };
                let st = list_at(Xf::IDENTITY.rotated(-0.22 + 0.12 * i as f32), |dl| {
                    props::stamp(dl, sr, stars, Ink::Pink, seed)
                });
                let at = p + v2(r * 0.95, r * 0.7);
                let mut a = ctx.riso.art(&mut rn, at, &st);
                a.set_scale(1.6);
                self.stamps.push(a);
            }
            ctx.sfx(Sfx::Stamp);
            ctx.buzz(35);
            ctx.kick(0.7);
            if loaves.iter().any(|l| l.stars >= 3) {
                ctx.sfx(Sfx::Sparkle);
            }
        }
        if *stamped {
            let k = ease_out_back(((t - 1.05) / 0.25).clamp(0.0, 1.0));
            for a in self.stamps.iter_mut() {
                a.set_scale(1.6 - 0.6 * k);
            }
        }
    }

    fn after_baking(&mut self, ctx: &mut Ctx) {
        self.clear_reveal();
        self.clear_trays();
        self.btns.clear();
        self.clear_bannetons();
        self.oven.set_visible(false);
        self.board.set_visible(false);
        self.dough.set_visible(false);
        self.guide_art.set_visible(false);
        let treats = ctx.state.unlocked_treats();
        if !treats.is_empty() && ctx.state.discard >= TRAY_COST {
            self.enter_treats(ctx, treats);
        } else {
            self.enter_done(ctx);
        }
    }

    // --- Treats ------------------------------------------------------------------------------

    fn clear_tray(&mut self) {
        for a in self.tray.drain(..) {
            a.free();
        }
        for a in self.tray_props.drain(..) {
            a.free();
        }
        for mut l in self.work_labels.drain(..) {
            l.queue_free();
        }
    }

    fn enter_treats(&mut self, ctx: &mut Ctx, treats: Vec<Treat>) {
        self.btns.clear();
        self.clear_tray();
        let mut rn = self.rn.clone();
        let n = treats.len() as f32;
        let y = self.dough_c.y - 20.0;
        for (i, t) in treats.iter().enumerate() {
            let x = 360.0 + (i as f32 - (n - 1.0) * 0.5) * 190.0;
            let b = Button::round(ctx, &mut rn, v2(x, y), 62.0, Ink::Yellow, Icon::Treat(*t));
            self.btns.add(Btn::Treat(*t), b);
            self.work_labels.push(ctx.riso.text(
                &mut rn,
                &TextSpec::new(t.name(), rect(x - 95.0, y + 76.0, 190.0, 60.0), 22.0).bold().wrap(),
            ));
        }
        let (_, pri) = ui::bar_rects(self.bar, false);
        let skip = Button::pill(ctx, &mut rn, pri, "Skip treats", Ink::Blue);
        self.btns.add(Btn::SkipTreats, skip);
        self.step = Step::Treats { kind: None, done: [false; 4] };
        self.set_prompt(ctx, &format!("Turn {} spoons of discard into treats?", TRAY_COST));
        ctx.note(
            "treats",
            "Every feeding leaves a spoon of discard. Two spoons make a whole tray of treats!",
        );
    }

    fn tray_spot(&self, i: usize) -> V2 {
        v2(360.0 + (i as f32 - 1.5) * 142.0, self.dough_c.y)
    }

    fn start_tray(&mut self, ctx: &mut Ctx, kind: Treat) {
        self.btns.clear();
        self.clear_tray();
        let mut rn = self.rn.clone();
        let pan = list(|dl| props::sheet_pan(dl, 620.0, 200.0));
        self.tray_props.push(ctx.riso.art(&mut rn, self.dough_c + v2(0.0, 8.0), &pan));
        for i in 0..4 {
            let a =
                ctx.riso.art(&mut rn, self.tray_spot(i), &list(|dl| treat_raw(dl, kind, 124.0, i as u32)));
            self.tray.push(a);
        }
        self.step = Step::Treats { kind: Some(kind), done: [false; 4] };
        self.set_prompt(
            ctx,
            match kind {
                Treat::Muffin => "Swipe across to bake the muffins!",
                Treat::CinnamonBun => "Swipe across to drizzle the icing!",
                Treat::Bagel => "Swipe across to seed the bagels!",
            },
        );
    }

    fn touch_tray(&mut self, ctx: &mut Ctx, p: V2) {
        if let Step::Treats { kind: Some(kind), done } = &mut self.step {
            for (i, a) in self.tray.iter_mut().enumerate() {
                if !done[i] && a.pos().dist(p) < 72.0 {
                    done[i] = true;
                    let k = *kind;
                    a.set(ctx.riso, &list(|dl| treat(dl, k, 124.0, i as u32)));
                    a.set_scale(1.15);
                    ctx.sfx_pitch(Sfx::Plop, 1.0 + i as f32 * 0.12);
                }
            }
        }
    }

    fn finish_tray(&mut self, ctx: &mut Ctx) {
        if let Step::Treats { kind: Some(kind), done } = self.step.clone() {
            let q = done.iter().filter(|d| **d).count() as f32 / 4.0;
            if q > 0.0 {
                ctx.act(proof_core::state::Action::MakeTray { treat: kind, quality: q });
            }
        }
    }

    // --- Done --------------------------------------------------------------------------------

    fn enter_done(&mut self, ctx: &mut Ctx) {
        self.btns.clear();
        self.clear_tray();
        let mut rn = self.rn.clone();
        let (_, pri) = ui::bar_rects(self.bar, false);
        let open = Button::pill(ctx, &mut rn, pri, "Open the shop!", Ink::Pink);
        self.btns.add(Btn::OpenShop, open);
        let n = ctx.state.shelf.len();
        self.set_prompt(ctx, &format!("The shelf is stocked: {n} goodies!"));
        // Everything cools on racks on the bench: loaves on top, treats below.
        let goods = ctx.state.shelf.clone();
        let loaves: Vec<_> = goods
            .iter()
            .filter_map(|g| if let proof_core::bake::Good::Loaf(l) = g { Some(l.clone()) } else { None })
            .collect();
        let treats: Vec<_> = goods
            .iter()
            .filter_map(|g| if let proof_core::bake::Good::Treat(t) = g { Some(t.clone()) } else { None })
            .collect();
        let per = 3usize;
        let rows = loaves.len().div_ceil(per).max(1);
        let has_treats = !treats.is_empty();
        // Centre the whole spread (racks + pan) on the free bench between the wall and the
        // action bar.
        let bench_mid = (self.lay.dough.y - self.lay.board_r + self.bar - ui::PILL_H) * 0.5 + 20.0;
        let spread = rows as f32 * 190.0 + if has_treats { 170.0 } else { 0.0 };
        // Taller benches get a slightly bigger spread.
        let avail = (self.bar - ui::PILL_H * 0.5 - 30.0) - (self.lay.dough.y - self.lay.board_r);
        let zoom = (avail / (spread + 240.0)).clamp(1.0, 1.25);
        let top_y = -spread * 0.5 + 25.0;
        let preview = list(|dl| {
            dl.with(Xf::at(v2(360.0, bench_mid)).scaled(zoom), |dl| {
                for row in 0..rows {
                    let in_row = (loaves.len() - row * per).min(per);
                    let y = top_y + row as f32 * 190.0;
                    if in_row > 0 {
                        dl.with(Xf::at(v2(0.0, y + 40.0)), |dl| {
                            props::cooling_rack(dl, in_row as f32 * 200.0 + 30.0, 150.0)
                        });
                    }
                    for k in 0..in_row {
                        let l = &loaves[row * per + k];
                        let x = (k as f32 - (in_row as f32 - 1.0) * 0.5) * 200.0;
                        dl.with(Xf::at(v2(x, y + 30.0)), |dl| loaf_top(dl, &l.view(80.0)));
                    }
                }
                if has_treats {
                    let nt = treats.len().min(6);
                    let y = top_y + rows as f32 * 190.0 + 40.0;
                    dl.with(Xf::at(v2(0.0, y + 10.0)), |dl| {
                        props::sheet_pan(dl, nt as f32 * 104.0 + 60.0, 130.0)
                    });
                    for (i, t) in treats.iter().take(6).enumerate() {
                        let x = (i as f32 - (nt as f32 - 1.0) * 0.5) * 104.0;
                        dl.with(Xf::at(v2(x, y)), |dl| treat(dl, t.kind, 96.0, t.seed));
                    }
                }
            })
        });
        let a = ctx.riso.art(&mut rn, V2::ZERO, &preview);
        self.tray.push(a);
        self.step = Step::Done;
        self.done_shot = false;
        ctx.sfx(Sfx::Sparkle);
    }

    fn click(&mut self, ctx: &mut Ctx, b: Btn) {
        match b {
            Btn::Undo => {
                if self.cuts.pop().is_some() {
                    if let Some(id) = self.sel {
                        ctx.act(proof_core::state::Action::Score {
                            dough: id,
                            cuts: self.cuts.clone(),
                            guide: self.guide,
                        });
                    }
                    ctx.sfx(Sfx::Squish);
                    self.redraw_dough(ctx);
                }
            }
            Btn::ToOven => self.mark_ready(ctx, false),
            Btn::BakeNow => {
                let ids = std::mem::take(&mut self.ready);
                self.enter_oven(ctx, ids);
            }
            Btn::Pull => self.pull(ctx),
            Btn::Next => {
                if matches!(self.step, Step::Reveal { t, .. } if t > 0.6) {
                    self.enter_work(ctx);
                }
            }
            Btn::Treat(t) => self.start_tray(ctx, t),
            Btn::SkipTreats => self.enter_done(ctx),
            Btn::OpenShop => ctx.act(proof_core::state::Action::OpenShop),
        }
    }

    fn click_tool(&mut self, ctx: &mut Ctx, t: Tool) {
        match t {
            Tool::Stencil(s) => {
                self.stencil = s;
                if s.is_none() {
                    self.topping = None;
                }
            }
            Tool::Topping(tp) => self.topping = if self.topping == tp { None } else { tp },
            Tool::Guide(g) => self.guide = g,
        }
        if let Some(id) = self.sel {
            ctx.act(proof_core::state::Action::Dress {
                dough: id,
                stencil: self.stencil,
                topping: self.topping,
            });
            if matches!(t, Tool::Guide(_)) {
                ctx.act(proof_core::state::Action::Score {
                    dough: id,
                    cuts: self.cuts.clone(),
                    guide: self.guide,
                });
            }
        }
        if matches!(t, Tool::Stencil(Some(_)) | Tool::Topping(Some(_))) {
            ctx.sfx(Sfx::Pour);
        }
        self.dough_bounce.pos = 0.92;
        self.sync_tools(ctx);
        self.redraw_dough(ctx);
    }

    fn pull(&mut self, ctx: &mut Ctx) {
        if let Step::Oven { ids, t, pulled } = &mut self.step {
            if *pulled {
                return;
            }
            *pulled = true;
            let crust = t.min(1.0);
            ctx.act(proof_core::state::Action::Bake { doughs: ids.clone(), crust });
            ctx.sfx(Sfx::Ding);
        }
    }

    /// Visitors' wants today (helps autopilot decorate sensibly).
    fn todays_wants(ctx: &Ctx) -> Vec<Want> {
        ctx.state.tomorrow.iter().flat_map(|v| v.order.wants.clone()).collect()
    }
}

impl Screen for Morning {
    fn root(&self) -> Gd<Node2D> {
        self.root.clone()
    }

    fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        self.hud.refresh(ctx.riso, ctx.state);
        self.tools.update(ctx, dt);
        self.btns.update(ctx, dt);
        let s = self.dough_bounce.step(dt, 300.0, 12.0);
        self.dough.set_scale(s);
        self.floaters.retain_mut(|f| f.update(dt));
        match self.step.clone() {
            Step::Peek(t) => {
                let t = t + dt;
                let k = ease_out_cubic((t / 1.4).min(1.0));
                let js = self.jar_scale;
                for (i, a) in self.jars.iter_mut().enumerate() {
                    if let Some(s) = ctx.state.starters.get(i) {
                        let mut v = s.view((t * 4.0).floor() / 4.0);
                        v.rise = v.band + (s.rise - v.band) * ((k * 8.0).floor() / 8.0);
                        a.set(ctx.riso, &list_at(Xf::IDENTITY.scaled(js), |dl| jar(dl, &v)));
                    }
                }
                if t > 1.8 {
                    ctx.note(
                        "welcome",
                        "Welcome to your bakery, sweet pea! I left Bubbles and two doughs in the fridge. Bake them, then open the shop.",
                    );
                    self.enter_work(ctx);
                } else {
                    self.step = Step::Peek(t);
                }
            }
            Step::Oven { ids, t, pulled } => {
                let nt = if pulled { t } else { t + dt / OVEN_SECS };
                self.draw_oven(ctx, nt.min(1.0), &ids, 0.0, if nt > 0.9 { Expr::Wow } else { Expr::Happy });
                if !pulled && nt >= 1.0 {
                    ctx.toast("Toasty: that's plenty, dear!");
                    self.step = Step::Oven { ids, t: 1.0, pulled: false };
                    self.pull(ctx);
                } else if let Step::Oven { t, .. } = &mut self.step {
                    *t = nt;
                }
            }
            Step::Reveal { loaves, t, mut stamped } => {
                let nt = t + dt;
                self.update_reveal(ctx, &loaves, nt, &mut stamped);
                self.step = Step::Reveal { loaves, t: nt, stamped };
            }
            _ => {}
        }
        for a in &mut self.tray {
            let s = a.node.get_scale().x;
            if s > 1.0 {
                a.set_scale((s - dt * 0.8).max(1.0));
            }
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, p: Ptr) {
        match p {
            Ptr::Down(q) => {
                if self.btns.down(ctx, q) || self.tools.down(ctx, q) {
                    return;
                }
                match &self.step {
                    Step::Work if self.inside_dough(ctx, q) => {
                        self.cutting = true;
                        self.stroke = vec![q];
                        self.stroke_t0 = ctx.time;
                        self.line.clear_points();
                        self.line.add_point(vec2(q));
                    }
                    Step::Oven { .. } if (self.oven.pos() + v2(0.0, -180.0)).dist(q) < 220.0 => {
                        self.pull(ctx)
                    }
                    Step::Treats { kind: Some(_), .. } => {
                        self.cutting = true;
                        self.touch_tray(ctx, q);
                    }
                    _ => {}
                }
            }
            Ptr::Move(q) => {
                if !self.cutting {
                    return;
                }
                if matches!(self.step, Step::Treats { .. }) {
                    self.touch_tray(ctx, q);
                    return;
                }
                if self.stroke.last().is_none_or(|l| l.dist(q) > 3.0) {
                    self.stroke.push(q);
                    self.line.add_point(vec2(q));
                }
            }
            Ptr::Up(q) => {
                if let Some(b) = self.btns.up(ctx, q) {
                    self.click(ctx, b);
                    return;
                }
                if let Some(t) = self.tools.up(ctx, q) {
                    self.click_tool(ctx, t);
                    return;
                }
                if !self.cutting {
                    return;
                }
                self.cutting = false;
                if matches!(self.step, Step::Treats { .. }) {
                    self.finish_tray(ctx);
                    return;
                }
                self.stroke.push(q);
                let pts = std::mem::take(&mut self.stroke);
                self.line.clear_points();
                let secs = ctx.time - self.stroke_t0;
                self.finish_cut(ctx, pts, secs);
            }
        }
    }

    fn events(&mut self, ctx: &mut Ctx, events: &[Event]) {
        let prints = events.iter().filter(|e| matches!(e, Event::NewPrint { .. })).count();
        if prints > 0 {
            ctx.toast(if prints == 1 {
                "New print for your Loaf Zine!".to_string()
            } else {
                format!("{prints} new prints for your Loaf Zine!")
            });
        }
        for e in events {
            match e {
                Event::Baked { loaves } => self.enter_reveal(ctx, loaves.clone()),
                Event::TrayMade { treat, count } => {
                    let mut rn = self.rn.clone();
                    self.floaters.push(Floater::new(
                        ctx.riso,
                        &mut rn,
                        &format!("+{count} {}s!", treat.name().to_lowercase()),
                        self.dough_c + v2(0.0, -150.0),
                        34.0,
                        Ink::Key,
                    ));
                    ctx.sfx(Sfx::Sparkle);
                    self.enter_done(ctx);
                }
                _ => {}
            }
        }
    }

    fn rejected(&mut self, ctx: &mut Ctx, r: proof_core::state::Reject) {
        ctx.toast(r.message());
        if matches!(self.step, Step::Treats { .. }) {
            self.enter_done(ctx);
        }
    }

    fn auto(&mut self, ctx: &mut Ctx) -> Option<String> {
        match self.step.clone() {
            Step::Peek(_) => None,
            Step::Work => {
                let id = self.sel?;
                if self.cuts.is_empty() {
                    let wants = Morning::todays_wants(ctx);
                    let idx = ctx.state.fridge.iter().position(|d| d.id == id).unwrap_or(0);
                    let want = wants.get(idx).copied();
                    if let Some(Want::Stencil(s)) = want {
                        self.click_tool(ctx, Tool::Stencil(Some(s)));
                    } else if let Some(Want::Topping(t)) = want {
                        self.click_tool(ctx, Tool::Topping(Some(t)));
                    } else if idx % 2 == 0 {
                        let s = ctx.state.unlocked_stencils();
                        self.click_tool(ctx, Tool::Stencil(s.last().copied()));
                    }
                    let pats = ctx.state.unlocked_patterns();
                    let p = match want {
                        Some(Want::Pattern(p)) => p,
                        _ => pats[(ctx.state.day as usize + idx) % pats.len()],
                    };
                    self.click_tool(ctx, Tool::Guide(Some(p)));
                    let shape = self.dough_shape(ctx);
                    for s in template(p, shape) {
                        let pts: Vec<V2> =
                            resample(&s, 12).into_iter().map(|q| self.dough_c + q * R).collect();
                        self.finish_cut(ctx, pts, 0.25);
                    }
                    Some("score".into())
                } else {
                    self.mark_ready(ctx, false);
                    None
                }
            }
            Step::Oven { t, pulled, .. } => {
                let wants = Morning::todays_wants(ctx);
                let target = wants
                    .iter()
                    .find_map(|w| if let Want::Crust(c) = w { Some(c.shade()) } else { None })
                    .unwrap_or(0.55);
                if !pulled && t >= target {
                    self.pull(ctx);
                    return Some("oven".into());
                }
                if !self.oven_shot && t > 0.25 {
                    self.oven_shot = true;
                    return Some("baking".into());
                }
                None
            }
            Step::Reveal { t, .. } => {
                if t > 1.5 && !self.reveal_shot {
                    // Photograph the settled card first, then move on.
                    self.reveal_shot = true;
                    Some("reveal".into())
                } else if self.reveal_shot {
                    self.enter_work(ctx);
                    None
                } else {
                    None
                }
            }
            Step::Treats { kind: None, .. } => {
                let treats = ctx.state.unlocked_treats();
                let t = treats[ctx.state.day as usize % treats.len()];
                self.start_tray(ctx, t);
                Some("tray".into())
            }
            Step::Treats { kind: Some(_), .. } => {
                let spots: Vec<V2> = self.tray.iter().map(|a| a.pos()).collect();
                for p in spots {
                    self.touch_tray(ctx, p);
                }
                self.finish_tray(ctx);
                Some("treats".into())
            }
            Step::Done => {
                if !self.done_shot {
                    self.done_shot = true;
                    return Some("stocked".into());
                }
                ctx.act(proof_core::state::Action::OpenShop);
                None
            }
        }
    }
}
