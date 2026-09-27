//! Printed widgets: buttons, the masthead HUD with its headline ribbon, floating text.
//! All hit-testing is in Rust.
//!
//! Sizes follow one grid: primary pills are 84 tall, catalog/price pills 56; round tool
//! buttons r = 32, navigation buttons r = 38. Screens keep 24–32 unit side margins.

use crate::riso::{Art, Riso, TextSpec, list};
use crate::screens::Ctx;
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::Spring;
use proof_core::art::icons::{Icon, icon};
use proof_core::art::props::{self, Look};
use proof_core::content::Flour;
use proof_core::draw::DrawList;
use proof_core::economy;
use proof_core::geom::{Rect, V2, Xf, rect, v2};
use proof_core::ink::Ink;
use proof_core::state::{GameState, Phase};

/// Primary action pill height.
pub const PILL_H: f32 = 84.0;
/// Side margin for bars and cards.
pub const MARGIN: f32 = 28.0;
/// Masthead height (below the safe-area inset).
pub const MAST_H: f32 = 96.0;
/// Centre line of the headline ribbon, below the masthead.
pub const RIBBON_Y: f32 = 132.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Pill(Ink),
    /// A pill with an icon badge on its left end.
    PillIcon(Ink, Icon),
    Round(Ink, Icon),
    /// A little wooden sign standing on a counter; the rect's bottom edge is the counter.
    Sign,
    /// A flour sack standing on a surface (the rect's bottom edge, above its name).
    Sack(Flour),
}

pub struct Button {
    pub rect: Rect,
    art: Art,
    label: Option<Gd<Label>>,
    label_y: f32,
    style: Style,
    pub enabled: bool,
    pub selected: bool,
    pub visible: bool,
    pressed: bool,
    drawn: Option<(bool, bool, bool)>,
    bounce: Spring,
}

/// Height of the name strip under a sack button.
const SACK_LABEL: f32 = 30.0;

impl Button {
    fn new(
        riso: &mut Riso,
        parent: &mut Gd<Node>,
        rect: Rect,
        style: Style,
        text: &str,
        size: f32,
    ) -> Button {
        let c = rect.center();
        let art = riso.art(parent, c, &DrawList::new());
        let (w, h) = (rect.w, rect.h);
        let (lrect, label_y) = match style {
            Style::Sign => (proof_core::geom::rect(-w * 0.5, -h * 0.5 - 2.0, w, h - 16.0), -h * 0.5 - 2.0),
            Style::Sack(_) => (
                proof_core::geom::rect(-w * 0.5 - 20.0, h * 0.5 - SACK_LABEL, w + 40.0, SACK_LABEL),
                h * 0.5 - SACK_LABEL,
            ),
            Style::PillIcon(..) => (
                proof_core::geom::rect(-w * 0.5 + h * 0.8, -h * 0.5 - 3.0, w - h * 0.8 - 10.0, h),
                -h * 0.5 - 3.0,
            ),
            _ => (proof_core::geom::rect(-w * 0.5, -h * 0.5 - 3.0, w, h), -h * 0.5 - 3.0),
        };
        let label = if text.is_empty() {
            None
        } else {
            let mut an = art.as_node();
            let t = TextSpec::new(text, lrect, size).bold().plain();
            Some(riso.text(&mut an, &t))
        };
        let mut b = Button {
            rect,
            art,
            label,
            label_y,
            style,
            enabled: true,
            selected: false,
            visible: true,
            pressed: false,
            drawn: None,
            bounce: Spring::new(1.0),
        };
        b.redraw(riso);
        b
    }

    /// A pill with an icon badge at its left end (settings toggles).
    pub fn pill_icon(
        ctx: &mut Ctx,
        parent: &mut Gd<Node>,
        rect: Rect,
        text: &str,
        ink: Ink,
        ic: Icon,
    ) -> Button {
        let size = (rect.h * 0.36).clamp(18.0, 28.0);
        Button::new(ctx.riso, parent, rect, Style::PillIcon(ink, ic), text, size)
    }

