//! Device-independent drawing commands for riso plates.
//!
//! Art functions push commands into a [`DrawList`]; `proof_raster` turns a list into four
//! antialiased ink plates. Semantics mirror real riso masters:
//! * every command targets one ink plate (or *knocks out* ink back to paper);
//! * [`Mode::Over`] replaces that plate's coverage inside the shape (last draw wins, like a
//!   single bitmap master), [`Mode::Add`] unions with what is already there;
//! * `tone` is ink density, rendered either as a flat tint or as a halftone screen.

use crate::geom::{Rect, V2, Xf};
use crate::ink::Ink;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    /// Flat tint (tone = ink density).
    Solid,
    /// Classic halftone dots at the ink's screen angle.
    Halftone,
    /// Halftone at double pitch, for bold decorative dots.
    Coarse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Replace plate coverage within the shape.
    Over,
    /// Union with existing coverage.
    Add,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub ink: Ink,
    pub tone: f32,
    pub screen: Screen,
    pub mode: Mode,
}

impl Paint {
    pub fn solid(ink: Ink, tone: f32) -> Paint {
        Paint { ink, tone, screen: Screen::Solid, mode: Mode::Over }
    }
    pub fn ht(ink: Ink, tone: f32) -> Paint {
        Paint { ink, tone, screen: Screen::Halftone, mode: Mode::Over }
    }
    pub fn coarse(ink: Ink, tone: f32) -> Paint {
        Paint { ink, tone, screen: Screen::Coarse, mode: Mode::Over }
    }
    pub fn add(self) -> Paint {
        Paint { mode: Mode::Add, ..self }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Closed polygon, non-zero fill.
    Poly(Vec<V2>),
    /// Several closed polygons filled even-odd (holes: bagels, rings).
    PolysEo(Vec<Vec<V2>>),
    /// Round-capped, round-joined line.
    Line { pts: Vec<V2>, width: f32, closed: bool },
}

impl Shape {
    fn bounds(&self) -> Option<Rect> {
        match self {
            Shape::Poly(p) => Rect::of_points(p),
            Shape::PolysEo(ps) => ps.iter().filter_map(|p| Rect::of_points(p)).reduce(|a, b| a.union(&b)),
            Shape::Line { pts, width, .. } => Rect::of_points(pts).map(|r| r.grow(width * 0.5 + 1.0)),
        }
    }
}

/// Which plates a knockout clears.
pub const PLATES_ALL: u8 = 0b1111;
pub const PLATES_COLOR: u8 = 0b0111;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Ink(Paint),
    /// Remove ink (reveal paper) with the given strength on the plates in `plates` bitmask.
    Knock {
        tone: f32,
        screen: Screen,
        plates: u8,
    },
    /// Paint the opaque paper backing mask (keeps cards opaque over other sprites).
    Backing,
    /// Intersect the clip with this shape until the matching `ClipPop`.
    ClipPush,
    ClipPop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cmd {
    pub op: Op,
    pub shape: Shape,
}

/// A list of plate commands with a transform stack for composing art.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub cmds: Vec<Cmd>,
    xf: Xf,
    stack: Vec<Xf>,
}

