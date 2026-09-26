//! The night page — "The Daily Crumb": today's summary, loaf of the day, the catalog, sleep.
//! Also hosts the Loaf Zine collection view.

use super::{Ctx, Nav, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list};
use crate::sfx::Sfx;
use crate::ui::{Button, Hud, Panel};
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::art::bread::loaf_top;
use proof_core::art::icons::{Icon, icon};
use proof_core::art::{props, twinkle};
use proof_core::content::{Pattern, Recipe};
use proof_core::draw::DrawList;
use proof_core::economy::{self, Unlock};
use proof_core::geom::{V2, Xf, rect, rounded_rect, soft_star, v2};
use proof_core::ink::Ink;
use proof_core::state::{Action, Event};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Btn {
    Buy(Unlock),
    Zine,
    Sleep,
    Back,
}

pub struct Night {
    root: Gd<Node2D>,
    rn: Gd<Node>,
    hud: Hud,
    zine: bool,
    page: Art,
    labels: Vec<Gd<Label>>,
    rows: Vec<Art>,
    btns: Panel<Btn>,
    shop: Panel<Btn>,
}

pub fn unlock_icon(u: Unlock) -> Icon {
    match u {
        Unlock::Recipe(r) => Icon::Recipe(r),
        Unlock::Flour(f) => Icon::Flour(f),
        Unlock::Stencil(s) => Icon::Stencil(s),
        Unlock::Topping(t) => Icon::Topping(t),
        Unlock::Pattern(p) => Icon::Pattern(p),
        Unlock::Treat(t) => Icon::Treat(t),
        Unlock::Fridge(_) => Icon::Fridge,
        Unlock::Jar(_) => Icon::Jar,
        Unlock::Lid => Icon::Oven,
    }
}

