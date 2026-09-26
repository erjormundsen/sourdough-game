//! Shop: regulars visit one at a time with icon orders; tap or drag a good onto them.

use super::{Ctx, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list};
use crate::sfx::Sfx;
use crate::ui::{Button, Floater, Hud, Panel};
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::{ease_in_out, ease_out_back};
use proof_core::art::bread::loaf_top;
use proof_core::art::critters::{CritterView, critter};
use proof_core::art::icons::{Icon, icon};
use proof_core::art::scenes::{Backdrop, backdrop, counter_front, counter_top};
use proof_core::art::treats::treat;
use proof_core::art::{Expr, props};
use proof_core::bake::Good;
use proof_core::content::Treat;
use proof_core::customer::{self, Reaction};
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf, rect, v2};
use proof_core::ink::Ink;
use proof_core::state::{Action, Event, ServeResult};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Btn {
    Hint,
    Sorry,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
enum Visitor {
    None,
    Entering(f32),
    Waiting,
    Reacting(f32, Reaction),
    Leaving(f32),
    Summary,
}

/// One display cell: a loaf, or a stack of the same treat.
struct Cell {
    key: CellKey,
    art: Art,
    home: V2,
    count: Option<Gd<Label>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CellKey {
    Loaf(u32),
    Treat(Treat),
}

pub struct Shop {
    root: Gd<Node2D>,
    rn: Gd<Node>,
    hud: Hud,
    _bg: Art,
    _front: Art,
    critter: Art,
    critter_expr: Expr,
    bubble: Art,
    bubble_text: Gd<Label>,
    state: Visitor,
    cells: Vec<Cell>,
    drag: Option<(usize, V2, bool)>,
    btns: Panel<Btn>,
    floaters: Vec<Floater>,
    fx: Vec<(Art, f32)>,
    summary: Vec<Art>,
    summary_labels: Vec<Gd<Label>>,
    counter_y: f32,
    stand: V2,
    t: f32,
    shown_order: bool,
}

impl Shop {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>) -> Shop {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let h = ctx.lay.h;
        let bg = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| backdrop(d, Backdrop::Shopfront, h)));
        let counter_y = counter_top(Backdrop::Shopfront, h);
        let stand = v2(360.0, counter_y + 40.0);
        let mut critter = ctx.riso.art(&mut rn, v2(1000.0, stand.y), &DrawList::new());
        critter.extra_scale = 1.3;
        critter.set_scale(1.3);
        let front = ctx.riso.art(&mut rn, V2::ZERO, &list(|d| counter_front(d, Backdrop::Shopfront, h)));
        let bubble_y = (stand.y - 610.0).max(340.0);
        let bubble = ctx.riso.art(&mut rn, v2(360.0, bubble_y), &DrawList::new());
        let mut bn = bubble.as_node();
        let bubble_text =
            ctx.riso.text(&mut bn, &TextSpec::new("", rect(-210.0, -92.0, 420.0, 70.0), 24.0).wrap());
        let hud = Hud::new(ctx, &mut rn, "Daylight edition · shop");
        let mut s = Shop {
            root,
            rn,
            hud,
            _bg: bg,
            _front: front,
            critter,
            critter_expr: Expr::Content,
            bubble,
            bubble_text,
            state: Visitor::None,
            cells: Vec::new(),
            drag: None,
            btns: Panel::default(),
            floaters: Vec::new(),
            fx: Vec::new(),
            summary: Vec::new(),
            summary_labels: Vec::new(),
            counter_y,
            stand,
            t: 0.0,
            shown_order: false,
        };
        s.build_cells(ctx);
        let mut rn2 = s.rn.clone();
        let hint_y = s.bubble.pos().y - 60.0;
        let hint = Button::round(ctx, &mut rn2, v2(612.0, hint_y), 28.0, Ink::Yellow, Icon::Book);
        s.btns.add(Btn::Hint, hint);
        let sorry =
            Button::pill(ctx, &mut rn2, rect(540.0, counter_y - 120.0, 160.0, 58.0), "Not today", Ink::Blue);
        s.btns.add(Btn::Sorry, sorry);
        s.bubble.set_visible(false);
        s.next_visitor(ctx);
        ctx.note(
            "shop",
            "Your regulars are here! Read their order bubble, then tap (or drag) a goodie onto them. Everyone pays — happy friends tip!",
        );
        s
    }

    fn visitor(&self, ctx: &Ctx) -> Option<proof_core::state::Visit> {
        ctx.state.current_visitor().cloned()
    }

    fn build_cells(&mut self, ctx: &mut Ctx) {
        for c in self.cells.drain(..) {
            c.art.free();
            if let Some(mut l) = c.count {
                l.queue_free();
            }
        }
        let mut keys: Vec<CellKey> = Vec::new();
        for g in &ctx.state.shelf {
            let k = match g {
                Good::Loaf(l) => CellKey::Loaf(l.id),
                Good::Treat(t) => CellKey::Treat(t.kind),
            };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        let mut rn = self.rn.clone();
        let top = self.counter_y + 110.0;
        for (i, k) in keys.iter().enumerate().take(8) {
            let home = v2(95.0 + (i % 4) as f32 * 176.0, top + (i / 4) as f32 * 150.0 - 10.0);
            let (d, count) = match k {
                CellKey::Loaf(id) => {
                    let l = ctx.state.shelf.iter().find_map(|g| match g {
                        Good::Loaf(l) if l.id == *id => Some(l.clone()),
                        _ => None,
                    });
                    (l.map(|l| list(|dl| loaf_top(dl, &l.view(62.0)))).unwrap_or_default(), 0)
                }
                CellKey::Treat(t) => {
                    let n = ctx
                        .state
                        .shelf
                        .iter()
                        .filter(|g| matches!(g, Good::Treat(x) if x.kind == *t))
                        .count();
                    let t = *t;
                    (list(|dl| treat(dl, t, 120.0, 1)), n)
                }
            };
            let art = ctx.riso.art(&mut rn, home, &d);
            let count = if count > 1 {
                let mut an = art.as_node();
                Some(ctx.riso.text(
                    &mut an,
                    &TextSpec::new(format!("×{count}"), rect(10.0, 20.0, 70.0, 40.0), 26.0).bold(),
                ))
            } else {
                None
            };
            self.cells.push(Cell { key: *k, art, home, count });
        }
    }

    fn shelf_index(&self, ctx: &Ctx, key: CellKey) -> Option<usize> {
        ctx.state.shelf.iter().position(|g| match (g, key) {
            (Good::Loaf(l), CellKey::Loaf(id)) => l.id == id,
            (Good::Treat(t), CellKey::Treat(k)) => t.kind == k,
            _ => false,
        })
    }

    fn draw_critter(&mut self, ctx: &mut Ctx) {
        let Some(v) = self.visitor(ctx) else { return };
        let view =
            CritterView { species: v.species, expr: self.critter_expr, t: (self.t * 3.0).floor() / 3.0 };
        self.critter.set(ctx.riso, &list(|d| critter(d, &view)));
    }

    fn draw_bubble(&mut self, ctx: &mut Ctx) {
        let Some(v) = self.visitor(ctx) else { return };
        let wants = v.order.wants.clone();
        let pre = v.preorder;
        let d = list(|d| {
            props::bubble(d, 460.0, 250.0, v2(-20.0, 190.0));
            let n = wants.len() as f32;
            for (i, w) in wants.iter().enumerate() {
                let x = (i as f32 - (n - 1.0) * 0.5) * 120.0;
                icon(d, Icon::Want(*w), v2(x, 40.0), 44.0);
            }
            if wants.len() == 2 {
                d.line(Ink::Key, 5.0, &[v2(-10.0, 40.0), v2(10.0, 40.0)]);
                d.line(Ink::Key, 5.0, &[v2(0.0, 30.0), v2(0.0, 50.0)]);
            }
            if pre {
                d.with(Xf::at(v2(-170.0, -118.0)).rotated(-0.12), |d| props::tape(d, 130.0, 34.0, Ink::Pink));
            }
        });
        self.bubble.set(ctx.riso, &d);
        let p = customer::profile(v.species);
        self.bubble_text.set_text(&format!("{}: “{}”", p.name, v.hello));
        self.bubble.set_visible(true);
        self.bubble.set_scale(0.3);
        let _ = pre;
    }

    fn next_visitor(&mut self, ctx: &mut Ctx) {
        if self.visitor(ctx).is_some() {
            self.critter_expr = Expr::Content;
            self.state = Visitor::Entering(0.0);
            self.draw_critter(ctx);
            self.bubble.set_visible(false);
            ctx.sfx(Sfx::Chirp);
            let sold_out = ctx.state.shelf.is_empty();
            if let Some(b) = self.btns.get(Btn::Sorry) {
                b.set_text(if sold_out { "Sold out!" } else { "Not today" });
            }
        } else {
            self.show_summary(ctx);
        }
    }

    fn serve(&mut self, ctx: &mut Ctx, cell: usize) {
        if self.state != Visitor::Waiting {
            return;
        }
        let key = self.cells[cell].key;
        if let Some(idx) = self.shelf_index(ctx, key) {
            ctx.act(Action::Serve { shelf_idx: idx });
        }
    }

    fn on_served(&mut self, ctx: &mut Ctx, r: &ServeResult) {
        self.critter_expr = match r.reaction {
            Reaction::Love => Expr::Excited,
            Reaction::Happy => Expr::Happy,
            _ => Expr::Content,
        };
        self.state = Visitor::Reacting(0.0, r.reaction);
        self.draw_critter(ctx);
        self.bubble.set_visible(false);
        let mut rn = self.rn.clone();
        let head = self.stand + v2(0.0, -250.0);
        let (word, s) = match r.reaction {
            Reaction::Love => ("Loved it!", Sfx::Sparkle),
            Reaction::Happy => ("Yum, thanks!", Sfx::Chirp),
            _ => ("Thank you!", Sfx::Plop),
        };
        self.floaters.push(Floater::new(ctx.riso, &mut rn, word, head + v2(0.0, -120.0), 38.0, Ink::Key));
        let coins = r.coins + r.tip;
        let mut f =
            Floater::new(ctx.riso, &mut rn, &format!("+{coins}"), head + v2(150.0, -40.0), 36.0, Ink::Pink);
        f.life = 1.6;
        self.floaters.push(f);
        ctx.sfx(Sfx::Coin);
        ctx.sfx(s);
        if r.hearts > 0 {
            let hearts = r.hearts;
            let burst = list(|d| {
                for i in 0..hearts * 3 {
                    let a = -2.4 + i as f32 * (1.6 / (hearts * 3) as f32);
                    props::heart_icon(d, V2::from_angle(a) * 150.0, 34.0);
                }
                props::burst(d, 110.0, 8, 2);
            });
            let a = ctx.riso.art(&mut rn, head, &burst);
            self.fx.push((a, 0.0));
            ctx.kick(0.5);
            ctx.buzz(20);
        }
        self.build_cells(ctx);
    }

    fn show_summary(&mut self, ctx: &mut Ctx) {
        self.state = Visitor::Summary;
        self.bubble.set_visible(false);
        self.critter.set_visible(false);
        if let Some(b) = self.btns.get(Btn::Sorry) {
            b.set_visible(false);
        }
        if let Some(b) = self.btns.get(Btn::Hint) {
            b.set_visible(false);
        }
        let s = ctx.state.today.clone();
        let mut rn = self.rn.clone();
        let y = 470.0;
        let card = list(|d| {
            props::ticket(d, 520.0, 520.0, Ink::Pink);
            props::coin(d, v2(-170.0, -80.0), 26.0);
            props::heart_icon(d, v2(-170.0, 10.0), 46.0);
            icon(d, Icon::Shop, v2(-170.0, 100.0), 32.0);
        });
        self.summary.push(ctx.riso.art(&mut rn, v2(360.0, y), &card));
        let lines = [
            ("Today's receipt".to_string(), -210.0, 30.0),
            (format!("{} coins  (+{} tips)", s.coins + s.tips, s.tips), -80.0, 28.0),
            (format!("{} hearts", s.hearts), 10.0, 28.0),
            (format!("{} served · {} loved it", s.served, s.loved), 100.0, 26.0),
        ];
        for (i, (t, dy, size)) in lines.iter().enumerate() {
            let spec = if i == 0 {
                TextSpec::new(t.clone(), rect(110.0, y + dy - 26.0, 500.0, 52.0), *size).bold()
            } else {
                TextSpec::new(t.clone(), rect(240.0, y + dy - 26.0, 360.0, 52.0), *size).left()
            };
            self.summary_labels.push(ctx.riso.text(&mut rn, &spec));
        }
        let h = ctx.lay.h;
        let close =
            Button::pill(ctx, &mut rn, rect(190.0, h - 110.0, 340.0, 86.0), "Close up shop", Ink::Pink);
        self.btns.add(Btn::Close, close);
        ctx.sfx(Sfx::Ding);
        ctx.kick(0.6);
    }

    fn cell_at(&self, p: V2) -> Option<usize> {
        self.cells.iter().position(|c| c.art.pos().dist(p) < 75.0)
    }

    fn over_critter(&self, p: V2) -> bool {
        (p - (self.stand + v2(0.0, -180.0))).len() < 190.0
    }

    fn click(&mut self, ctx: &mut Ctx, b: Btn) {
        match b {
            Btn::Hint => {
                if let Some(v) = self.visitor(ctx) {
                    let p = customer::profile(v.species);
                    ctx.toast(format!("{} wants: {}", p.name, v.order.label()));
                }
            }
            Btn::Sorry => {
                if self.state == Visitor::Waiting {
                    ctx.act(Action::Skip);
                }
            }
            Btn::Close => ctx.act(Action::CloseShop),
        }
    }
}

