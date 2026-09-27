// Proof — riso ink: prints one sprite's four plates (R pink, G yellow, B blue, A key), each a
// screened 1-bit master rasterised in Rust (proof_raster), multiplied over whatever is below.
//
// Every drum is misregistered by the same page-space affine offset for every sprite on screen
// (translation + a hair of rotation/stretch about the page centre); the print-in kick slams them
// further out. Screen-space offsets become UV offsets through the UV derivatives, so node scale
// and rotation are handled. Budget: 7 texture reads, early-out on empty pixels.
shader_type canvas_item;
render_mode blend_mul, unshaded;

#include "res://shaders/riso_press.gdshaderinc"

uniform vec3 ink0 : source_color = vec3(1.0, 0.36, 0.68);
uniform vec3 ink1 : source_color = vec3(1.0, 0.89, 0.29);
uniform vec3 ink2 : source_color = vec3(0.24, 0.56, 0.82);
uniform vec3 ink3 : source_color = vec3(0.18, 0.23, 0.42);
// Drum registration: shift in px and the linear part [a11, a12, a21, a22] about the page centre.
uniform vec2 shift0 = vec2(1.7, 1.2);
uniform vec4 lin0 = vec4(0.0);
uniform vec2 shift1 = vec2(-1.4, 0.8);
uniform vec4 lin1 = vec4(0.0);
uniform vec2 shift2 = vec2(0.7, -1.5);
uniform vec4 lin2 = vec4(0.0);
uniform vec2 shift3 = vec2(0.0);
uniform vec4 lin3 = vec4(0.0);
// Ink-spread warp amplitude in px.
uniform float warp_px = 0.45;

// Node modulate alpha (fades). In fragment() COLOR already includes the texture, and our
// texture channels are ink plates, so read modulate in vertex() instead.
varying float fade;
varying highp vec2 sp;

void vertex() {
	fade = COLOR.a;
	sp = press_screen_pos(CANVAS_MATRIX, MODEL_MATRIX, VERTEX);
}

vec2 drum(vec2 d, vec2 shift, vec4 a) {
	return shift + vec2(a.x * d.x + a.y * d.y, a.z * d.x + a.w * d.y);
}

void fragment() {
	highp vec2 screen = 1.0 / SCREEN_PIXEL_SIZE;
	vec4 f = texture(fine_tex, sp / fine_size);
	vec4 m = texture(mid_tex, sp / mid_size);
	highp vec2 d = sp - 0.5 * screen;
	highp vec2 w = (m.rg - 0.5) * (2.0 * warp_px);
	// UV per screen pixel along x and down the page (highp: offsets are fractions of a texel).
	highp vec2 ux = dFdx(UV);
	highp vec2 uy = dFdy(UV) * sign(dFdy(sp.y));
	highp vec2 o0 = w - drum(d, shift0, lin0);
	highp vec2 o1 = vec2(-w.y, w.x) - drum(d, shift1, lin1);
	highp vec2 o2 = vec2(w.y, -w.x) - drum(d, shift2, lin2);
	highp vec2 o3 = w * -0.6 - drum(d, shift3, lin3);
	vec4 v = vec4(
		texture(TEXTURE, UV + ux * o0.x + uy * o0.y).r,
		texture(TEXTURE, UV + ux * o1.x + uy * o1.y).g,
		texture(TEXTURE, UV + ux * o2.x + uy * o2.y).b,
		texture(TEXTURE, UV + ux * o3.x + uy * o3.y).a);
	if (v.x + v.y + v.z + v.w < 0.002) {
		COLOR = vec4(1.0);
	} else {
		vec4 c = texture(coarse_tex, sp / coarse_px);
		vec4 cov = press_ink(v, f, m, c, sp / screen) * fade;
		vec3 col = mix(vec3(1.0), ink0, cov.x) * mix(vec3(1.0), ink1, cov.y)
			* mix(vec3(1.0), ink2, cov.z) * mix(vec3(1.0), ink3, cov.w);
		COLOR = vec4(col, 1.0);
	}
}