impl Night {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>, zine: bool) -> Night {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let h = ctx.lay.h;
        let page = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let hud = Hud::new(ctx, &mut rn, if zine { "Loaf Zine" } else { "Night edition" });
        let mut n = Night {
            root,
            rn,
            hud,
            zine,
            page,
            labels: Vec::new(),
            rows: Vec::new(),
            btns: Panel::default(),
            shop: Panel::default(),
        };
        let mut rn2 = n.rn.clone();
        let back = Button::round(ctx, &mut rn2, v2(80.0, h - 62.0), 38.0, Ink::Blue, Icon::Back);
        n.btns.add(Btn::Back, back);
        if zine {
            n.build_zine(ctx);
        } else {
            let z =
                Button::pill(ctx, &mut rn2, rect(140.0, h - 104.0, 240.0, 82.0), "Loaf Zine", Ink::Yellow);
            n.btns.add(Btn::Zine, z);
            let s = Button::pill(ctx, &mut rn2, rect(400.0, h - 104.0, 290.0, 82.0), "Sleep", Ink::Pink);
            n.btns.add(Btn::Sleep, s);
            n.build_page(ctx);
            n.build_catalog(ctx);
            ctx.note("night", "This is your nightly zine! Spend coins on new recipes and tools, then sleep. Tomorrow's doughs are already resting.");
        }
        n
    }

    fn label(&mut self, ctx: &mut Ctx, spec: TextSpec) {
        let mut rn = self.rn.clone();
        self.labels.push(ctx.riso.text(&mut rn, &spec));
    }

    fn build_page(&mut self, ctx: &mut Ctx) {
        let st = ctx.state;
        let best = st.today.best.clone();
        let (lvl, xp) = (st.level(), st.xp);
        let frac = economy::xp_to_next(xp).map(|(a, b)| a as f32 / b as f32).unwrap_or(1.0);
        let d = list(|d| {
            // Masthead rules and stars.
            d.line(Ink::Key, 5.0, &[v2(30.0, 186.0), v2(690.0, 186.0)]);
            d.line(Ink::Key, 2.0, &[v2(30.0, 194.0), v2(690.0, 194.0)]);
            d.line(Ink::Key, 2.0, &[v2(30.0, 470.0), v2(690.0, 470.0)]);
            d.line(Ink::Key, 2.0, &[v2(360.0, 210.0), v2(360.0, 455.0)]);
            for (x, s) in [(56.0, 14.0), (664.0, 14.0)] {
                twinkle(d, v2(x, 140.0), s, Ink::Pink);
            }
            // Loaf of the day.
            if let Some(l) = &best {
                d.with(Xf::at(v2(190.0, 340.0)), |d| loaf_top(d, &l.view(76.0)));
            } else {
                d.ht(Ink::Blue, 0.25, &proof_core::geom::circle(v2(190.0, 330.0), 80.0));
            }
            // Stats icons.
            props::coin(d, v2(410.0, 250.0), 20.0);
            props::heart_icon(d, v2(410.0, 305.0), 38.0);
            let badge = soft_star(v2(410.0, 362.0), 24.0, 14.0, 8, 0.0);
            d.fill(Ink::Yellow, 1.0, &badge);
            d.outline(Ink::Key, 3.0, &badge);
            let track = rounded_rect(rect(390.0, 405.0, 290.0, 20.0), 10.0);
            d.backing(&track);
            d.outline(Ink::Key, 3.0, &track);
            if frac > 0.02 {
                d.fill(Ink::Pink, 0.9, &rounded_rect(rect(392.0, 407.0, 286.0 * frac, 16.0), 8.0));
            }
        });
        self.page.set(ctx.riso, &d);
        self.label(ctx, TextSpec::new("The Daily Crumb", rect(30.0, 100.0, 660.0, 80.0), 56.0).bold());
        self.label(
            ctx,
            TextSpec::new(format!("Day {} · evening edition", st.day), rect(30.0, 196.0, 660.0, 30.0), 20.0)
                .plain(),
        );
        self.label(ctx, TextSpec::new("Loaf of the Day", rect(40.0, 206.0, 300.0, 34.0), 24.0).bold());
        let cap = best
            .as_ref()
            .map(|l| format!("{} {}", l.title(), "★".repeat(l.stars as usize)))
            .unwrap_or_else(|| "Nothing baked today".into());
        self.label(ctx, TextSpec::new(cap, rect(40.0, 420.0, 300.0, 44.0), 20.0).wrap());
        let t = &st.today;
        self.label(
            ctx,
            TextSpec::new(
                format!("{} coins earned", t.coins + t.tips),
                rect(440.0, 232.0, 250.0, 36.0),
                23.0,
            )
            .left(),
        );
        self.label(
            ctx,
            TextSpec::new(
                format!("{} hearts · {} served", t.hearts, t.served),
                rect(440.0, 287.0, 250.0, 36.0),
                23.0,
            )
            .left(),
        );
        self.label(
            ctx,
            TextSpec::new(format!("Bakery level {lvl}"), rect(440.0, 344.0, 250.0, 36.0), 23.0).bold().left(),
        );
    }

    fn build_catalog(&mut self, ctx: &mut Ctx) {
        self.shop.clear();
        for a in self.rows.drain(..) {
            a.free();
        }
        let mut items = ctx.state.shop_items();
        items.sort_by_key(|c| (c.cost > ctx.state.coins, c.cost));
        let top = 490.0;
        self.label(ctx, TextSpec::new("The Catalog", rect(30.0, top, 660.0, 44.0), 32.0).bold());
        let mut rn = self.rn.clone();
        let max = ((ctx.lay.h - 150.0 - (top + 60.0)) / 96.0).floor().max(1.0) as usize;
        if items.is_empty() {
            self.label(
                ctx,
                TextSpec::new(
                    "You own everything for now! Level up for more.",
                    rect(60.0, top + 70.0, 600.0, 80.0),
                    24.0,
                )
                .wrap(),
            );
        }
        for (i, it) in items.iter().take(max).enumerate() {
            let y = top + 100.0 + i as f32 * 96.0;
            let u = it.unlock;
            let row = list(|d| {
                let card = rounded_rect(rect(30.0, y - 42.0, 660.0, 84.0), 16.0);
                d.backing(&card);
                d.fill(Ink::Yellow, 0.12, &card);
                d.outline(Ink::Key, 3.0, &card);
                icon(d, unlock_icon(u), v2(84.0, y), 30.0);
            });
            self.rows.push(ctx.riso.art(&mut rn, V2::ZERO, &row));
            self.label(ctx, TextSpec::new(u.name(), rect(130.0, y - 38.0, 380.0, 40.0), 24.0).bold().left());
            self.label(ctx, TextSpec::new(u.blurb(), rect(130.0, y - 2.0, 380.0, 34.0), 18.0).left().plain());
            let mut b = Button::pill(
                ctx,
                &mut rn,
                rect(528.0, y - 30.0, 150.0, 60.0),
                &format!("{} ◉", it.cost),
                Ink::Pink,
            );
            if it.cost > ctx.state.coins {
                b.set_enabled(ctx, false);
            }
            self.shop.add(Btn::Buy(u), b);
        }
    }

    fn build_zine(&mut self, ctx: &mut Ctx) {
        let prints: Vec<_> = ctx.state.zine.iter().rev().take(12).cloned().collect();
        let found = ctx.state.discovered.iter().filter(|k| !k.ends_with("None")).count();
        let total = Recipe::ALL.len() * Pattern::ALL.len();
        let d = list(|d| {
            d.line(Ink::Key, 5.0, &[v2(30.0, 186.0), v2(690.0, 186.0)]);
            for (i, l) in prints.iter().enumerate() {
                let x = 130.0 + (i % 3) as f32 * 230.0;
                let y = 345.0 + (i / 3) as f32 * 215.0;
                let frame = rounded_rect(rect(x - 100.0, y - 95.0, 200.0, 200.0), 14.0);
                d.backing(&frame);
                d.fill(Ink::Yellow, 0.1, &frame);
                d.outline(Ink::Key, 3.0, &frame);
                d.with(Xf::at(v2(x, y - 12.0)), |d| loaf_top(d, &l.view(64.0)));
            }
        });
        self.page.set(ctx.riso, &d);
        self.label(ctx, TextSpec::new("Loaf Zine", rect(30.0, 100.0, 660.0, 80.0), 56.0).bold());
        self.label(
            ctx,
            TextSpec::new(
                format!("{found} / {total} prints discovered"),
                rect(30.0, 196.0, 660.0, 34.0),
                22.0,
            )
            .plain(),
        );
        for (i, l) in prints.iter().enumerate() {
            let x = 130.0 + (i % 3) as f32 * 230.0;
            let y = 345.0 + (i / 3) as f32 * 215.0;
            let cap = format!("Day {} {}", l.day, "★".repeat(l.stars as usize));
            self.label(ctx, TextSpec::new(cap, rect(x - 100.0, y + 64.0, 200.0, 30.0), 18.0).plain());
        }
        if prints.is_empty() {
            self.label(
                ctx,
                TextSpec::new("Bake something to start your zine!", rect(60.0, 400.0, 600.0, 60.0), 26.0),
            );
        }
    }

    fn click(&mut self, ctx: &mut Ctx, b: Btn) {
        match b {
            Btn::Buy(u) => ctx.act(Action::Buy(u)),
            Btn::Zine => ctx.nav(Nav::Zine),
            Btn::Sleep => {
                ctx.sfx(Sfx::Whoosh);
                ctx.act(Action::Sleep);
            }
            Btn::Back => ctx.nav(if self.zine { Nav::Night } else { Nav::Phase }),
        }
    }
}

