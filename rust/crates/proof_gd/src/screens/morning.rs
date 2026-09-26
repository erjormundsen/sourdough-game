//! Morning: wake-up peek → dress & score each dough → Toasty's oven → the reveal → treats.

use super::{Ctx, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list, list_at, vec2};
use crate::sfx::Sfx;
use crate::ui::{Button, Floater, Hud, Panel};
use godot::classes::line_2d::{LineCapMode, LineJointMode};
use godot::classes::{Label, Line2D, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::{Spring, ease_out_back, ease_out_cubic, ease_out_elastic};
use proof_core::art::bread::{LoafView, crumb_slice, loaf_top};
use proof_core::art::icons::Icon;
use proof_core::art::jar::jar;
use proof_core::art::oven::{OvenView, oven};
use proof_core::art::scenes::{Backdrop, backdrop, counter_front, counter_top};
use proof_core::art::treats::{treat, treat_raw};
use proof_core::art::{Expr, props};
use proof_core::bake::Loaf;
use proof_core::content::{Pattern, Shape, Stencil, Topping, Treat};
use proof_core::customer::Want;
use proof_core::geom::{V2, Xf, polyline_len, rect, resample, v2};
use proof_core::ink::Ink;
use proof_core::scoring::template;
use proof_core::state::{Event, TRAY_COST};

const R: f32 = 165.0;
const OVEN_SECS: f32 = 6.5;

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
    step: Step,
    sel: Option<u32>,
    cuts: Vec<Vec<V2>>,
    stencil: Option<Stencil>,
    topping: Option<Topping>,
    guide: Option<Pattern>,
    dough: Art,
    dough_bounce: Spring,
    guide_art: Art,
    stroke: Vec<V2>,
    stroke_t0: f32,
    line: Gd<Line2D>,
    cutting: bool,
    tools: Panel<Tool>,
    btns: Panel<Btn>,
    bannetons: Vec<(u32, Art)>,
    ready: Vec<u32>,
    oven: Art,
    meter: Art,
    oven_q: i32,
    reveal: Vec<Art>,
    reveal_labels: Vec<Gd<Label>>,
    prompt: Gd<Label>,
    tray: Vec<Art>,
    work_labels: Vec<Gd<Label>>,
    oven_shot: bool,
    floaters: Vec<Floater>,
    counter_y: f32,
    dough_c: V2,
}

fn dough_center(h: f32) -> V2 {
    v2(360.0, 600.0 + (h - 1280.0) * 0.3)
}

