//! Printed widgets: buttons, the HUD strip, floating text. All hit-testing is in Rust.

use crate::riso::{Art, Riso, TextSpec, list};
use crate::screens::Ctx;
use godot::classes::{Label, Node, Node2D};
use godot::prelude::*;
use proof_core::anim::Spring;
use proof_core::art::icons::{Icon, icon};
use proof_core::art::props;
use proof_core::draw::DrawList;
use proof_core::economy;
use proof_core::geom::{Rect, V2, rect, v2};
use proof_core::ink::Ink;
use proof_core::state::GameState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Pill(Ink),
    Round(Ink, Icon),
    /// Just an icon, no plate.
    Bare(Icon),
}

pub struct Button {
    pub rect: Rect,
    art: Art,
    label: Option<Gd<Label>>,
    style: Style,
    pub enabled: bool,
    pub selected: bool,
    pub visible: bool,
    pressed: bool,
    drawn: Option<(bool, bool, bool)>,
    bounce: Spring,
}

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
        let label = if text.is_empty() {
            None
        } else {
            let mut an = art.as_node();
            let t = TextSpec::new(
                text,
                proof_core::geom::rect(-rect.w * 0.5, -rect.h * 0.5 - 3.0, rect.w, rect.h),
                size,
            )
            .bold()
            .plain();
            Some(riso.text(&mut an, &t))
        };
        let mut b = Button {
            rect,
            art,
            label,
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

    pub fn pill(ctx: &mut Ctx, parent: &mut Gd<Node>, rect: Rect, text: &str, ink: Ink) -> Button {
        let size = (rect.h * 0.42).clamp(20.0, 34.0);
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

    pub fn bare(ctx: &mut Ctx, parent: &mut Gd<Node>, c: V2, r: f32, ic: Icon) -> Button {
        Button::new(ctx.riso, parent, rect(c.x - r, c.y - r, r * 2.0, r * 2.0), Style::Bare(ic), "", 0.0)
    }

    fn redraw(&mut self, riso: &mut Riso) {
        let state = (self.pressed, self.selected, self.enabled);
        if self.drawn == Some(state) {
            return;
        }
        self.drawn = Some(state);
        let (w, h) = (self.rect.w, self.rect.h);
        let pressed = self.pressed;
        let selected = self.selected;
        let d = list(|d| match self.style {
            Style::Pill(ink) => props::button(d, w, h, if selected { Ink::Yellow } else { ink }, pressed),
            Style::Round(ink, ic) => {
                let y = if pressed { 4.0 } else { 0.0 };
                props::round_button(d, w * 0.5, ink, selected);
                icon(d, ic, v2(0.0, y - 1.0), w * 0.3);
            }
            Style::Bare(ic) => {
                if selected {
                    d.fill(Ink::Yellow, 0.9, &proof_core::geom::circle(V2::ZERO, w * 0.5));
                }
                icon(d, ic, V2::ZERO, w * 0.36);
            }
        });
        self.art.set(riso, &d);
        if let Some(l) = &mut self.label {
            l.set_position(Vector2::new(-w * 0.5, -h * 0.5 - 3.0 + if pressed { 5.0 } else { 0.0 }));
        }
        self.art.set_alpha(if self.enabled { 1.0 } else { 0.35 });
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
            self.bounce.pos = 0.86;
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

/// The printed strip across the top: day, coins, level.
pub struct Hud {
    _root: Gd<Node2D>,
    day: Gd<Label>,
    coins: Gd<Label>,
    level: Gd<Label>,
    bar: Art,
    shown: (u32, u32, u32),
}

impl Hud {
    pub fn new(ctx: &mut Ctx, parent: &mut Gd<Node>, subtitle: &str) -> Hud {
        let mut root = Node2D::new_alloc();
        root.set_position(Vector2::new(0.0, ctx.lay.top));
        parent.add_child(&root);
        let mut rn: Gd<Node> = root.clone().upcast();
        let strip = list(|d| {
            let r = rect(12.0, 12.0, 696.0, 70.0);
            props::tape(d, r.w, r.h, Ink::Yellow);
            let _ = r;
        });
        let _strip = ctx.riso.art(&mut rn, v2(360.0, 47.0), &strip);
        let icons = list(|d| {
            props::coin(d, v2(470.0, 47.0), 19.0);
            props::heart_icon(d, v2(610.0, 47.0), 36.0);
        });
        let _icons = ctx.riso.art(&mut rn, V2::ZERO, &icons);
        let day =
            ctx.riso.text(&mut rn, &TextSpec::new("", rect(34.0, 14.0, 330.0, 40.0), 30.0).bold().left());
        let _sub = ctx
            .riso
            .text(&mut rn, &TextSpec::new(subtitle, rect(34.0, 48.0, 330.0, 28.0), 19.0).left().plain());
        let coins =
            ctx.riso.text(&mut rn, &TextSpec::new("", rect(496.0, 22.0, 90.0, 50.0), 28.0).bold().left());
        let level =
            ctx.riso.text(&mut rn, &TextSpec::new("", rect(586.0, 26.0, 48.0, 40.0), 20.0).bold().plain());
        let bar = ctx.riso.art(&mut rn, V2::ZERO, &DrawList::new());
        let mut h = Hud { _root: root, day, coins, level, bar, shown: (u32::MAX, u32::MAX, u32::MAX) };
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
        let d = list(|d| {
            let track = proof_core::geom::rounded_rect(rect(640.0, 40.0, 56.0, 14.0), 7.0);
            d.backing(&track);
            d.outline(Ink::Key, 2.5, &track);
            if frac > 0.02 {
                d.fill(
                    Ink::Pink,
                    0.9,
                    &proof_core::geom::rounded_rect(rect(641.0, 41.0, 54.0 * frac, 12.0), 6.0),
                );
            }
        });
        self.bar.set(riso, &d);
    }
}

/// A short-lived floating label ("+12", "Loved it!").
pub struct Floater {
    pub label: Gd<Label>,
    pub t: f32,
    pub life: f32,
    pub from: V2,
    pub rise: f32,
}

impl Floater {
    pub fn new(riso: &Riso, parent: &mut Gd<Node>, text: &str, at: V2, size: f32, ink: Ink) -> Floater {
        let t = TextSpec::new(text, rect(at.x - 150.0, at.y - 30.0, 300.0, 60.0), size).bold().ink(ink);
        let label = riso.text(parent, &t);
        Floater { label, t: 0.0, life: 1.4, from: at, rise: 70.0 }
    }

    /// Returns false when finished (and freed).
    pub fn update(&mut self, dt: f32) -> bool {
        self.t += dt;
        let k = (self.t / self.life).min(1.0);
        let y = self.from.y - self.rise * proof_core::anim::ease_out_cubic(k);
        self.label.set_position(Vector2::new(self.from.x - 150.0, y - 30.0));
        let a = if k > 0.7 { 1.0 - (k - 0.7) / 0.3 } else { 1.0 };
        self.label.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
        if k >= 1.0 {
            self.label.queue_free();
            false
        } else {
            true
        }
    }
}
