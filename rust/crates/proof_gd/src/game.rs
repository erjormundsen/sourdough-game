//! The root node: owns the game state and runs the Action → Event loop.

use crate::riso::{Art, Riso, TextSpec, list, v2of};
use crate::screens::{Ctx, Layout, Nav, Out, Ptr, Screen};
use crate::sfx::{Mixer, Sfx};
use crate::ui::Button;
use godot::classes::file_access::ModeFlags;
use godot::classes::notify::NodeNotification;
use godot::classes::{
    ColorRect, DirAccess, DisplayServer, FileAccess, INode2D, InputEvent, InputEventScreenDrag,
    InputEventScreenTouch, Label, Node, Node2D, Os, Time,
};
use godot::prelude::*;
use proof_core::anim::{Spring, ease_out_back};
use proof_core::art::props;
use proof_core::geom::{rect, v2};
use proof_core::ink::{Edition, Ink, Palette};
use proof_core::state::{Action, GameState, Phase};

const SAVE: &str = "user://save.json";

struct Toast {
    art: Art,
    label: Gd<Label>,
    t: f32,
    /// Shown in the low lane (above the action bar) instead of under the masthead.
    low: bool,
}

/// How long a toast stays up (seconds).
const TOAST_LIFE: f32 = 2.6;

struct Note {
    root: Gd<Node2D>,
    key: String,
    ok: Button,
    t: f32,
}

#[derive(Clone, Debug)]
enum Mode {
    Play,
    /// Walk the whole loop, screenshotting each step into `dir`.
    Tour {
        dir: String,
        days: u32,
        wait: f32,
        pending: Option<String>,
        shots: u32,
        steps: u32,
    },
    /// Headless smoke run for `days` days.
    Autoplay {
        days: u32,
        wait: f32,
        steps: u32,
    },
}

#[derive(GodotClass)]
#[class(base=Node2D)]
pub struct Game {
    base: Base<Node2D>,
    state: GameState,
    riso: Option<Riso>,
    mixer: Option<Mixer>,
    host: Option<Gd<Node2D>>,
    overlay: Option<Gd<Node2D>>,
    paper: Option<Gd<ColorRect>>,
    screen: Option<Box<dyn Screen>>,
    out: Out,
    time: f32,
    kick: Spring,
    pal_from: Palette,
    pal_to: Palette,
    pal_t: f32,
    toasts: Vec<Toast>,
    /// The current screen wants toasts in the low lane (the night pages' nameplate lives
    /// where the top lane is).
    toast_low: bool,
    note: Option<Note>,
    mode: Mode,
    lay: Layout,
    touching: bool,
    autosave: f32,
    note_queue: Vec<(String, String)>,
}

/// Saves the bakery when the app is paused or closed. Lives in its own node so engine
/// notifications never re-enter `Game` while it is busy.
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct Keeper {
    base: Base<Node>,
    game: Option<Gd<Game>>,
}

#[godot_api]
impl godot::classes::INode for Keeper {
    fn on_notification(&mut self, what: NodeNotification) {
        if matches!(what, NodeNotification::WM_CLOSE_REQUEST | NodeNotification::APPLICATION_PAUSED)
            && let Some(g) = self.game.as_mut()
        {
            g.bind().save();
        }
    }
}

#[godot_api]
impl INode2D for Game {
    fn init(base: Base<Node2D>) -> Self {
        let p = Edition::Dawn.palette();
        Game {
            base,
            state: GameState::new(1),
            riso: None,
            mixer: None,
            host: None,
            overlay: None,
            paper: None,
            screen: None,
            out: Out::default(),
            time: 0.0,
            kick: Spring::new(0.0),
            pal_from: p,
            pal_to: p,
            pal_t: 1.0,
            toasts: Vec::new(),
            toast_low: false,
            note: None,
            mode: Mode::Play,
            lay: Layout { w: 720.0, h: 1280.0, top: 0.0, bottom: 0.0 },
            touching: false,
            autosave: 0.0,
            note_queue: Vec::new(),
        }
    }

