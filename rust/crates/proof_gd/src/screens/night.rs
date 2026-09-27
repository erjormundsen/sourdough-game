//! The night page — "The Daily Crumb": today's summary, loaf of the day, the catalog, sleep.
//! Also hosts the Loaf Zine collection view.
//!
//! Laid out like a printed newspaper page: nameplate, dateline between rules, two columns
//! (loaf of the day | the day's numbers), then the catalog as classified listings.

use super::{Ctx, Nav, Ptr, Screen, new_root};
use crate::riso::{Art, TextSpec, list};
use crate::sfx::Sfx;
use crate::ui::{self, Button, Hud, Panel};
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::art::bread::loaf_top;
use proof_core::art::icons::{Icon, icon};
use proof_core::art::{props, twinkle};
use proof_core::content::{Pattern, Recipe};
use proof_core::draw::{DrawList, Paint};
use proof_core::economy::{self, Unlock};
use proof_core::geom::{V2, Xf, rect, rect_poly, scallop, v2};
use proof_core::ink::Ink;
use proof_core::state::{Action, Event};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Btn {
    Buy(Unlock),
    Zine,
    Sleep,
    Back,
    Pref(u8),
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
    top: f32,
    bar: f32,
    /// Autopilot: this page has been photographed.
    shot: bool,
}