impl Screen for Shop {
    fn root(&self) -> Gd<Node2D> {
        self.root.clone()
    }

    fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        self.t += dt;
        self.hud.refresh(ctx.riso, ctx.state);
        self.btns.update(ctx, dt);
        self.floaters.retain_mut(|f| f.update(dt));
        for (a, t) in &mut self.fx {
            *t += dt;
            a.set_scale(0.6 + 0.5 * ease_out_back((*t / 0.5).min(1.0)));
            a.set_alpha(1.0 - ((*t - 0.8) / 0.5).clamp(0.0, 1.0));
        }
        let mut done = Vec::new();
        for (i, (_, t)) in self.fx.iter().enumerate() {
            if *t > 1.4 {
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            let (a, _) = self.fx.remove(i);
            a.free();
        }
        match self.state.clone() {
            Visitor::Entering(t) => {
                let t = t + dt;
                let k = ease_out_back((t / 0.6).min(1.0));
                self.critter.set_pos(v2(900.0 + (self.stand.x - 900.0) * k, self.stand.y));
                if t >= 0.6 {
                    self.state = Visitor::Waiting;
                    self.draw_bubble(ctx);
                } else {
                    self.state = Visitor::Entering(t);
                }
            }
            Visitor::Waiting => {
                // Idle bob (redraw a few times a second).
                let q = (self.t * 3.0).floor();
                if q != ((self.t - dt) * 3.0).floor() {
                    self.draw_critter(ctx);
                }
                let s = self.bubble.node.get_scale().x;
                if s < 1.0 {
                    self.bubble.set_scale((s + dt * 4.0).min(1.0));
                }
            }
            Visitor::Reacting(t, r) => {
                let t = t + dt;
                let hop = if r == Reaction::Love {
                    (t * 14.0).sin().abs() * 18.0 * (1.0 - t / 1.4).max(0.0)
                } else {
                    0.0
                };
                self.critter.set_pos(self.stand - v2(0.0, hop));
                self.state = if t > 1.4 { Visitor::Leaving(0.0) } else { Visitor::Reacting(t, r) };
            }
            Visitor::Leaving(t) => {
                let t = t + dt;
                let k = ease_in_out((t / 0.45).min(1.0));
                self.critter.set_pos(v2(self.stand.x - 700.0 * k, self.stand.y));
                if t >= 0.45 {
                    self.state = Visitor::None;
                    self.next_visitor(ctx);
                } else {
                    self.state = Visitor::Leaving(t);
                }
            }
            _ => {}
        }
        // Cells float home when not dragged.
        for (i, c) in self.cells.iter_mut().enumerate() {
            if self.drag.is_some_and(|(d, _, _)| d == i) {
                continue;
            }
            let p = c.art.pos();
            c.art.set_pos(p + (c.home - p) * (dt * 12.0).min(1.0));
            c.art.set_scale(1.0);
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, p: Ptr) {
        match p {
            Ptr::Down(q) => {
                if self.btns.down(ctx, q) {
                    return;
                }
                if self.state == Visitor::Waiting
                    && let Some(i) = self.cell_at(q)
                {
                    self.drag = Some((i, q, false));
                    self.cells[i].art.set_scale(1.15);
                    ctx.sfx(Sfx::Tap);
                }
            }
            Ptr::Move(q) => {
                if let Some((i, start, moved)) = self.drag.as_mut() {
                    if q.dist(*start) > 14.0 {
                        *moved = true;
                    }
                    if *moved {
                        let c = &mut self.cells[*i];
                        c.art.set_pos(q);
                        c.art.set_scale(1.15);
                    }
                }
            }
            Ptr::Up(q) => {
                if let Some(b) = self.btns.up(ctx, q) {
                    self.click(ctx, b);
                    return;
                }
                if let Some((i, _, moved)) = self.drag.take() {
                    if !moved || self.over_critter(q) {
                        self.serve(ctx, i);
                    } else {
                        ctx.sfx(Sfx::Squish);
                    }
                }
            }
        }
    }

    fn events(&mut self, ctx: &mut Ctx, events: &[Event]) {
        for e in events {
            match e {
                Event::Served(r) => self.on_served(ctx, r),
                Event::Skipped { species, preordered } => {
                    let name = customer::profile(*species).name;
                    if *preordered {
                        ctx.toast(format!("{name} pre-ordered for tomorrow!"));
                    } else {
                        ctx.toast(format!("{name} will come back another day."));
                    }
                    self.critter_expr = Expr::Hungry;
                    self.draw_critter(ctx);
                    self.bubble.set_visible(false);
                    self.state = Visitor::Leaving(0.0);
                }
                Event::Gift { from, unlock, coins } => {
                    let name = customer::profile(*from).name;
                    match unlock {
                        Some(u) => ctx.toast(format!("{name} gave you a keepsake: {}!", u.name())),
                        None => ctx.toast(format!("{name} left you a tip jar: +{coins}!")),
                    }
                    ctx.sfx(Sfx::Sparkle);
                }
                Event::LevelUp { level } => {
                    ctx.toast(format!("Bakery level {level}! New things in the catalog."));
                    ctx.sfx(Sfx::Sparkle);
                    ctx.kick(1.0);
                }
                _ => {}
            }
        }
    }

    fn auto(&mut self, ctx: &mut Ctx) -> Option<String> {
        match self.state {
            Visitor::Waiting => {
                let v = self.visitor(ctx)?;
                if !self.shown_order {
                    self.shown_order = true;
                    return Some(format!("order_{:?}", v.species).to_lowercase());
                }
                self.shown_order = false;
                if self.cells.is_empty() {
                    ctx.act(Action::Skip);
                    return Some("soldout".into());
                }
                let hist = ctx.state.friends.get(&v.species).map(|f| f.history.clone()).unwrap_or_default();
                let best = (0..self.cells.len()).max_by(|a, b| {
                    let ga = self
                        .shelf_index(ctx, self.cells[*a].key)
                        .map(|i| customer::satisfaction(&v.order, &ctx.state.shelf[i], &hist))
                        .unwrap_or(0.0);
                    let gb = self
                        .shelf_index(ctx, self.cells[*b].key)
                        .map(|i| customer::satisfaction(&v.order, &ctx.state.shelf[i], &hist))
                        .unwrap_or(0.0);
                    ga.total_cmp(&gb)
                })?;
                self.serve(ctx, best);
                Some(format!("serve_{:?}", v.species).to_lowercase())
            }
            Visitor::Summary => {
                ctx.act(Action::CloseShop);
                Some("receipt".into())
            }
            _ => None,
        }
    }
}
