//! The riso press: turns core `DrawList`s into textures and builds printed nodes.
//!
//! Every printed layer shares the press model in `riso_press.gdshaderinc`: the ink material
//! (plates → screened ink with per-drum misregistration and ink film), the text material, the
//! paper backing and the page itself all read the same paper/ink textures and the same
//! [`PressStyle`] uniforms as the artboard's CPU preview (`proof_raster::press`). Edition
//! crossfades and the print-in "kick" are just uniform updates.

use godot::classes::canvas_item::TextureFilter;
use godot::classes::image::Format;
use godot::classes::text_server::AutowrapMode;
use godot::classes::{
    Font, FontFile, FontVariation, Image, ImageTexture, Label, Node, Node2D, Shader, ShaderMaterial, Sprite2D,
};
use godot::global::{HorizontalAlignment, VerticalAlignment};
use godot::prelude::*;
use proof_core::draw::DrawList;
use proof_core::geom::{V2, Xf};
use proof_core::ink::{Ink, Palette, Rgb};
use proof_raster::{Param, PressStyle, PressTextures, RasterConfig, Tex, press_textures, rasterize};
use std::collections::HashMap;
use std::sync::Arc;

pub fn vec2(v: V2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

pub fn v2of(v: Vector2) -> V2 {
    proof_core::geom::v2(v.x, v.y)
}

pub fn color(c: Rgb) -> Color {
    Color::from_rgb(c.0, c.1, c.2)
}

struct Cached {
    ink: Gd<ImageTexture>,
    backing: Option<Gd<ImageTexture>>,
    origin: V2,
}

/// Owns materials, the texture cache and the font.
pub struct Riso {
    /// Device pixels per reference unit (raster resolution).
    pub scale: f32,
    pub palette: Palette,
    pub ink_mat: Gd<ShaderMaterial>,
    pub backing_mat: Gd<ShaderMaterial>,
    pub paper_mat: Gd<ShaderMaterial>,
    pub text_mat: Gd<ShaderMaterial>,
    pub font: Gd<Font>,
    pub bold: Gd<Font>,
    cache: HashMap<u64, Cached>,
    cfg: RasterConfig,
    /// Current print-in kick (0 = in register).
    pub kick: f32,
    /// How the press prints: drum registration, ink film, paper (shared with the artboard).
    pub style: PressStyle,
    press: Arc<PressTextures>,
    /// Palette last sent to the materials.
    applied: Option<Palette>,
    /// Reduce motion: smaller kicks and offsets.
    pub calm: bool,
}

fn texture(t: &Tex) -> Gd<ImageTexture> {
    let n = t.size as i32;
    let img = Image::create_from_data(n, n, false, Format::RGBA8, &PackedByteArray::from(t.rgba.as_slice()))
        .expect("press image");
    ImageTexture::create_from_image(&img).expect("press texture")
}

fn material(path: &str, tex: &[(&str, &Gd<ImageTexture>)]) -> Gd<ShaderMaterial> {
    let mut m = ShaderMaterial::new_gd();
    m.set_shader(&load::<Shader>(path));
    for (name, t) in tex {
        m.set_shader_parameter(*name, &t.to_variant());
    }
    m
}

fn set_param(m: &mut Gd<ShaderMaterial>, name: &str, p: Param) {
    let v = match p {
        Param::F(x) => x.to_variant(),
        Param::V2(x, y) => Vector2::new(x, y).to_variant(),
        Param::V4(a) => Vector4::new(a[0], a[1], a[2], a[3]).to_variant(),
    };
    m.set_shader_parameter(name, &v);
}

fn weighted(base: &Gd<FontFile>, weight: i64) -> Gd<Font> {
    let mut v = FontVariation::new_gd();
    v.set_base_font(base);
    let mut d = VarDictionary::new();
    d.set("wght", weight);
    v.set_variation_opentype(&d);
    v.upcast()
}

impl Riso {
    pub fn new(scale: f32, palette: Palette) -> Riso {
        let press = press_textures(scale);
        let (fine, mid, coarse) = (texture(&press.fine), texture(&press.mid), texture(&press.coarse));
        let tex = [("fine_tex", &fine), ("mid_tex", &mid), ("coarse_tex", &coarse)];
        let ink_mat = material("res://shaders/riso_ink.gdshader", &tex);
        let backing_mat = material("res://shaders/riso_backing.gdshader", &tex);
        let paper_mat = material("res://shaders/paper.gdshader", &tex);
        let text_mat = material("res://shaders/riso_text.gdshader", &tex);
        let file = load::<FontFile>("res://fonts/Fredoka.ttf");
        let font = weighted(&file, 560);
        let bold = weighted(&file, 680);
        let mut r = Riso {
            scale,
            palette,
            ink_mat,
            backing_mat,
            paper_mat,
            text_mat,
            font,
            bold,
            cache: HashMap::new(),
            cfg: RasterConfig { scale, ..RasterConfig::default() },
            kick: 0.0,
            style: PressStyle::default(),
            press,
            applied: None,
            calm: false,
        };
        r.apply_style();
        r.apply_palette(palette);
        r.apply_kick(0.0);
        r
    }

    /// Send the whole press style to every material (paper, ink film, drums).
    pub fn apply_style(&mut self) {
        let params = self.style.shader_params(&self.press, self.scale);
        for m in [&mut self.ink_mat, &mut self.backing_mat, &mut self.paper_mat, &mut self.text_mat] {
            for (name, p) in &params {
                set_param(m, name, *p);
            }
        }
    }

    pub fn apply_palette(&mut self, p: Palette) {
        if self.applied == Some(p) {
            return;
        }
        self.applied = Some(p);
        self.palette = p;
        for (i, ink) in p.inks.iter().enumerate() {
            self.ink_mat.set_shader_parameter(&format!("ink{i}"), &color(*ink).to_variant());
        }
        for m in [&mut self.ink_mat, &mut self.backing_mat, &mut self.paper_mat, &mut self.text_mat] {
            m.set_shader_parameter("paper", &color(p.paper).to_variant());
        }
    }

    /// Slam the drums out of register (`kick` 0..1, springing back to 0).
    pub fn apply_kick(&mut self, kick: f32) {
        let (kick, k_off) = if self.calm { (kick * 0.15, 0.5) } else { (kick, 1.0) };
        if (kick - self.kick).abs() < 1e-4 && kick != 0.0 {
            return;
        }
        self.kick = kick;
        let mut st = self.style;
        st.kick = kick;
        st.offsets = st.offsets.map(|o| o * k_off);
        const SHIFT: [&str; 4] = ["shift0", "shift1", "shift2", "shift3"];
        const LIN: [&str; 4] = ["lin0", "lin1", "lin2", "lin3"];
        for k in 0..4 {
            let (sh, a) = st.drums[k].affine(st.offsets[k], kick);
            set_param(&mut self.ink_mat, SHIFT[k], Param::V2(sh.x * self.scale, sh.y * self.scale));
            set_param(&mut self.ink_mat, LIN[k], Param::V4(a));
        }
    }

    pub fn ink(&self, ink: Ink) -> Color {
        color(self.palette.inks[ink.idx()])
    }

    fn textures(&mut self, list: &DrawList, extra_scale: f32) -> Option<(u64, &Cached)> {
        if list.is_empty() {
            return None;
        }
        let scale = (self.scale * extra_scale).clamp(0.5, 4.0);
        let key = list.hash64() ^ (scale.to_bits() as u64).rotate_left(17);
        if !self.cache.contains_key(&key) {
            if self.cache.len() > 400 {
                self.cache.clear();
            }
            let cfg = RasterConfig { scale, ..self.cfg };
            let pl = rasterize(list, &cfg, None);
            let img = Image::create_from_data(
                pl.width as i32,
                pl.height as i32,
                false,
                Format::RGBA8,
                &PackedByteArray::from(pl.rgba),
            )?;
            let ink = ImageTexture::create_from_image(&img)?;
            let backing = pl.backing.and_then(|b| {
                let img = Image::create_from_data(
                    pl.width as i32,
                    pl.height as i32,
                    false,
                    Format::L8,
                    &PackedByteArray::from(b),
                )?;
                ImageTexture::create_from_image(&img)
            });
            self.cache.insert(key, Cached { ink, backing, origin: pl.origin });
        }
        self.cache.get(&key).map(|c| (key, c))
    }

    /// Make a printed art node under `parent` (anchor at `pos`).
    pub fn art(&mut self, parent: &mut Gd<Node>, pos: V2, list: &DrawList) -> Art {
        let mut node = Node2D::new_alloc();
        node.set_position(vec2(pos));
        parent.add_child(&node);
        let mut sprite = Sprite2D::new_alloc();
        sprite.set_centered(false);
        sprite.set_texture_filter(TextureFilter::LINEAR);
        sprite.set_material(&self.ink_mat);
        node.add_child(&sprite);
        let mut art = Art { node, sprite, backing: None, key: 0, extra_scale: 1.0 };
        art.set(self, list);
        art
    }

    /// A text label printed in key ink with a pink misregistered shadow.
    pub fn text(&self, parent: &mut Gd<Node>, t: &TextSpec) -> Gd<Label> {
        let mut l = Label::new_alloc();
        l.set_text(t.text.as_str());
        l.add_theme_font_override("font", &if t.bold { self.bold.clone() } else { self.font.clone() });
        l.add_theme_font_size_override("font_size", t.size as i32);
        l.add_theme_color_override("font_color", self.ink(t.ink));
        let mut sh = self.ink(Ink::Pink);
        sh.a = if t.shadow { 0.85 } else { 0.0 };
        l.add_theme_color_override("font_shadow_color", sh);
        l.add_theme_constant_override("shadow_offset_x", 2);
        l.add_theme_constant_override("shadow_offset_y", 2);
        l.add_theme_constant_override("line_spacing", -4);
        l.set_material(&self.text_mat);
        l.set_horizontal_alignment(t.align);
        l.set_vertical_alignment(VerticalAlignment::CENTER);
        if t.wrap {
            l.set_autowrap_mode(AutowrapMode::WORD_SMART);
        }
        l.set_position(Vector2::new(t.rect.x, t.rect.y));
        l.set_size(Vector2::new(t.rect.w, t.rect.h));
        l.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        parent.add_child(&l);
        l
    }
}

/// Text layout request (reference units).
#[derive(Clone, Debug)]
pub struct TextSpec {
    pub text: String,
    pub rect: proof_core::geom::Rect,
    pub size: f32,
    pub ink: Ink,
    pub align: HorizontalAlignment,
    pub bold: bool,
    pub wrap: bool,
    pub shadow: bool,
}

impl TextSpec {
    pub fn new(text: impl Into<String>, rect: proof_core::geom::Rect, size: f32) -> TextSpec {
        TextSpec {
            text: text.into(),
            rect,
            size,
            ink: Ink::Key,
            align: HorizontalAlignment::CENTER,
            bold: false,
            wrap: false,
            shadow: true,
        }
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn left(mut self) -> Self {
        self.align = HorizontalAlignment::LEFT;
        self
    }
    pub fn right(mut self) -> Self {
        self.align = HorizontalAlignment::RIGHT;
        self
    }
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
    pub fn ink(mut self, ink: Ink) -> Self {
        self.ink = ink;
        self
    }
    pub fn plain(mut self) -> Self {
        self.shadow = false;
        self
    }
}

/// A printed art node: anchor `Node2D` + ink sprite (+ optional paper backing).
pub struct Art {
    pub node: Gd<Node2D>,
    sprite: Gd<Sprite2D>,
    backing: Option<Gd<Sprite2D>>,
    key: u64,
    /// Rasterise at a higher resolution when the node will be scaled up.
    pub extra_scale: f32,
}

impl Art {
    /// Replace the artwork (re-rasterises only when the drawing changed).
    pub fn set(&mut self, riso: &mut Riso, list: &DrawList) {
        let extra = self.extra_scale;
        let scale = (riso.scale * extra).clamp(0.5, 4.0);
        let Some((key, c)) = riso.textures(list, extra) else {
            self.sprite.set_visible(false);
            return;
        };
        self.sprite.set_visible(true);
        if key == self.key {
            return;
        }
        self.key = key;
        let (ink, backing, origin) = (c.ink.clone(), c.backing.clone(), c.origin);
        self.sprite.set_texture(&ink);
        self.sprite.set_scale(Vector2::new(1.0 / scale, 1.0 / scale));
        self.sprite.set_position(vec2(origin));
        match (backing, &mut self.backing) {
            (Some(tex), Some(b)) => {
                b.set_texture(&tex);
                b.set_scale(Vector2::new(1.0 / scale, 1.0 / scale));
                b.set_position(vec2(origin));
                b.set_visible(true);
            }
            (Some(tex), None) => {
                let mut b = Sprite2D::new_alloc();
                b.set_centered(false);
                b.set_texture_filter(TextureFilter::LINEAR);
                b.set_material(&riso.backing_mat);
                b.set_texture(&tex);
                b.set_scale(Vector2::new(1.0 / scale, 1.0 / scale));
                b.set_position(vec2(origin));
                self.node.add_child(&b);
                self.node.move_child(&b, 0);
                self.backing = Some(b);
            }
            (None, Some(b)) => b.set_visible(false),
            (None, None) => {}
        }
    }

    pub fn pos(&self) -> V2 {
        v2of(self.node.get_position())
    }
    pub fn set_pos(&mut self, p: V2) {
        self.node.set_position(vec2(p));
    }
    pub fn set_scale(&mut self, s: f32) {
        self.node.set_scale(Vector2::new(s, s));
    }
    pub fn set_scale_xy(&mut self, sx: f32, sy: f32) {
        self.node.set_scale(Vector2::new(sx, sy));
    }
    pub fn set_visible(&mut self, v: bool) {
        self.node.set_visible(v);
    }
    pub fn set_alpha(&mut self, a: f32) {
        self.node.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, a.clamp(0.0, 1.0)));
    }
    pub fn free(self) {
        let mut n = self.node;
        if n.is_instance_valid() {
            n.queue_free();
        }
    }
    pub fn as_node(&self) -> Gd<Node> {
        self.node.clone().upcast()
    }
}

/// Draw helper: build a list with a transform in one go.
pub fn list(f: impl FnOnce(&mut DrawList)) -> DrawList {
    let mut d = DrawList::new();
    f(&mut d);
    d
}

pub fn list_at(xf: Xf, f: impl FnOnce(&mut DrawList)) -> DrawList {
    let mut d = DrawList::new();
    d.with(xf, f);
    d
}