impl DrawList {
    pub fn new() -> Self {
        DrawList::default()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    pub fn xf(&self) -> Xf {
        self.xf
    }

    pub fn push(&mut self, xf: Xf) {
        self.stack.push(self.xf);
        self.xf = self.xf.then(&xf);
    }

    pub fn pop(&mut self) {
        self.xf = self.stack.pop().unwrap_or_default();
    }

    /// Draw with an extra local transform.
    pub fn with(&mut self, xf: Xf, f: impl FnOnce(&mut DrawList)) {
        self.push(xf);
        f(self);
        self.pop();
    }

    fn tx(&self, pts: &[V2]) -> Vec<V2> {
        pts.iter().map(|p| self.xf.apply(*p)).collect()
    }

    fn push_cmd(&mut self, op: Op, shape: Shape) {
        self.cmds.push(Cmd { op, shape });
    }

    fn poly_shape(&self, poly: &[V2]) -> Shape {
        Shape::Poly(self.tx(poly))
    }

    fn line_shape(&self, pts: &[V2], width: f32, closed: bool) -> Shape {
        Shape::Line { pts: self.tx(pts), width: width * self.xf.width_scale(), closed }
    }

    // --- fills -------------------------------------------------------------

    pub fn fill_p(&mut self, paint: Paint, poly: &[V2]) {
        if poly.len() >= 3 && paint.tone > 0.0 {
            let s = self.poly_shape(poly);
            self.push_cmd(Op::Ink(paint), s);
        }
    }

    /// Flat tint fill.
    pub fn fill(&mut self, ink: Ink, tone: f32, poly: &[V2]) {
        self.fill_p(Paint::solid(ink, tone), poly);
    }

    /// Halftone fill.
    pub fn ht(&mut self, ink: Ink, tone: f32, poly: &[V2]) {
        self.fill_p(Paint::ht(ink, tone), poly);
    }

    /// Halftone fill unioned with existing ink (blush over skin, etc.).
    pub fn ht_add(&mut self, ink: Ink, tone: f32, poly: &[V2]) {
        self.fill_p(Paint::ht(ink, tone).add(), poly);
    }

    pub fn fill_eo(&mut self, paint: Paint, polys: &[Vec<V2>]) {
        let s = Shape::PolysEo(polys.iter().map(|p| self.tx(p)).collect());
        self.push_cmd(Op::Ink(paint), s);
    }

    // --- lines -------------------------------------------------------------

    pub fn stroke_p(&mut self, paint: Paint, width: f32, pts: &[V2], closed: bool) {
        if pts.len() >= 2 && paint.tone > 0.0 {
            let s = self.line_shape(pts, width, closed);
            self.push_cmd(Op::Ink(paint), s);
        }
    }

    /// Solid open line.
    pub fn line(&mut self, ink: Ink, width: f32, pts: &[V2]) {
        self.stroke_p(Paint::solid(ink, 1.0), width, pts, false);
    }

    /// Solid closed outline.
    pub fn outline(&mut self, ink: Ink, width: f32, poly: &[V2]) {
        self.stroke_p(Paint::solid(ink, 1.0), width, poly, true);
    }

    // --- paper -------------------------------------------------------------

    /// Knock all ink out to bare paper.
    pub fn knock(&mut self, poly: &[V2]) {
        self.knock_p(1.0, Screen::Solid, PLATES_ALL, poly);
    }

    /// Knock colour plates only (key lines survive).
    pub fn knock_color(&mut self, poly: &[V2]) {
        self.knock_p(1.0, Screen::Solid, PLATES_COLOR, poly);
    }

    pub fn knock_p(&mut self, tone: f32, screen: Screen, plates: u8, poly: &[V2]) {
        if poly.len() >= 3 {
            let s = self.poly_shape(poly);
            self.push_cmd(Op::Knock { tone, screen, plates }, s);
        }
    }

    pub fn knock_line(&mut self, tone: f32, width: f32, pts: &[V2], closed: bool) {
        if pts.len() >= 2 {
            let s = self.line_shape(pts, width, closed);
            self.push_cmd(Op::Knock { tone, screen: Screen::Solid, plates: PLATES_ALL }, s);
        }
    }

    /// Opaque paper backing under this shape (also knocks ink from earlier commands).
    pub fn backing(&mut self, poly: &[V2]) {
        let s = self.poly_shape(poly);
        self.push_cmd(Op::Backing, s);
    }

    // --- clipping ----------------------------------------------------------

    pub fn clip_push(&mut self, poly: &[V2]) {
        let s = self.poly_shape(poly);
        self.push_cmd(Op::ClipPush, s);
    }

    pub fn clip_pop(&mut self) {
        self.push_cmd(Op::ClipPop, Shape::Poly(Vec::new()));
    }

    /// Draw `f` clipped to `poly`.
    pub fn clipped(&mut self, poly: &[V2], f: impl FnOnce(&mut DrawList)) {
        self.clip_push(poly);
        f(self);
        self.clip_pop();
    }

    // --- composition -------------------------------------------------------

    /// Append another list (already in absolute coordinates) through the current transform.
    pub fn append(&mut self, other: &DrawList) {
        for c in &other.cmds {
            let shape = match &c.shape {
                Shape::Poly(p) => Shape::Poly(self.tx(p)),
                Shape::PolysEo(ps) => Shape::PolysEo(ps.iter().map(|p| self.tx(p)).collect()),
                Shape::Line { pts, width, closed } => {
                    Shape::Line { pts: self.tx(pts), width: width * self.xf.width_scale(), closed: *closed }
                }
            };
            self.cmds.push(Cmd { op: c.op.clone(), shape });
        }
    }

    /// Bounds of everything that can put ink or backing on paper.
    pub fn bounds(&self) -> Option<Rect> {
        self.cmds
            .iter()
            .filter(|c| matches!(c.op, Op::Ink(_) | Op::Backing))
            .filter_map(|c| c.shape.bounds())
            .reduce(|a, b| a.union(&b))
    }

    /// Stable content hash (for texture caches).
    pub fn hash64(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for c in &self.cmds {
            match &c.op {
                Op::Ink(p) => {
                    eat(1);
                    eat(p.ink as u64);
                    eat(p.tone.to_bits() as u64);
                    eat(p.screen as u64);
                    eat(p.mode as u64);
                }
                Op::Knock { tone, screen, plates } => {
                    eat(2);
                    eat(tone.to_bits() as u64);
                    eat(*screen as u64);
                    eat(*plates as u64);
                }
                Op::Backing => eat(3),
                Op::ClipPush => eat(4),
                Op::ClipPop => eat(5),
            }
            let mut pts = |ps: &[V2]| {
                eat(ps.len() as u64);
                for p in ps {
                    eat(((p.x.to_bits() as u64) << 32) | p.y.to_bits() as u64);
                }
            };
            match &c.shape {
                Shape::Poly(p) => pts(p),
                Shape::PolysEo(ps) => ps.iter().for_each(|p| pts(p)),
                Shape::Line { pts: p, width, closed } => {
                    pts(p);
                    eat(width.to_bits() as u64);
                    eat(*closed as u64);
                }
            }
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{circle, v2};

    #[test]
    fn transform_stack_applies_and_restores() {
        let mut d = DrawList::new();
        d.with(Xf::at(v2(100.0, 0.0)).scaled(2.0), |d| {
            d.fill(Ink::Pink, 1.0, &circle(v2(0.0, 0.0), 10.0));
            d.line(Ink::Key, 3.0, &[v2(0.0, 0.0), v2(1.0, 0.0)]);
        });
        d.fill(Ink::Pink, 1.0, &circle(v2(0.0, 0.0), 10.0));
        let b0 = d.cmds[0].shape.bounds().unwrap();
        assert!((b0.center().x - 100.0).abs() < 0.5 && (b0.w - 40.0).abs() < 0.5);
        match &d.cmds[1].shape {
            Shape::Line { width, .. } => assert!((width - 6.0).abs() < 1e-4),
            _ => panic!(),
        }
        let b2 = d.cmds[2].shape.bounds().unwrap();
        assert!(b2.center().x.abs() < 0.5);
    }

    #[test]
    fn hash_changes_with_content() {
        let mut a = DrawList::new();
        a.fill(Ink::Pink, 1.0, &circle(v2(0.0, 0.0), 10.0));
        let mut b = DrawList::new();
        b.fill(Ink::Pink, 0.9, &circle(v2(0.0, 0.0), 10.0));
        assert_ne!(a.hash64(), b.hash64());
        assert_eq!(a.hash64(), a.clone().hash64());
    }
}
