// Proof — the riso press: one paper-and-ink model shared by every printed layer
// (riso_ink, riso_text, riso_backing, paper).
//
// GPU twin of rust/crates/proof_raster/src/press.rs (the artboard's CPU preview). Both read the
// same generated textures and the same numbers (PressStyle::shader_params), so keep the maths
// here and there identical.
//
// Positions are screen pixels with a top-left origin, computed in vertex() from the canvas
// transform (`press_screen_pos`), so paper, grain and drum registration line up across every
// sprite, label and backing on the page whatever the GL framebuffer orientation is.

// fine (1 texel = 1 px): R paper tooth, G paper fibres (0.5 = none), B ragged-edge noise, A pinholes.
uniform sampler2D fine_tex : repeat_enable, filter_nearest;
// mid (1 texel = 1 px): RG ink-spread warp, B ink-film noise, A feed streaks.
uniform sampler2D mid_tex : repeat_enable, filter_linear;
// coarse (1 texel = 3 reference units): RGB mottling of pink/yellow/blue, A key mottling + formation.
uniform sampler2D coarse_tex : repeat_enable, filter_linear;
uniform float fine_size = 512.0;
uniform float mid_size = 256.0;
uniform float coarse_px = 1152.0;

uniform vec3 paper : source_color = vec3(0.97, 0.95, 0.9);
uniform float tooth = 0.08;
uniform float fibres = 0.18;
uniform float formation = 0.045;

uniform float spread = 0.05;
uniform float rough = 0.34;
uniform vec4 mottle = vec4(0.13, 0.11, 0.13, 0.06);
uniform vec4 velvet = vec4(0.22, 0.18, 0.22, 0.12);
uniform vec4 pinholes = vec4(0.85, 0.8, 0.85, 0.5);
uniform vec4 streaks = vec4(0.1, 0.08, 0.1, 0.04);
uniform vec4 starve_x = vec4(0.035, -0.025, 0.03, 0.0);
uniform vec4 starve_y = vec4(0.025, 0.035, -0.025, 0.012);
uniform vec4 trap = vec4(0.12, 0.0, 0.14, 0.05);

// Screen position in px (top-left origin) of a vertex; call from vertex().
vec2 press_screen_pos(mat4 canvas, mat4 model, vec2 vertex) {
	return (canvas * model * vec4(vertex, 0.0, 1.0)).xy;
}

vec3 paper_color(vec4 f, vec4 c) {
	float form = c.a - 0.5;
	float k = 1.0 - tooth * (f.r - 0.5) + fibres * (f.g - 0.5) - formation * form;
	return vec3(paper.r * k, paper.g * k, paper.b * k * (1.0 - 0.6 * formation * form));
}

// Ink film: raw plate coverage (pink, yellow, blue, key) -> printed density.
// `u` is the pixel's position on the page, 0..1.
vec4 press_ink(vec4 v, vec4 f, vec4 m, vec4 c, vec2 u) {
	// Ink spread and ragged edges where coverage is partial (edges, screen dots).
	vec4 n = vec4(f.b - 0.5, m.b - 0.5, 0.5 - f.b, f.r - 0.5);
	vec4 edge = 4.0 * v * (1.0 - v);
	vec4 e = clamp(v + edge * (spread + rough * 2.0 * n), 0.0, 1.0);
	// Paper tooth peaks skip ink ("velvet"), pinholes, streaks along the feed, mottling,
	// starvation across the sheet, and uneven trapping over earlier inks (Y, P, B, K order).
	float peak = smoothstep(0.52, 0.86, f.r);
	vec4 prior = vec4(e.y, 0.0, min(e.x + e.y, 1.0), min(e.x + e.y + e.z, 1.0));
	float gate = smoothstep(0.3, 0.7, c.a);
	vec4 mot = smoothstep(vec4(0.32), vec4(0.86), c);
	vec4 ramp = (u.x - 0.5) * starve_x + (u.y - 0.5) * starve_y;
	vec4 body = 1.0 - mottle * mot - velvet * peak - pinholes * f.a - streaks * (m.a * gate) - ramp
		- trap * prior * m.b;
	return e * clamp(body, 0.0, 1.0);
}