    fn ready(&mut self) {
        let args: Vec<String> =
            Os::singleton().get_cmdline_user_args().as_slice().iter().map(|s| s.to_string()).collect();
        let arg = |name: &str| -> Option<String> {
            args.iter().find_map(|a| a.strip_prefix(&format!("--{name}=")).map(|v| v.to_string()))
        };
        let flag = |name: &str| args.iter().any(|a| a == &format!("--{name}"));
        let seed = arg("seed")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| Time::singleton().get_ticks_msec() ^ 0x5eed);
        let days = arg("days").and_then(|s| s.parse().ok()).unwrap_or(1u32);
        if flag("tour") {
            let dir = arg("shots").unwrap_or_else(|| "user://tour".into());
            self.mode = Mode::Tour { dir, days, wait: 1.5, pending: None, shots: 0, steps: 0 };
        } else if let Some(n) = arg("autoplay").and_then(|s| s.parse().ok()) {
            self.mode = Mode::Autoplay { days: n, wait: 0.2, steps: 0 };
        }
        let fresh = flag("fresh") || !matches!(self.mode, Mode::Play);
        self.state =
            if fresh { GameState::new(seed) } else { self.load().unwrap_or_else(|| GameState::new(seed)) };
        godot_print!(
            "Proof: day {} ({:?}), seed {}, mode {:?}",
            self.state.day,
            self.state.phase,
            self.state.seed,
            self.mode
        );

        self.lay = self.measure();
        let scale = self.raster_scale();
        let pal = edition_for(self.state.phase).palette();
        self.pal_from = pal;
        self.pal_to = pal;
        self.riso = Some(Riso::new(scale, pal));

        let mut base: Gd<Node> = self.base().clone().upcast();
        let mut paper = ColorRect::new_alloc();
        paper.set_position(Vector2::new(-400.0, -400.0));
        paper.set_size(Vector2::new(self.lay.w + 800.0, self.lay.h + 800.0));
        paper.set_material(&self.riso.as_ref().unwrap().paper_mat);
        paper.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        base.add_child(&paper);
        self.paper = Some(paper);
        let host = Node2D::new_alloc();
        base.add_child(&host);
        self.host = Some(host);
        let mut overlay = Node2D::new_alloc();
        overlay.set_z_index(50);
        base.add_child(&overlay);
        self.overlay = Some(overlay);
        self.mixer = Some(Mixer::new(&mut base));
        let mut keeper = Keeper::new_alloc();
        keeper.bind_mut().game = Some(self.to_gd());
        base.add_child(&keeper);
        self.center_host();
        self.switch(Nav::Phase);
    }

    fn process(&mut self, dt: f64) {
        let dt = dt as f32;
        self.time += dt;
        // Print-in kick settles back into register.
        let k = self.kick.step(dt, 170.0, 13.0);
        // Edition crossfade.
        if self.pal_t < 1.0 {
            self.pal_t = (self.pal_t + dt / 0.9).min(1.0);
        }
        let pal = self.pal_from.lerp(&self.pal_to, proof_core::anim::ease_in_out(self.pal_t));
        let prefs = self.state.settings;
        if let Some(m) = self.mixer.as_mut() {
            m.muted = !prefs.sound;
            m.haptics = prefs.haptics;
        }
        if let Some(r) = self.riso.as_mut() {
            r.calm = prefs.calm;
            r.apply_kick(k.abs());
            r.apply_palette(pal);
        }
        self.autosave += dt;
        if self.autosave > 20.0 {
            self.autosave = 0.0;
            self.save();
        }
        self.update_screen(dt);
        self.update_overlays(dt);
        self.flush();
        self.drive(dt);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let local = self.base().make_input_local(&event);
        let host_x = self.host.as_ref().map(|h| h.get_position().x).unwrap_or(0.0);
        let ptr = if let Ok(t) = local.clone().try_cast::<InputEventScreenTouch>() {
            if t.get_index() != 0 {
                return;
            }
            let p = v2of(t.get_position()) - v2(host_x, 0.0);
            if t.is_pressed() {
                self.touching = true;
                Ptr::Down(p)
            } else {
                self.touching = false;
                Ptr::Up(p)
            }
        } else if let Ok(d) = local.try_cast::<InputEventScreenDrag>() {
            if d.get_index() != 0 || !self.touching {
                return;
            }
            Ptr::Move(v2of(d.get_position()) - v2(host_x, 0.0))
        } else {
            return;
        };
        if matches!(self.mode, Mode::Play) {
            self.pointer(ptr);
        }
    }
}

/// Grandma's note sits in the middle of the safe area.
fn note_centre(lay: Layout) -> f32 {
    lay.top + (lay.h - lay.top - lay.bottom) * 0.5
}

fn edition_for(phase: Phase) -> Edition {
    match phase {
        Phase::Morning => Edition::Dawn,
        Phase::Shop => Edition::Daylight,
        Phase::Evening => Edition::Dusk,
    }
}