    pub fn pill(ctx: &mut Ctx, parent: &mut Gd<Node>, rect: Rect, text: &str, ink: Ink) -> Button {
        let size = (rect.h * 0.4).clamp(20.0, 33.0);
        Button::new(ctx.riso, parent, rect, Style::Pill(ink), text, size)
    }

    pub fn round(ctx: &mut Ctx, parent: &mut Gd<Node>, c: V2, r: f32, ink: Ink, ic: Icon) -> Button {
        Button::new(
            ctx.riso,
            parent,
            rect(c.x - r, c.y - r, r * 2.0, r * 2.0),
            Style::Round(ink, ic),
            "",
            0.0,
        )
    }

    /// A counter sign whose feet stand on `base` (bottom centre); `w`×`h` is the board.
    pub fn sign(ctx: &mut Ctx, parent: &mut Gd<Node>, base: V2, w: f32, h: f32, text: &str) -> Button {
        let r = rect(base.x - w * 0.5, base.y - h - 16.0, w, h + 16.0);
        Button::new(ctx.riso, parent, r, Style::Sign, text, (h * 0.34).clamp(18.0, 26.0))
    }

    /// A flour sack standing on `base` (bottom centre) with its name printed below.
    pub fn sack(ctx: &mut Ctx, parent: &mut Gd<Node>, base: V2, flour: Flour) -> Button {
        let (w, h) = (140.0, 166.0 + SACK_LABEL);
        let r = rect(base.x - w * 0.5, base.y - 166.0, w, h);
        Button::new(ctx.riso, parent, r, Style::Sack(flour), flour.name(), 21.0)
    }

    fn redraw(&mut self, riso: &mut Riso) {
        let state = (self.pressed, self.selected, self.enabled);
        if self.drawn == Some(state) {
            return;
        }
        self.drawn = Some(state);
        let (w, h) = (self.rect.w, self.rect.h);
        let look = Look { pressed: self.pressed, selected: self.selected, disabled: !self.enabled };
        let mut lift = 0.0;
        let d = list(|d| match self.style {
            Style::PillIcon(ink, ic) => {
                props::button_ex(d, w, h, ink, look);
                lift = props::button_lift(h, look.pressed);
                let c = v2(-w * 0.5 + h * 0.5 + 2.0, lift);
                let badge = proof_core::geom::circle(c, h * 0.34);
                d.backing(&badge);
                d.fill(Ink::Yellow, 0.12, &badge);
                d.outline(Ink::Key, 2.6, &badge);
                icon(d, ic, c + v2(0.0, -1.0), h * 0.22);
            }
            Style::Pill(ink) => {
                props::button_ex(d, w, h, ink, look);
                lift = props::button_lift(h, look.pressed);
            }
            Style::Round(ink, ic) => {
                let r = w * 0.5;
                props::round_button_ex(d, r, ink, look);
                let y = props::round_lift(r, look.pressed);
                icon(d, ic, v2(0.0, y - 1.0), r * 0.6);
            }
            Style::Sign => {
                d.with(Xf::at(v2(0.0, h * 0.5)), |d| props::sign(d, w, h - 16.0, look));
                lift = if look.pressed { 3.0 } else { 0.0 };
            }
            Style::Sack(f) => {
                let y = if look.pressed { 3.0 } else { 0.0 };
                if self.selected {
                    d.ht(
                        Ink::Yellow,
                        0.5,
                        &proof_core::geom::ellipse(v2(0.0, h * 0.5 - SACK_LABEL - 60.0), 86.0, 96.0, 0.0),
                    );
                }
                d.with(
                    Xf::at(v2(0.0, h * 0.5 - SACK_LABEL + y)).scaled(if look.pressed { 0.97 } else { 1.0 }),
                    |d| props::flour_bag(d, f),
                );
                // Name strip: a bit of masking tape on the counter edge.
                d.with(Xf::at(v2(0.0, h * 0.5 - SACK_LABEL * 0.5)).rotated(-0.02), |d| {
                    props::tape(d, 128.0, SACK_LABEL - 4.0, Ink::Yellow)
                });
            }
        });
        self.art.set(riso, &d);
        if let Some(l) = &mut self.label {
            let x = l.get_position().x;
            l.set_position(Vector2::new(x, self.label_y + lift));
            l.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, if self.enabled { 1.0 } else { 0.45 }));
        }
        self.art.set_alpha(if self.enabled { 1.0 } else { 0.85 });
    }

    pub fn hit(&self, p: V2) -> bool {
        self.visible && self.enabled && self.rect.grow(6.0).contains(p)
    }

    /// Pointer went down; returns true if this button took it.
    pub fn down(&mut self, ctx: &mut Ctx, p: V2) -> bool {
        if self.hit(p) {
            self.pressed = true;
            self.redraw(ctx.riso);
            ctx.sfx(crate::sfx::Sfx::Tap);
            true
        } else {
            false
        }
    }

    /// Pointer released; returns true on a click.
    pub fn up(&mut self, ctx: &mut Ctx, p: V2) -> bool {
        if !self.pressed {
            return false;
        }
        self.pressed = false;
        self.redraw(ctx.riso);
        if self.hit(p) {
            self.bounce.pos = 0.88;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        self.redraw(ctx.riso);
        let s = self.bounce.step(dt, 380.0, 14.0);
        self.art.set_scale(s);
    }

    pub fn set_selected(&mut self, ctx: &mut Ctx, s: bool) {
        self.selected = s;
        self.redraw(ctx.riso);
    }
    pub fn set_enabled(&mut self, ctx: &mut Ctx, e: bool) {
        self.enabled = e;
        self.redraw(ctx.riso);
    }
    pub fn set_visible(&mut self, v: bool) {
        self.visible = v;
        self.art.set_visible(v);
    }
    pub fn set_text(&mut self, t: &str) {
        if let Some(l) = &mut self.label {
            l.set_text(t);
        }
    }
    pub fn set_z(&mut self, z: i32) {
        self.art.node.set_z_index(z);
    }
    pub fn free(self) {
        self.art.free();
    }
}

