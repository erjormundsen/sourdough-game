//! Screens are plain Rust structs owned by [`crate::game::Game`]. They build printed nodes,
//! react to pointer input and core events, and push [`Action`]s back through [`Ctx`].

pub mod evening;
pub mod morning;
pub mod night;
pub mod shop;

use crate::riso::Riso;
use crate::sfx::Sfx;
use godot::classes::Node2D;
use godot::prelude::*;
use proof_core::geom::V2;
use proof_core::state::{Action, Event, GameState, Reject};

/// Pointer input in reference coordinates (touch index 0 only).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ptr {
    Down(V2),
    Move(V2),
    Up(V2),
}

/// Visible canvas in reference units (720 wide; height depends on the phone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub w: f32,
    pub h: f32,
    /// Safe-area insets.
    pub top: f32,
    pub bottom: f32,
}

/// Where the game should go next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    /// The screen for the current phase.
    Phase,
    Night,
    Zine,
}

#[derive(Default)]
pub struct Out {
    pub actions: Vec<Action>,
    pub sfx: Vec<(Sfx, f32)>,
    pub buzz: Vec<i32>,
    pub toasts: Vec<String>,
    pub kick: f32,
    pub nav: Option<Nav>,
    pub notes: Vec<(String, String)>,
}

pub struct Ctx<'a> {
    pub state: &'a GameState,
    pub riso: &'a mut Riso,
    pub out: &'a mut Out,
    pub lay: Layout,
    pub time: f32,
}

impl Ctx<'_> {
    pub fn act(&mut self, a: Action) {
        self.out.actions.push(a);
    }
    pub fn sfx(&mut self, s: Sfx) {
        self.out.sfx.push((s, 1.0));
    }
    pub fn sfx_pitch(&mut self, s: Sfx, pitch: f32) {
        self.out.sfx.push((s, pitch));
    }
    pub fn buzz(&mut self, ms: i32) {
        self.out.buzz.push(ms);
    }
    pub fn toast(&mut self, t: impl Into<String>) {
        self.out.toasts.push(t.into());
    }
    /// Slam the inks out of register for a moment.
    pub fn kick(&mut self, k: f32) {
        self.out.kick = self.out.kick.max(k);
    }
    pub fn nav(&mut self, n: Nav) {
        self.out.nav = Some(n);
    }
    /// Show one of Grandma's notes (only the first time `key` comes up).
    pub fn note(&mut self, key: &str, text: &str) {
        if !self.state.saw_tip(key) && !self.out.notes.iter().any(|(k, _)| k == key) {
            self.out.notes.push((key.to_string(), text.to_string()));
        }
    }
}

pub trait Screen {
    fn root(&self) -> Gd<Node2D>;
    fn update(&mut self, ctx: &mut Ctx, dt: f32);
    fn pointer(&mut self, ctx: &mut Ctx, p: Ptr);
    fn events(&mut self, _ctx: &mut Ctx, _events: &[Event]) {}
    fn rejected(&mut self, ctx: &mut Ctx, r: Reject) {
        ctx.toast(r.message());
    }
    /// Autopilot for tours and smoke tests: take the next sensible step.
    /// Returns a short label for screenshots, or `None` while waiting on animation.
    fn auto(&mut self, ctx: &mut Ctx) -> Option<String>;
}

/// Build a fresh screen root under `host`.
pub fn new_root(host: &mut Gd<Node2D>) -> Gd<Node2D> {
    let root = Node2D::new_alloc();
    host.add_child(&root);
    root
}