fn pref_text(st: proof_core::state::Settings, i: u8) -> String {
    let on = |b: bool| if b { "on" } else { "off" };
    match i {
        0 => format!("Sound {}", on(st.sound)),
        1 => format!("Buzz {}", on(st.haptics)),
        _ => format!("Calm {}", on(st.calm)),
    }
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

/// A newspaper nameplate: big title between a double rule and a dateline strip.
fn nameplate(d: &mut DrawList, y: f32) {
    for (x, s, ink) in [(70.0, 15.0, Ink::Pink), (650.0, 15.0, Ink::Yellow)] {
        twinkle(d, v2(x, y), s, ink);
    }
    let r1 = y + 40.0;
    d.line(Ink::Key, 5.0, &[v2(28.0, r1), v2(692.0, r1)]);
    d.line(Ink::Key, 1.6, &[v2(28.0, r1 + 7.0), v2(692.0, r1 + 7.0)]);
    d.line(Ink::Key, 1.6, &[v2(28.0, r1 + 38.0), v2(692.0, r1 + 38.0)]);
}

impl Night {
    pub fn new(ctx: &mut Ctx, host: &mut Gd<Node2D>, zine: bool) -> Night {
        let root = new_root(host);
        let mut rn: Gd<Node> = root.clone().upcast();
        let top = ctx.lay.top;
        let bar = ui::bar_y(ctx);
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
            top,
            bar,
            shot: false,
        };
        let mut rn2 = n.rn.clone();
        if zine {
            let (_, pri) = ui::bar_rects(bar, false);
            let back = Button::pill(ctx, &mut rn2, pri, "Back to tonight", Ink::Blue);
            n.btns.add(Btn::Back, back);
            n.build_zine(ctx);
        } else {
            let back = Button::round(ctx, &mut rn2, v2(ui::MARGIN + 38.0, bar), 38.0, Ink::Blue, Icon::Back);
            n.btns.add(Btn::Back, back);
            let x0 = ui::MARGIN + 76.0 + 16.0;
            let z = Button::pill(
                ctx,
                &mut rn2,
                rect(x0, bar - ui::PILL_H * 0.5, 224.0, ui::PILL_H),
                "Loaf Zine",
                Ink::Yellow,
            );
            n.btns.add(Btn::Zine, z);
            let x1 = x0 + 224.0 + 16.0;
            let s = Button::pill(
                ctx,
                &mut rn2,
                rect(x1, bar - ui::PILL_H * 0.5, 720.0 - ui::MARGIN - x1, ui::PILL_H),
                "Sleep",
                Ink::Pink,
            );
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

    fn y(&self, v: f32) -> f32 {
        self.top + v
    }

    fn build_page(&mut self, ctx: &mut Ctx) {
        let st = ctx.state;
        let best = st.today.best.clone();
        let (lvl, xp) = (st.level(), st.xp);
        let frac = economy::xp_to_next(xp).map(|(a, b)| a as f32 / b as f32).unwrap_or(1.0);
        let (y_name, y_cols) = (self.y(150.0), self.y(260.0));
        let col_h = 230.0;
        let d = list(|d| {
            nameplate(d, y_name);
            // Column rules.
            d.line(Ink::Key, 1.6, &[v2(348.0, y_cols + 6.0), v2(348.0, y_cols + col_h - 6.0)]);
            d.line(Ink::Key, 3.0, &[v2(28.0, y_cols + col_h + 8.0), v2(692.0, y_cols + col_h + 8.0)]);
            // Loaf of the day on a paper doily.
            let lc = v2(188.0, y_cols + 118.0);
            let lace = scallop(lc + v2(0.0, 6.0), 96.0, 16, 0.06);
            d.backing(&lace);
            d.fill(Ink::Yellow, 0.1, &lace);
            d.stroke_p(Paint::solid(Ink::Key, 0.45), 1.4, &lace, true);
            if let Some(l) = &best {
                d.with(Xf::at(lc), |d| loaf_top(d, &l.view(70.0)));
            } else {
                d.ht(Ink::Blue, 0.22, &proof_core::geom::circle(lc, 64.0));
            }
            // The day's numbers.
            let x = 392.0;
            props::coin(d, v2(x, y_cols + 34.0), 17.0);
            props::heart_icon(d, v2(x, y_cols + 88.0), 34.0);
            props::rosette(d, v2(x, y_cols + 146.0), 22.0);
            d.with(Xf::at(v2(x - 18.0, y_cols + 186.0)), |d| props::meter(d, 300.0, 18.0, frac, Ink::Pink));
        });
        self.page.set(ctx.riso, &d);
        self.label(
            ctx,
            TextSpec::new("The Daily Crumb", rect(60.0, y_name - 36.0, 600.0, 72.0), 54.0).bold(),
        );
        self.label(
            ctx,
            TextSpec::new(
                format!("Day {} · Evening edition · one crumb", st.day),
                rect(28.0, y_name + 48.0, 664.0, 30.0),
                19.0,
            )
            .plain(),
        );
        self.label(ctx, TextSpec::new("Loaf of the Day", rect(40.0, y_cols - 4.0, 296.0, 34.0), 23.0).bold());
        let cap = best
            .as_ref()
            .map(|l| format!("{} {}", l.title(), "★".repeat(l.stars as usize)))
            .unwrap_or_else(|| "Nothing baked today".into());
        self.label(ctx, TextSpec::new(cap, rect(40.0, y_cols + col_h - 26.0, 296.0, 30.0), 18.0).plain());
        let t = &st.today;
        let x = 420.0;
        self.label(
            ctx,
            TextSpec::new(
                format!("{} coins earned", t.coins + t.tips),
                rect(x, y_cols + 16.0, 272.0, 36.0),
                23.0,
            )
            .left(),
        );
        self.label(
            ctx,
            TextSpec::new(
                format!("{} hearts · {} served", t.hearts, t.served),
                rect(x, y_cols + 70.0, 272.0, 36.0),
                23.0,
            )
            .left(),
        );
        self.label(
            ctx,
            TextSpec::new(format!("Bakery level {lvl}"), rect(x, y_cols + 128.0, 272.0, 36.0), 23.0)
                .bold()
                .left(),
        );
    }

    fn build_catalog(&mut self, ctx: &mut Ctx) {
        self.shop.clear();
        for a in self.rows.drain(..) {
            a.free();
        }
        let mut items = ctx.state.shop_items();
        items.sort_by_key(|c| (c.cost > ctx.state.coins, c.cost));
        let top = self.y(514.0);
        self.label(ctx, TextSpec::new("The Catalog", rect(28.0, top, 664.0, 44.0), 30.0).bold());
        let mut rn = self.rn.clone();
        let pitch = 100.0;
        let first = top + 58.0;
        let bottom = self.bar - ui::PILL_H * 0.5 - 18.0;
        let max = ((bottom - first) / pitch).floor().max(1.0) as usize;
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
            let y = first + i as f32 * pitch;
            let u = it.unlock;
            let afford = it.cost <= ctx.state.coins;
            let row = list(|d| {
                d.with(Xf::at(v2(28.0, y)), |d| props::listing(d, 664.0, 88.0, afford));
                icon(d, unlock_icon(u), v2(28.0 + 88.0 * 0.56, y + 44.0), 27.0);
                props::coin(d, v2(534.0, y + 44.0), 13.0);
            });
            self.rows.push(ctx.riso.art(&mut rn, V2::ZERO, &row));
            self.label(ctx, TextSpec::new(u.name(), rect(128.0, y + 8.0, 390.0, 40.0), 23.0).bold().left());
            self.label(
                ctx,
                TextSpec::new(u.blurb(), rect(128.0, y + 46.0, 390.0, 32.0), 17.0).left().plain(),
            );
            let mut b = Button::pill(
                ctx,
                &mut rn,
                rect(556.0, y + 16.0, 122.0, 56.0),
                &format!("{}", it.cost),
                Ink::Pink,
            );
            if !afford {
                b.set_enabled(ctx, false);
            }
            self.shop.add(Btn::Buy(u), b);
        }
        // Fill the rest of the page with next level's goodies, printed as teasers.
        let lvl = ctx.state.level();
        let shown = items.len().min(max);
        let teasers: Vec<_> = economy::catalog(lvl + 1)
            .filter(|c| c.level > lvl && !ctx.state.has(c.unlock))
            .take(max.saturating_sub(shown))
            .copied()
            .collect();
        for (k, it) in teasers.iter().enumerate() {
            let y = first + (shown + k) as f32 * pitch;
            let u = it.unlock;
            let row = list(|d| {
                d.with(Xf::at(v2(28.0, y)), |d| props::listing(d, 664.0, 88.0, false));
                icon(d, unlock_icon(u), v2(28.0 + 88.0 * 0.56, y + 44.0), 27.0);
                // Faded: a wash of paper over the icon, and a tape tag instead of a price.
                d.knock_p(
                    0.55,
                    proof_core::draw::Screen::Solid,
                    proof_core::draw::PLATES_ALL,
                    &proof_core::geom::circle(v2(28.0 + 88.0 * 0.56, y + 44.0), 32.0),
                );
                d.with(Xf::at(v2(617.0, y + 44.0)).rotated(-0.05), |d| {
                    props::tape(d, 124.0, 34.0, Ink::Blue)
                });
            });
            self.rows.push(ctx.riso.art(&mut rn, V2::ZERO, &row));
            self.label(
                ctx,
                TextSpec::new(u.name(), rect(128.0, y + 8.0, 390.0, 40.0), 23.0).bold().left().plain(),
            );
            self.label(
                ctx,
                TextSpec::new("Coming in the next issue…", rect(128.0, y + 46.0, 390.0, 32.0), 17.0)
                    .left()
                    .plain(),
            );
            self.label(
                ctx,
                TextSpec::new(format!("Level {}", it.level), rect(557.0, y + 27.0, 120.0, 34.0), 19.0)
                    .bold()
                    .plain(),
            );
        }
    }

    fn build_zine(&mut self, ctx: &mut Ctx) {
        let prints: Vec<_> = ctx.state.zine.iter().rev().take(12).cloned().collect();
        let found = ctx.state.discovered.iter().filter(|k| !k.ends_with("None")).count();
        let total = Recipe::ALL.len() * Pattern::ALL.len();
        let y_name = self.y(150.0);
        let prefs_y = self.bar - ui::PILL_H * 0.5 - 70.0;
        let grid_top = y_name + 110.0;
        let rows_fit = ((prefs_y - 96.0 - grid_top) / 236.0).floor().clamp(1.0, 4.0) as usize;
        let slots = (rows_fit * 3).min(12);
        let shown = prints.len().min(slots);
        let pitch_y = ((prefs_y - 96.0 - grid_top) / rows_fit as f32).min(250.0);
        let spot = |i: usize| {
            v2(130.0 + (i % 3) as f32 * 230.0, grid_top + pitch_y * 0.5 + (i / 3) as f32 * pitch_y)
        };
        let d = list(|d| {
            nameplate(d, y_name);
            d.with(Xf::at(v2(410.0, y_name + 52.0)), |d| {
                props::meter(d, 250.0, 16.0, found as f32 / total as f32, Ink::Pink)
            });
            for i in 0..slots {
                let p = spot(i);
                let rot = ((i as f32) * 2.3).sin() * 0.05;
                if let Some(l) = prints.get(i) {
                    d.with(Xf::at(p).rotated(rot), |d| {
                        props::photo(d, 196.0, 214.0, Ink::Yellow);
                        d.with(Xf::at(v2(0.0, -16.0)), |d| loaf_top(d, &l.view(62.0)));
                    });
                } else {
                    // An empty album slot with photo corners, waiting for a print.
                    let (w, h) = (196.0, 214.0);
                    let r = proof_core::geom::rect(p.x - w * 0.5, p.y - h * 0.5, w, h);
                    let pts = rect_poly(r);
                    for k in 0..4 {
                        props::dashed(d, pts[k], pts[(k + 1) % 4], 9.0, 7.0, 2.0, 0.35);
                    }
                    for (k, c) in pts.iter().enumerate() {
                        let sx = if k == 0 || k == 3 { 1.0 } else { -1.0 };
                        let sy = if k < 2 { 1.0 } else { -1.0 };
                        let tri = vec![*c, *c + v2(sx * 26.0, 0.0), *c + v2(0.0, sy * 26.0)];
                        d.fill(Ink::Key, 0.25, &tri);
                    }
                    let ghost = proof_core::geom::ellipse(p + v2(0.0, -10.0), 50.0, 40.0, 0.0);
                    d.stroke_p(Paint::solid(Ink::Key, 0.2), 2.0, &ghost, true);
                }
            }
            // Settings strip.
            let strip = rect_poly(rect(28.0, prefs_y - 44.0, 664.0, 2.0));
            d.fill(Ink::Key, 0.6, &strip);
        });
        self.page.set(ctx.riso, &d);
        self.label(ctx, TextSpec::new("Loaf Zine", rect(60.0, y_name - 36.0, 600.0, 72.0), 54.0).bold());
        self.label(
            ctx,
            TextSpec::new(
                format!("{found} / {total} prints discovered"),
                rect(40.0, y_name + 48.0, 360.0, 30.0),
                19.0,
            )
            .left()
            .plain(),
        );
        for (i, l) in prints.iter().take(shown).enumerate() {
            let p = spot(i);
            let cap = format!("Day {} {}", l.day, "★".repeat(l.stars as usize));
            self.label(ctx, TextSpec::new(cap, rect(p.x - 98.0, p.y + 72.0, 196.0, 30.0), 18.0).plain());
        }
        if prints.is_empty() {
            self.label(
                ctx,
                TextSpec::new(
                    "Bake something to start your zine!",
                    rect(60.0, grid_top + 120.0, 600.0, 60.0),
                    26.0,
                ),
            );
        }
        self.build_prefs(ctx, prefs_y);
    }

    fn build_prefs(&mut self, ctx: &mut Ctx, y: f32) {
        let mut rn = self.rn.clone();
        let st = ctx.state.settings;
        self.label(ctx, TextSpec::new("Settings", rect(28.0, y - 84.0, 200.0, 34.0), 20.0).bold().left());
        let w = (664.0 - 2.0 * 14.0) / 3.0;
        for i in 0..3u8 {
            let ic = [Icon::Sound, Icon::Buzz, Icon::Calm][i as usize];
            let b = Button::pill_icon(
                ctx,
                &mut rn,
                rect(28.0 + i as f32 * (w + 14.0), y - 32.0, w, 64.0),
                &pref_text(st, i),
                Ink::Blue,
                ic,
            );
            self.btns.add(Btn::Pref(i), b);
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
            Btn::Pref(i) => {
                let mut st = ctx.state.settings;
                match i {
                    0 => st.sound = !st.sound,
                    1 => st.haptics = !st.haptics,
                    _ => st.calm = !st.calm,
                }
                ctx.act(Action::Settings(st));
                let text = pref_text(st, i);
                if let Some(b) = self.btns.get(Btn::Pref(i)) {
                    b.set_text(&text);
                }
            }
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
        // Photograph each page first, then move on (labels match what's on screen).
        if self.zine {
            if !self.shot {
                self.shot = true;
                return Some("zine".into());
            }
            ctx.nav(Nav::Night);
            return None;
        }
        if !ctx.state.saw_tip("tour_zine") && !ctx.state.zine.is_empty() {
            if !self.shot {
                self.shot = true;
                return Some("night_page".into());
            }
            ctx.act(Action::Tip("tour_zine".into()));
            ctx.nav(Nav::Zine);
            return None;
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