impl Game {
    fn measure(&self) -> Layout {
        let vr = self.base().get_viewport_rect();
        let (vw, vh) = (vr.size.x.max(1.0), vr.size.y.max(1.0));
        let ds = DisplayServer::singleton();
        let safe = ds.get_display_safe_area();
        let win =
            self.base().get_window().map(|w| w.get_size()).unwrap_or(Vector2i::new(vw as i32, vh as i32));
        let px_per_unit = (win.y as f32 / vh).max(0.01);
        let (top, bottom) = if safe.size.y > 0 && win.y > 0 {
            let top = (safe.position.y as f32 / px_per_unit).clamp(0.0, 80.0);
            // Home-indicator strip (only when the window fills the display).
            let below = (win.y - (safe.position.y + safe.size.y)) as f32 / px_per_unit;
            (top, below.clamp(0.0, 40.0))
        } else {
            (0.0, 0.0)
        };
        Layout { w: vw, h: vh, top, bottom }
    }

    fn raster_scale(&self) -> f32 {
        let vr = self.base().get_viewport_rect();
        let win = self.base().get_window().map(|w| w.get_size()).unwrap_or(Vector2i::new(720, 1280));
        (win.y as f32 / vr.size.y.max(1.0)).clamp(1.0, 3.0)
    }

    fn center_host(&mut self) {
        let x = ((self.lay.w - 720.0) * 0.5).max(0.0);
        if let Some(h) = self.host.as_mut() {
            h.set_position(Vector2::new(x, 0.0));
        }
        if let Some(o) = self.overlay.as_mut() {
            o.set_position(Vector2::new(x, 0.0));
        }
        self.lay.w = 720.0;
    }

    fn ctx_parts(&mut self) -> (Ctx<'_>, Option<&mut Box<dyn Screen>>) {
        let ctx = Ctx {
            state: &self.state,
            riso: self.riso.as_mut().expect("riso"),
            out: &mut self.out,
            lay: self.lay,
            time: self.time,
        };
        (ctx, self.screen.as_mut())
    }

    fn update_screen(&mut self, dt: f32) {
        if self.note.is_some() {
            // Notes pause the screen underneath (but keep it breathing).
            let (mut ctx, screen) = self.ctx_parts();
            if let Some(s) = screen {
                s.update(&mut ctx, dt * 0.25);
            }
            return;
        }
        let (mut ctx, screen) = self.ctx_parts();
        if let Some(s) = screen {
            s.update(&mut ctx, dt);
        }
    }

    fn pointer(&mut self, p: Ptr) {
        if let Some(mut note) = self.note.take() {
            let (mut ctx, _) = self.ctx_parts();
            let done = match p {
                Ptr::Down(q) => {
                    note.ok.down(&mut ctx, q);
                    false
                }
                Ptr::Up(q) => note.ok.up(&mut ctx, q),
                Ptr::Move(_) => false,
            };
            if done {
                ctx.act(Action::Tip(note.key.clone()));
                ctx.sfx(Sfx::Plop);
                let mut r = note.root;
                r.queue_free();
            } else {
                self.note = Some(note);
            }
            self.flush();
            return;
        }
        let (mut ctx, screen) = self.ctx_parts();
        if let Some(s) = screen {
            s.pointer(&mut ctx, p);
        }
        self.flush();
    }

    /// Apply queued actions, deliver events, then handle side effects.
    fn flush(&mut self) {
        for _ in 0..32 {
            if self.out.actions.is_empty() {
                break;
            }
            let actions = std::mem::take(&mut self.out.actions);
            for a in actions {
                let before = self.state.phase;
                let res = self.state.apply(a);
                let changed = self.state.phase != before;
                let (mut ctx, screen) = self.ctx_parts();
                match res {
                    Ok(ev) => {
                        if let Some(s) = screen {
                            s.events(&mut ctx, &ev);
                        }
                    }
                    Err(r) => {
                        if let Some(s) = screen {
                            s.rejected(&mut ctx, r);
                        }
                    }
                }
                if changed {
                    self.out.nav = Some(Nav::Phase);
                    self.save();
                }
            }
        }
        self.side_effects();
    }