/// A set of buttons keyed by an id, routing pointer presses.
pub struct Panel<Id: Copy + PartialEq> {
    pub items: Vec<(Id, Button)>,
    active: Option<usize>,
}

impl<Id: Copy + PartialEq> Default for Panel<Id> {
    fn default() -> Self {
        Panel { items: Vec::new(), active: None }
    }
}

impl<Id: Copy + PartialEq> Panel<Id> {
    pub fn add(&mut self, id: Id, b: Button) {
        self.items.push((id, b));
    }
    pub fn get(&mut self, id: Id) -> Option<&mut Button> {
        self.items.iter_mut().find(|(i, _)| *i == id).map(|(_, b)| b)
    }
    pub fn down(&mut self, ctx: &mut Ctx, p: V2) -> bool {
        self.active = None;
        for (i, (_, b)) in self.items.iter_mut().enumerate().rev() {
            if b.down(ctx, p) {
                self.active = Some(i);
                return true;
            }
        }
        false
    }
    pub fn up(&mut self, ctx: &mut Ctx, p: V2) -> Option<Id> {
        let i = self.active.take()?;
        let (id, b) = &mut self.items[i];
        if b.up(ctx, p) { Some(*id) } else { None }
    }
    pub fn update(&mut self, ctx: &mut Ctx, dt: f32) {
        for (_, b) in &mut self.items {
            b.update(ctx, dt);
        }
    }
    pub fn clear(&mut self) {
        for (_, b) in self.items.drain(..) {
            b.free();
        }
        self.active = None;
    }
}