impl Morning {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>) -> Morning {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let h = ctx.lay.h;
        let bg = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| backdrop(d, Backdrop::Bakehouse, h)));
        // Starter jars sit on the shelf for the wake-up peek.
        let mut jars = Vec::new();
        for (i, s) in ctx.state.starters.iter().enumerate() {
            let mut view = s.view(0.0);
            view.rise = view.band;
            let a = ctx.riso.art(
                &mut rn,
                v2(62.0 + i as f32 * 80.0, 304.0),
                &list_at(Xf::IDENTITY.scaled(0.42), |d| jar(d, &view)),
            );
            jars.push(a);
        }
        let counter_y = counter_top(Backdrop::Bakehouse, h);
        let dough = ctx.riso.art(&mut rn, dough_center(h), &DrawList::new());
        let guide_art = ctx.riso.art(&mut rn, dough_center(h), &DrawList::new());
        let oven = ctx.riso.art(&mut rn, v2(360.0, counter_y + 8.0), &DrawList::new());
        let meter = ctx.riso.art(&mut rn, v2(360.0, counter_y - 510.0), &DrawList::new());
        let front = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| counter_front(d, Backdrop::Bakehouse, h)));
        let mut line = Line2D::new_alloc();
        line.set_width(5.0);
        line.set_default_color(ctx.riso.ink(Ink::Key));
        line.set_joint_mode(LineJointMode::ROUND);
        line.set_begin_cap_mode(LineCapMode::ROUND);
        line.set_end_cap_mode(LineCapMode::ROUND);
        line.set_material(&ctx.riso.text_mat);
        rn.add_child(&line);
        let prompt = ctx.riso.text(&mut rn, &TextSpec::new("", rect(40.0, 318.0, 640.0, 60.0), 30.0).bold());
        let hud = Hud::new(ctx, &mut rn, "Dawn edition · bake");
        let mut m = Morning {
            root,
            rn,
            hud,
            _bg: bg,
            _front: front,
            jars,
            step: Step::Peek(0.0),
            sel: None,
            cuts: Vec::new(),
            stencil: None,
            topping: None,
            guide: None,
            dough,
            dough_bounce: Spring::new(1.0),
            guide_art,
            stroke: Vec::new(),
            stroke_t0: 0.0,
            line,
            cutting: false,
            tools: Panel::default(),
            btns: Panel::default(),
            bannetons: Vec::new(),
            ready: Vec::new(),
            oven,
            meter,
            oven_q: -1,
            reveal: Vec::new(),
            reveal_labels: Vec::new(),
            prompt,
            tray: Vec::new(),
            work_labels: Vec::new(),
            oven_shot: false,
            floaters: Vec::new(),
            counter_y,
            dough_c: dough_center(h),
        };
        m.set_prompt("Good morning! The jars woke up bubbly.");
        ctx.sfx(Sfx::Bubble);
        m
    }

    fn set_prompt(&mut self, t: &str) {
        self.prompt.set_text(t);
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
        self.set_prompt("Dress it, then swipe to score!");
        ctx.note(
            "score",
            "Swipe across the dough with your blade to score it. One bold slash makes a big ear. Try a pattern if you like!",
        );
    }

    fn build_work_ui(&mut self, ctx: &mut Ctx) {
        self.tools.clear();
        self.btns.clear();
        let y1 = self.dough_c.y + R + 72.0;
        let y2 = y1 + 80.0;
        let mut x = 150.0;
        let add = |m: &mut Morning, ctx: &mut Ctx, tool: Tool, ic: Icon, y: f32, x: &mut f32| {
            let b = Button::round(ctx, &mut m.rn, v2(*x, y), 31.0, Ink::Yellow, ic);
            m.tools.add(tool, b);
            *x += 70.0;
        };
        add(self, ctx, Tool::Stencil(None), Icon::None, y1, &mut x);
        for s in ctx.state.unlocked_stencils() {
            add(self, ctx, Tool::Stencil(Some(s)), Icon::Stencil(s), y1, &mut x);
        }
        for t in ctx.state.unlocked_toppings() {
            add(self, ctx, Tool::Topping(Some(t)), Icon::Topping(t), y1, &mut x);
        }
        let mut x = 150.0;
        add(self, ctx, Tool::Guide(None), Icon::Freehand, y2, &mut x);
        for p in ctx.state.unlocked_patterns() {
            add(self, ctx, Tool::Guide(Some(p)), Icon::Pattern(p), y2, &mut x);
        }
        let mut rn = self.rn.clone();
        for mut l in self.work_labels.drain(..) {
            l.queue_free();
        }
        self.work_labels.push(
            ctx.riso.text(
                &mut rn,
                &TextSpec::new("Dress", rect(10.0, y1 - 22.0, 104.0, 44.0), 24.0).bold().right(),
            ),
        );
        self.work_labels.push(
            ctx.riso.text(
                &mut rn,
                &TextSpec::new("Score", rect(10.0, y2 - 22.0, 104.0, 44.0), 24.0).bold().right(),
            ),
        );
        let undo =
            Button::round(ctx, &mut rn, v2(640.0, self.dough_c.y - 120.0), 34.0, Ink::Blue, Icon::Undo);
        self.btns.add(Btn::Undo, undo);
        let h = ctx.lay.h;
        let to = Button::pill(ctx, &mut rn, rect(190.0, h - 104.0, 340.0, 80.0), "Into the oven!", Ink::Pink);
        self.btns.add(Btn::ToOven, to);
        if !self.ready.is_empty() {
            let now = Button::pill(ctx, &mut rn, rect(540.0, h - 96.0, 160.0, 64.0), "Bake 1", Ink::Yellow);
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

    fn build_bannetons(&mut self, ctx: &mut Ctx) {
        for (_, a) in self.bannetons.drain(..) {
            a.free();
        }
        let y = self.counter_y + 88.0;
        let mut rn = self.rn.clone();
        for (i, d) in ctx.state.fridge.iter().enumerate() {
            let full = Some(d.id) != self.sel;
            let ready = self.ready.contains(&d.id);
            let shape = d.shape;
            let art = list(|dl| {
                props::banneton(dl, shape, 44.0, full);
                if ready {
                    proof_core::art::twinkle(dl, v2(40.0, -34.0), 12.0, Ink::Yellow);
                }
            });
            let a = ctx.riso.art(&mut rn, v2(80.0 + i as f32 * 104.0, y), &art);
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

    fn clear_work(&mut self) {
        self.tools.clear();
        for mut l in self.work_labels.drain(..) {
            l.queue_free();
        }
    }

    fn enter_oven(&mut self, ctx: &mut Ctx, ids: Vec<u32>) {
        self.clear_work();
        self.btns.clear();
        for (_, a) in self.bannetons.drain(..) {
            a.free();
        }
        self.dough.set_visible(false);
        self.guide_art.set_visible(false);
        self.oven.set_visible(true);
        self.meter.set_visible(true);
        self.oven.set_scale(1.25);
        self.oven_q = -1;
        let mut rn = self.rn.clone();
        let h = ctx.lay.h;
        let pull = Button::pill(ctx, &mut rn, rect(210.0, h - 104.0, 300.0, 80.0), "Pull!", Ink::Pink);
        self.btns.add(Btn::Pull, pull);
        self.step = Step::Oven { ids, t: 0.0, pulled: false };
        self.set_prompt("Pull when the crust looks just right");
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
        let d = list(|dl| {
            oven(dl, &view, |dl| {
                let n = doughs.len().max(1) as f32;
                for (i, dough) in doughs.iter().enumerate() {
                    let x = (i as f32 - (n - 1.0) * 0.5) * 92.0;
                    let mut v = dough.view(40.0);
                    v.bake = crust.min(1.0);
                    v.crust = crust;
                    v.spring = crust;
                    for c in &mut v.cuts {
                        c.bloom = crust * 0.8;
                        c.ear = crust * 0.5;
                    }
                    dl.with(Xf::at(v2(x, -150.0)), |dl| loaf_top(dl, &v));
                }
            })
        });
        self.oven.set(ctx.riso, &d);
        let meter = list(|dl| {
            let w = 520.0;
            let track = proof_core::geom::rounded_rect(rect(-w * 0.5, -22.0, w, 44.0), 22.0);
            dl.backing(&track);
            for (i, (label_ink, tone)) in
                [(Ink::Yellow, 0.45), (Ink::Yellow, 0.9), (Ink::Pink, 0.75)].iter().enumerate()
            {
                let x0 = -w * 0.5 + i as f32 * w / 3.0;
                let seg = proof_core::geom::rect_poly(rect(x0, -22.0, w / 3.0, 44.0));
                dl.clipped(&track, |dl| {
                    dl.fill(*label_ink, *tone, &seg);
                    if i == 2 {
                        dl.fill(Ink::Yellow, 0.9, &seg);
                        dl.ht(Ink::Key, 0.25, &seg);
                    }
                });
            }
            dl.outline(Ink::Key, 4.0, &track);
            let mx = -w * 0.5 + w * t.min(1.0);
            let marker = vec![v2(mx, -30.0), v2(mx - 16.0, -58.0), v2(mx + 16.0, -58.0)];
            dl.fill(Ink::Pink, 1.0, &marker);
            dl.outline(Ink::Key, 3.5, &marker);
            dl.line(Ink::Key, 5.0, &[v2(mx, -24.0), v2(mx, 24.0)]);
        });
        self.meter.set(ctx.riso, &meter);
    }

    // --- Reveal ------------------------------------------------------------------------------

    fn clear_reveal(&mut self) {
        for a in self.reveal.drain(..) {
            a.free();
        }
        for mut l in self.reveal_labels.drain(..) {
            l.queue_free();
        }
    }

    fn enter_reveal(&mut self, ctx: &mut Ctx, loaves: Vec<Loaf>) {
        self.btns.clear();
        self.meter.set_visible(false);
        self.clear_reveal();
        let mut rn = self.rn.clone();
        let h = ctx.lay.h;
        let card = list(|dl| props::ticket(dl, 660.0, 800.0, Ink::Yellow));
        let card_c = v2(360.0, 150.0 + 400.0 + (h - 1280.0) * 0.2);
        self.reveal.push(ctx.riso.art(&mut rn, card_c, &card));
        let n = loaves.len();
        for (i, l) in loaves.iter().enumerate() {
            let x = if n == 1 { 360.0 } else { 200.0 + i as f32 * 320.0 };
            let mut a = ctx.riso.art(&mut rn, v2(x, card_c.y - 110.0), &DrawList::new());
            a.set_scale(0.2);
            self.reveal.push(a);
            let title =
                TextSpec::new(l.title(), rect(x - 160.0, card_c.y + 70.0, 320.0, 80.0), 25.0).bold().wrap();
            self.reveal_labels.push(ctx.riso.text(&mut rn, &title));
            let _ = l;
        }
        let best = loaves.iter().max_by(|a, b| a.quality.total_cmp(&b.quality)).cloned();
        if let Some(b) = best {
            let crumb = list(|dl| crumb_slice(dl, 200.0, 130.0, b.openness, b.crust, b.seed));
            let a = ctx.riso.art(&mut rn, v2(200.0, card_c.y + 250.0), &crumb);
            self.reveal.push(a);
            let txt = format!(
                "Crumb from {}:\n{}",
                b.starter,
                if b.openness > 0.7 {
                    "open & airy!"
                } else if b.openness > 0.45 {
                    "soft & even"
                } else {
                    "a bit tight"
                }
            );
            self.reveal_labels.push(ctx.riso.text(
                &mut rn,
                &TextSpec::new(txt, rect(330.0, card_c.y + 190.0, 300.0, 120.0), 24.0).left().wrap(),
            ));
        }
        let next = Button::pill(ctx, &mut rn, rect(210.0, h - 104.0, 300.0, 80.0), "Yay!", Ink::Pink);
        self.btns.add(Btn::Next, next);
        self.step = Step::Reveal { loaves, t: 0.0, stamped: false };
        self.set_prompt("");
        ctx.sfx(Sfx::Ding);
        ctx.kick(0.8);
    }

    fn update_reveal(&mut self, ctx: &mut Ctx, loaves: &[Loaf], t: f32, stamped: &mut bool) {
        let n = loaves.len();
        let bloom = ease_out_cubic((t / 1.0).min(1.0));
        let q = (bloom * 10.0).round() / 10.0;
        for (i, l) in loaves.iter().enumerate() {
            let r = if n == 1 { 165.0 } else { 118.0 };
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
            let loaf_y = self.reveal[1].pos().y;
            let mut rn = self.rn.clone();
            for (i, l) in loaves.iter().enumerate() {
                let lx = if n == 1 { 360.0 } else { 200.0 + i as f32 * 320.0 };
                let off = if n == 1 { 150.0 } else { 100.0 };
                let (x, card_y) = (lx + off, loaf_y + off + 250.0);
                let stars = l.stars as u32;
                let seed = l.seed;
                let st = list_at(Xf::IDENTITY.rotated(-0.25 + 0.1 * i as f32), |dl| {
                    props::stamp(dl, 62.0, stars, Ink::Pink, seed)
                });
                let mut a = ctx.riso.art(&mut rn, v2(x, card_y - 250.0), &st);
                a.set_scale(1.6);
                self.reveal.push(a);
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
            let base = 1 + n;
            for a in self.reveal.iter_mut().skip(base + if n > 0 { 1 } else { 0 }) {
                a.set_scale(1.6 - 0.6 * k);
            }
        }
    }

    fn after_baking(&mut self, ctx: &mut Ctx) {
        self.clear_reveal();
        self.clear_work();
        self.btns.clear();
        for (_, a) in self.bannetons.drain(..) {
            a.free();
        }
        self.oven.set_visible(false);
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

    fn enter_treats(&mut self, ctx: &mut Ctx, treats: Vec<Treat>) {
        self.btns.clear();
        let mut rn = self.rn.clone();
        let n = treats.len() as f32;
        for (i, t) in treats.iter().enumerate() {
            let x = 360.0 + (i as f32 - (n - 1.0) * 0.5) * 150.0;
            let b = Button::round(ctx, &mut rn, v2(x, 640.0), 58.0, Ink::Yellow, Icon::Treat(*t));
            self.btns.add(Btn::Treat(*t), b);
        }
        let h = ctx.lay.h;
        let skip = Button::pill(ctx, &mut rn, rect(230.0, h - 104.0, 260.0, 76.0), "Skip", Ink::Blue);
        self.btns.add(Btn::SkipTreats, skip);
        self.step = Step::Treats { kind: None, done: [false; 4] };
        self.set_prompt(&format!("Turn {} discard into a treat tray?", TRAY_COST));
        ctx.note(
            "treats",
            "Every feeding leaves a spoon of discard. Two spoons make a whole tray of treats!",
        );
    }

    fn start_tray(&mut self, ctx: &mut Ctx, kind: Treat) {
        self.btns.clear();
        let mut rn = self.rn.clone();
        for i in 0..4 {
            let a = ctx.riso.art(
                &mut rn,
                v2(135.0 + i as f32 * 150.0, 640.0),
                &list(|dl| treat_raw(dl, kind, 130.0, i)),
            );
            self.tray.push(a);
        }
        self.step = Step::Treats { kind: Some(kind), done: [false; 4] };
        self.set_prompt(match kind {
            Treat::Muffin => "Swipe across to bake the muffins!",
            Treat::CinnamonBun => "Swipe across to drizzle the icing!",
            Treat::Bagel => "Swipe across to seed the bagels!",
        });
    }

    fn touch_tray(&mut self, ctx: &mut Ctx, p: V2) {
        if let Step::Treats { kind: Some(kind), done } = &mut self.step {
            for (i, a) in self.tray.iter_mut().enumerate() {
                if !done[i] && a.pos().dist(p) < 75.0 {
                    done[i] = true;
                    let k = *kind;
                    a.set(ctx.riso, &list(|dl| treat(dl, k, 130.0, i as u32)));
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
        for a in self.tray.drain(..) {
            a.free();
        }
        let mut rn = self.rn.clone();
        let h = ctx.lay.h;
        let open =
            Button::pill(ctx, &mut rn, rect(180.0, h - 110.0, 360.0, 88.0), "Open the shop!", Ink::Pink);
        self.btns.add(Btn::OpenShop, open);
        let n = ctx.state.shelf.len();
        self.set_prompt(&format!("The shelf is stocked: {n} goodies!"));
        // A little preview of the shelf: loaves on top, treats below.
        let goods = ctx.state.shelf.clone();
        let loaves: Vec<_> = goods
            .iter()
            .filter_map(|g| if let proof_core::bake::Good::Loaf(l) = g { Some(l.clone()) } else { None })
            .collect();
        let treats: Vec<_> = goods
            .iter()
            .filter_map(|g| if let proof_core::bake::Good::Treat(t) = g { Some(t.clone()) } else { None })
            .collect();
        let preview = list(|dl| {
            let per = 3usize;
            for (i, l) in loaves.iter().enumerate() {
                let row = (i / per) as f32;
                let in_row = (loaves.len() - (i / per) * per).min(per) as f32;
                let x = ((i % per) as f32 - (in_row - 1.0) * 0.5) * 200.0;
                dl.with(Xf::at(v2(x, row * 190.0)), |dl| loaf_top(dl, &l.view(80.0)));
            }
            let ty = loaves.len().div_ceil(per) as f32 * 190.0 + 10.0;
            let nt = treats.len().min(6) as f32;
            for (i, t) in treats.iter().take(6).enumerate() {
                let x = (i as f32 - (nt - 1.0) * 0.5) * 100.0;
                dl.with(Xf::at(v2(x, ty)), |dl| treat(dl, t.kind, 96.0, t.seed));
            }
        });
        let a = ctx.riso.art(&mut rn, v2(360.0, 520.0), &preview);
        self.tray.push(a);
        self.step = Step::Done;
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

use proof_core::draw::DrawList;

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
                for (i, a) in self.jars.iter_mut().enumerate() {
                    if let Some(s) = ctx.state.starters.get(i) {
                        let mut v = s.view((t * 4.0).floor() / 4.0);
                        v.rise = v.band + (s.rise - v.band) * ((k * 8.0).floor() / 8.0);
                        a.set(ctx.riso, &list_at(Xf::IDENTITY.scaled(0.42), |dl| jar(dl, &v)));
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
                    Step::Oven { .. } if self.oven.pos().dist(q) < 220.0 => self.pull(ctx),
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
                        v2(360.0, 520.0),
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
                if t > 1.5 {
                    self.enter_work(ctx);
                    Some("reveal".into())
                } else {
                    None
                }
            }
            Step::Treats { kind: None, .. } => {
                let treats = ctx.state.unlocked_treats();
                let t = treats[ctx.state.day as usize % treats.len()];
                self.start_tray(ctx, t);
                None
            }
            Step::Treats { kind: Some(_), .. } => {
                for x in [135.0, 285.0, 435.0, 585.0] {
                    self.touch_tray(ctx, v2(x, 640.0));
                }
                self.finish_tray(ctx);
                Some("treats".into())
            }
            Step::Done => {
                ctx.act(proof_core::state::Action::OpenShop);
                Some("stocked".into())
            }
        }
    }
}