    fn side_effects(&mut self) {
        let sfx = std::mem::take(&mut self.out.sfx);
        let buzz = std::mem::take(&mut self.out.buzz);
        if let Some(m) = self.mixer.as_mut() {
            for (s, p) in sfx {
                m.play(s, p);
            }
            for ms in buzz {
                m.buzz(ms);
            }
        }
        if self.out.kick > 0.0 {
            self.kick.pos = self.kick.pos.max(self.out.kick);
            self.kick.vel = 0.0;
            self.out.kick = 0.0;
        }
        for t in std::mem::take(&mut self.out.toasts) {
            self.spawn_toast(&t);
        }
        for n in std::mem::take(&mut self.out.notes) {
            if !self.note_queue.iter().any(|(k, _)| *k == n.0) {
                self.note_queue.push(n);
            }
        }
        self.note_queue.retain(|(k, _)| !self.state.saw_tip(k));
        if self.note.is_none()
            && !self.note_queue.is_empty()
            && matches!(self.mode, Mode::Play | Mode::Tour { .. })
        {
            let (key, text) = self.note_queue.remove(0);
            self.spawn_note(key, &text);
        }
        if let Some(nav) = self.out.nav.take() {
            self.switch(nav);
        }
    }

    fn switch(&mut self, nav: Nav) {
        let mut host = self.host.clone().expect("host");
        if let Some(old) = self.screen.take() {
            let mut r = old.root();
            r.queue_free();
        }
        let edition = match nav {
            Nav::Phase => edition_for(self.state.phase),
            Nav::Night | Nav::Zine => Edition::Dusk,
        };
        let cur = self.pal_from.lerp(&self.pal_to, self.pal_t);
        self.pal_from = cur;
        self.pal_to = edition.palette();
        self.pal_t = if cur == self.pal_to { 1.0 } else { 0.0 };
        // Old news fades out quickly on a new page; pick this page's toast lane.
        for t in &mut self.toasts {
            t.t = t.t.max(TOAST_LIFE - 0.4);
        }
        self.toast_low = matches!(nav, Nav::Night | Nav::Zine);
        let (mut ctx, _) = self.ctx_parts();
        let screen: Box<dyn Screen> = match nav {
            Nav::Phase => match ctx.state.phase {
                Phase::Morning => Box::new(crate::screens::morning::Morning::new(&mut ctx, &mut host)),
                Phase::Shop => Box::new(crate::screens::shop::Shop::new(&mut ctx, &mut host)),
                Phase::Evening => Box::new(crate::screens::evening::Evening::new(&mut ctx, &mut host)),
            },
            Nav::Night => Box::new(crate::screens::night::Night::new(&mut ctx, &mut host, false)),
            Nav::Zine => Box::new(crate::screens::night::Night::new(&mut ctx, &mut host, true)),
        };
        ctx.kick(1.0);
        ctx.sfx(Sfx::Whoosh);
        self.screen = Some(screen);
        self.side_effects();
    }

    // ------------------------------------------------------------------------------------
    // Overlays: toasts and Grandma's notes
    // ------------------------------------------------------------------------------------

    /// Toasts hang in a lane just below the masthead and its headline ribbon, or (low lane)
    /// stack upward from just above the bottom action bar.
    fn toast_y(&self, i: usize, low: bool) -> f32 {
        if low {
            let bar_top = self.lay.h - self.lay.bottom - 26.0 - crate::ui::PILL_H;
            bar_top - 52.0 - i as f32 * 72.0
        } else {
            self.lay.top + 190.0 + i as f32 * 72.0
        }
    }

    fn spawn_toast(&mut self, text: &str) {
        let mut ov: Gd<Node> = self.overlay.clone().expect("overlay").upcast();
        let low = self.toast_low;
        let y = self.toast_y(self.toasts.iter().filter(|t| t.low == low).count(), low);
        let riso = self.riso.as_mut().expect("riso");
        let size = 23.0;
        let tw = riso.bold.get_string_size_ex(text).font_size(size as i32).done().x;
        let (w, h) = if tw + 104.0 <= 660.0 { ((tw + 104.0).max(320.0), 60.0) } else { (660.0, 88.0) };
        let stub = 57.0;
        let art = riso.art(&mut ov, v2(360.0, y), &list(|d| props::slip(d, w, h, Ink::Yellow)));
        let mut an = art.as_node();
        let spec = TextSpec::new(text, rect(-w * 0.5 + stub + 8.0, -h * 0.5 - 2.0, w - stub - 20.0, h), size)
            .bold()
            .plain();
        let label = riso.text(&mut an, &if h > 60.0 { spec.wrap() } else { spec });
        self.toasts.push(Toast { art, label, t: 0.0, low });
        if self.toasts.len() > 3 {
            let t = self.toasts.remove(0);
            t.art.free();
        }
    }