/// Where a bottom action bar's buttons go: one centred primary, or a secondary on the left
/// and the primary on the right. Returns `(secondary, primary)` rects.
pub fn bar_rects(y: f32, with_secondary: bool) -> (Rect, Rect) {
    if with_secondary {
        (
            rect(MARGIN, y - PILL_H * 0.5, 250.0, PILL_H),
            rect(MARGIN + 250.0 + 16.0, y - PILL_H * 0.5, 720.0 - 2.0 * MARGIN - 266.0, PILL_H),
        )
    } else {
        (rect(0.0, 0.0, 0.0, 0.0), rect(360.0 - 190.0, y - PILL_H * 0.5, 380.0, PILL_H))
    }
}

/// Centre line of the bottom action bar for a screen `h` tall with a bottom inset.
pub fn bar_y(ctx: &Ctx) -> f32 {
    ctx.lay.h - ctx.lay.bottom - 26.0 - PILL_H * 0.5
}

/// The printed masthead across the top: phase badge, day and edition, coins, level + XP,
/// and an optional headline ribbon for the current instruction.
pub struct Hud {
    root: Gd<Node2D>,
    day: Gd<Label>,
    coins: Gd<Label>,
    level: Gd<Label>,
    bar: Art,
    shown: (u32, u32, u32),
    ribbon: Option<(Art, Gd<Label>)>,
    said: String,
}

impl Hud {
    pub fn new(ctx: &mut Ctx, parent: &mut Gd<Node>, subtitle: &str) -> Hud {
        let mut root = Node2D::new_alloc();
        let top = ctx.lay.top;
        root.set_position(Vector2::new(0.0, top));
        parent.add_child(&root);
        let mut rn: Gd<Node> = root.clone().upcast();
        let badge_icon = if subtitle.contains("Zine") {
            Icon::Book
        } else if subtitle.contains("Night") {
            Icon::Moon
        } else {
            match ctx.state.phase {
                Phase::Morning => Icon::Sun,
                Phase::Shop => Icon::Shop,
                Phase::Evening => Icon::Moon,
            }
        };
        let strip = list(|d| {
            props::masthead(d, 720.0, MAST_H, top);
            // Phase badge.
            let c = v2(58.0, 46.0);
            let disc = proof_core::geom::circle(c, 31.0);
            d.backing(&disc);
            d.fill(Ink::Yellow, 0.12, &disc);
            d.stroke_p(
                proof_core::draw::Paint::solid(Ink::Pink, 0.9),
                5.0,
                &proof_core::geom::circle(c, 26.5),
                true,
            );
            d.outline(Ink::Key, 3.0, &disc);
            icon(d, badge_icon, c, 20.0);
        });
        let _strip = ctx.riso.art(&mut rn, V2::ZERO, &strip);
        let chips = list(|d| {
            d.with(Xf::at(v2(480.0, 47.0)), |d| props::chip(d, 146.0, 50.0));
            props::coin(d, v2(431.0, 47.0), 18.0);
            props::rosette(d, v2(590.0, 47.0), 27.0);
        });
        let _chips = ctx.riso.art(&mut rn, V2::ZERO, &chips);
        let day =
            ctx.riso.text(&mut rn, &TextSpec::new("", rect(100.0, 10.0, 250.0, 44.0), 33.0).bold().left());
        let _sub = ctx
            .riso
            .text(&mut rn, &TextSpec::new(subtitle, rect(101.0, 49.0, 290.0, 28.0), 18.0).left().plain());
        let coins = ctx
            .riso
            .text(&mut rn, &TextSpec::new("", rect(455.0, 22.0, 92.0, 50.0), 27.0).bold().left().plain());
        let level =
            ctx.riso.text(&mut rn, &TextSpec::new("", rect(566.0, 25.0, 48.0, 44.0), 22.0).bold().plain());
        let bar = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let mut h = Hud {
            root,
            day,
            coins,
            level,
            bar,
            shown: (u32::MAX, u32::MAX, u32::MAX),
            ribbon: None,
            said: String::new(),
        };
        h.refresh(ctx.riso, ctx.state);
        h
    }