impl Screen for Night {
    fn root(&self) -> Gd<Node2D> {
        self.root.clone()
    }

    fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        self.hud.refresh(ctx.riso, ctx.state);
        self.btns.update(ctx, dt);
        self.shop.update(ctx, dt);
    }

    fn pointer(&mut self, ctx: &mut Ctx, p: Ptr) {
        match p {
            Ptr::Down(q) => {
                let _ = self.btns.down(ctx, q) || self.shop.down(ctx, q);
            }
            Ptr::Up(q) => {
                if let Some(b) = self.btns.up(ctx, q).or_else(|| self.shop.up(ctx, q)) {
                    self.click(ctx, b);
                }
            }
            Ptr::Move(_) => {}
        }
    }

    fn events(&mut self, ctx: &mut Ctx, events: &[Event]) {
        for e in events {
            match e {
                Event::Bought(u) => {
                    ctx.toast(format!("Unlocked: {}!", u.name()));
                    ctx.sfx(Sfx::Coin);
                    ctx.sfx(Sfx::Sparkle);
                    ctx.kick(0.5);
                    // Rebuild the catalog (labels included).
                    for mut l in self.labels.drain(..) {
                        l.queue_free();
                    }
                    self.build_page(ctx);
                    self.build_catalog(ctx);
                }
                Event::NewStarter { jar } => {
                    if let Some(s) = ctx.state.starters.get(*jar) {
                        ctx.toast(format!("{} joined the shelf! Feed them tonight.", s.name));
                    }
                }
                _ => {}
            }
        }
    }

    fn auto(&mut self, ctx: &mut Ctx) -> Option<String> {
        if self.zine {
            ctx.nav(Nav::Night);
            return Some("zine".into());
        }
        if !ctx.state.saw_tip("tour_zine") && !ctx.state.zine.is_empty() {
            ctx.act(Action::Tip("tour_zine".into()));
            ctx.nav(Nav::Zine);
            return Some("night_page".into());
        }
        let items = ctx.state.shop_items();
        if let Some(it) = items.iter().filter(|c| c.cost + 5 <= ctx.state.coins).min_by_key(|c| c.cost) {
            ctx.act(Action::Buy(it.unlock));
            return Some("bought".into());
        }
        ctx.act(Action::Sleep);
        Some("sleep".into())
    }
}