    fn spawn_note(&mut self, key: String, text: &str) {
        let mut ov: Gd<Node> = self.overlay.clone().expect("overlay").upcast();
        let lay = self.lay;
        let riso = self.riso.as_mut().expect("riso");
        let mut root = Node2D::new_alloc();
        ov.add_child(&root);
        let mut rn: Gd<Node> = root.clone().upcast();
        let mut dim = ColorRect::new_alloc();
        dim.set_position(Vector2::new(-400.0, -200.0));
        dim.set_size(Vector2::new(1520.0, lay.h + 400.0));
        let mut c = riso.ink(Ink::Key);
        c.a = 0.32;
        dim.set_color(c);
        dim.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        rn.add_child(&dim);
        // Size the card to the message: one ruled line (40 units) per line of text.
        let tw = riso.font.get_string_size_ex(text).font_size(26).done().x;
        let lines = ((tw / ((628.0 - 104.0) * 0.86)).ceil() as i32).max(2) as f32;
        let (cw, ch) = (628.0, (104.0 + lines * 40.0 + 206.0).clamp(440.0, 640.0));
        let cy = note_centre(lay);
        let (x0, y0) = (360.0 - cw * 0.5, cy - ch * 0.5);
        let card = list(|d| {
            props::note_card(d, cw, ch);
            props::grandma(d, v2(-cw * 0.5 + 74.0, -ch * 0.5 + 40.0), 52.0);
            proof_core::art::twinkle(d, v2(cw * 0.5 - 40.0, -ch * 0.5 + 46.0), 13.0, Ink::Yellow);
            proof_core::art::props::heart_icon(d, v2(cw * 0.5 - 70.0, ch * 0.5 - 148.0), 26.0);
        });
        let _card = riso.art(&mut rn, v2(360.0, cy), &card);
        riso.text(
            &mut rn,
            &TextSpec::new("A note from Grandma", rect(x0 + 140.0, y0 + 20.0, cw - 170.0, 56.0), 28.0)
                .bold()
                .left(),
        );
        // The body sits on the card's ruled lines (40 units apart, first at y0 + 136).
        let mut body = riso.text(
            &mut rn,
            &TextSpec::new(text, rect(x0 + 66.0, y0 + 104.0, cw - 104.0, 280.0), 26.0).wrap().left().plain(),
        );
        body.add_theme_constant_override("line_spacing", 8);
        body.set_vertical_alignment(godot::global::VerticalAlignment::TOP);
        riso.text(
            &mut rn,
            &TextSpec::new("— Grandma", rect(x0 + 260.0, y0 + ch - 170.0, cw - 360.0, 40.0), 24.0)
                .right()
                .plain(),
        );
        let mut ctx = Ctx { state: &self.state, riso, out: &mut self.out, lay, time: self.time };
        let ok = Button::pill(
            &mut ctx,
            &mut rn,
            rect(360.0 - 130.0, y0 + ch - 112.0, 260.0, 76.0),
            "Got it!",
            Ink::Pink,
        );
        root.set_scale(Vector2::new(0.6, 0.6));
        self.note = Some(Note { root, key, ok, t: 0.0 });
    }