    pub fn refresh(&mut self, riso: &mut Riso, s: &GameState) {
        let now = (s.day, s.coins, s.xp);
        if now == self.shown {
            return;
        }
        self.shown = now;
        self.day.set_text(&format!("Day {}", s.day));
        self.coins.set_text(&format!("{}", s.coins));
        self.level.set_text(&format!("{}", s.level()));
        let frac = economy::xp_to_next(s.xp).map(|(a, b)| a as f32 / b as f32).unwrap_or(1.0);
        let d = list(|d| d.with(Xf::at(v2(624.0, 40.0)), |d| props::meter(d, 74.0, 15.0, frac, Ink::Pink)));
        self.bar.set(riso, &d);
    }

    /// Show `text` on the headline ribbon under the masthead (empty hides it).
    pub fn say(&mut self, riso: &mut Riso, text: &str) {
        if text == self.said {
            return;
        }
        self.said = text.to_string();
        let size = 25.0;
        let tw = riso.bold.get_string_size_ex(text).font_size(size as i32).done().x;
        let w = (tw + 70.0).clamp(280.0, 600.0);
        let art = list(|d| d.with(Xf::at(v2(360.0, RIBBON_Y)), |d| props::ribbon(d, w, 42.0, Ink::Pink)));
        if self.ribbon.is_none() {
            let mut rn: Gd<Node> = self.root.clone().upcast();
            let a = riso.art(&mut rn, V2::ZERO, &DrawList::new());
            let l = riso.text(
                &mut rn,
                &TextSpec::new("", rect(40.0, RIBBON_Y - 23.0, 640.0, 44.0), size).bold().plain(),
            );
            self.ribbon = Some((a, l));
        }
        if let Some((a, l)) = &mut self.ribbon {
            a.set(riso, &art);
            a.set_visible(!text.is_empty());
            l.set_text(text);
            l.set_visible(!text.is_empty());
        }
    }
}

/// A short-lived floating label ("+12", "Loved it!").
pub struct Floater {
    pub label: Gd<Label>,
    sticker: Art,
    pub t: f32,
    pub life: f32,
    pub from: V2,
    pub rise: f32,
}

impl Floater {
    /// A word that pops up on a little paper sticker and floats away.
    pub fn new(riso: &mut Riso, parent: &mut Gd<Node>, text: &str, at: V2, size: f32, ink: Ink) -> Floater {
        let tw = riso.bold.get_string_size_ex(text).font_size(size as i32).done().x;
        let (w, h) = (tw + 40.0, size * 1.55);
        let tint = if ink == Ink::Pink { Ink::Yellow } else { Ink::Pink };
        let tilt = if text.len().is_multiple_of(2) { -0.05 } else { 0.05 };
        let sticker = riso.art(
            parent,
            at,
            &list(|d| d.with(Xf::IDENTITY.rotated(tilt), |d| props::sticker(d, w, h, tint))),
        );
        let t =
            TextSpec::new(text, rect(at.x - 150.0, at.y - 30.0, 300.0, 60.0), size).bold().ink(ink).plain();
        let label = riso.text(parent, &t);
        Floater { label, sticker, t: 0.0, life: 1.4, from: at, rise: 70.0 }
    }

    /// Returns false when finished (and freed).
    pub fn update(&mut self, dt: f32) -> bool {
        self.t += dt;
        let k = (self.t / self.life).min(1.0);
        let y = self.from.y - self.rise * proof_core::anim::ease_out_cubic(k);
        self.label.set_position(Vector2::new(self.from.x - 150.0, y - 30.0 - 2.0));
        let pop = 0.7 + 0.3 * proof_core::anim::ease_out_back((self.t / 0.25).min(1.0));
        self.sticker.set_pos(v2(self.from.x, y));
        self.sticker.set_scale(pop);
        let a = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
        self.label.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
        self.sticker.set_alpha(a);
        if k >= 1.0 {
            self.label.queue_free();
            let mut n = self.sticker.node.clone();
            if n.is_instance_valid() {
                n.queue_free();
            }
            false
        } else {
            true
        }
    }
}