    fn update_overlays(&mut self, dt: f32) {
        let mut keep = Vec::new();
        let mut lanes = [0usize; 2];
        for mut t in std::mem::take(&mut self.toasts) {
            t.t += dt;
            let life = TOAST_LIFE;
            if t.t >= life {
                t.art.free();
                continue;
            }
            let i = lanes[usize::from(t.low)];
            lanes[usize::from(t.low)] += 1;
            let y = self.toast_y(i, t.low);
            let slide = ease_out_back((t.t / 0.35).min(1.0));
            t.art.set_pos(v2(360.0, y - 60.0 * (1.0 - slide)));
            let a = if t.t > life - 0.4 { (life - t.t) / 0.4 } else { 1.0 };
            t.art.set_alpha(a);
            t.label.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a));
            keep.push(t);
        }
        self.toasts = keep;
        if let Some(n) = self.note.as_mut() {
            n.t += dt;
            let s = 0.6 + 0.4 * ease_out_back((n.t / 0.4).min(1.0));
            let c = Vector2::new(360.0, note_centre(self.lay));
            n.root.set_scale(Vector2::new(s, s));
            n.root.set_position(c - c * s);
            let riso = self.riso.as_mut().expect("riso");
            let mut ctx =
                Ctx { state: &self.state, riso, out: &mut self.out, lay: self.lay, time: self.time };
            n.ok.update(&mut ctx, dt);
        }
    }

    // ------------------------------------------------------------------------------------
    // Save / load
    // ------------------------------------------------------------------------------------

    fn save(&self) {
        if !matches!(self.mode, Mode::Play) {
            return;
        }
        let tmp = format!("{SAVE}.tmp");
        if let Some(mut f) = FileAccess::open(&tmp, ModeFlags::WRITE) {
            f.store_string(&self.state.to_json());
            f.close();
            let _ = DirAccess::rename_absolute(&tmp, SAVE);
        }
    }

    fn load(&self) -> Option<GameState> {
        if !FileAccess::file_exists(SAVE) {
            return None;
        }
        let s = FileAccess::get_file_as_string(SAVE).to_string();
        match GameState::from_json(&s) {
            Ok(g) => Some(g),
            Err(e) => {
                godot_warn!("Proof: could not read save ({e}); starting fresh");
                None
            }
        }
    }

    // ------------------------------------------------------------------------------------
    // Tour & autoplay drivers
    // ------------------------------------------------------------------------------------

    fn drive(&mut self, dt: f32) {
        let mut mode = self.mode.clone();
        match &mut mode {
            Mode::Play => return,
            Mode::Tour { dir, days, wait, pending, shots, steps } => {
                *wait -= dt;
                if *wait > 0.0 {
                    self.mode = mode;
                    return;
                }
                if let Some(label) = pending.take() {
                    *shots += 1;
                    self.screenshot(dir, &format!("{:02}_{}", shots, label));
                }
                if self.state.day > *days || *steps > 400 {
                    godot_print!("Proof tour: {} shots over {} steps", shots, steps);
                    self.quit(if *steps > 400 { 1 } else { 0 });
                    return;
                }
                *steps += 1;
                if let Some(n) = self.note.as_ref() {
                    if n.t < 0.5 {
                        // Let it pop in, then photograph it before dismissing.
                        *wait = 0.6;
                        *pending = Some(format!("note_{}", n.key));
                        self.mode = mode;
                        return;
                    }
                    let n = self.note.take().unwrap();
                    self.out.actions.push(Action::Tip(n.key.clone()));
                    let mut r = n.root;
                    r.queue_free();
                    *wait = 0.3;
                    self.mode = mode;
                    self.flush();
                    return;
                }
                let label = {
                    let (mut ctx, screen) = self.ctx_parts();
                    screen.and_then(|s| s.auto(&mut ctx))
                };
                *wait = if label.is_some() { 1.3 } else { 0.25 };
                *pending = label;
            }
            Mode::Autoplay { days, wait, steps } => {
                *wait -= dt;
                if *wait > 0.0 {
                    self.mode = mode;
                    return;
                }
                if self.state.day > *days {
                    godot_print!(
                        "Proof autoplay: reached day {} in {} steps, {} coins, level {}",
                        self.state.day,
                        steps,
                        self.state.coins,
                        self.state.level()
                    );
                    self.quit(0);
                    return;
                }
                if *steps > 3000 {
                    godot_error!("Proof autoplay: stuck on day {} ({:?})", self.state.day, self.state.phase);
                    self.quit(1);
                    return;
                }
                *steps += 1;
                if let Some(n) = self.note.take() {
                    let mut r = n.root;
                    r.queue_free();
                }
                let (mut ctx, screen) = self.ctx_parts();
                let _ = screen.and_then(|s| s.auto(&mut ctx));
                *wait = 0.12;
            }
        }
        self.mode = mode;
        self.flush();
    }

    fn screenshot(&self, dir: &str, name: &str) {
        let Some(vp) = self.base().get_viewport() else { return };
        let Some(tex) = vp.get_texture() else { return };
        let Some(img) = tex.get_image() else { return };
        let _ = DirAccess::make_dir_recursive_absolute(dir);
        let path = format!("{dir}/{name}.png");
        img.save_png(&path);
        godot_print!("shot {path}");
    }

    fn quit(&mut self, code: i32) {
        if let Some(m) = self.mixer.as_mut() {
            m.stop_all();
        }
        let mut tree = self.base().get_tree();
        tree.quit_ex().exit_code(code).done();
    }
}
