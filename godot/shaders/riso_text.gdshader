// Proof — text printed in riso ink (multiply): glyph coverage goes through the same ink film as
// the key drum (mottling, tooth, pinholes, ragged edges), so labels print like the art.
shader_type canvas_item;
render_mode blend_mul, unshaded;

#include "res://shaders/riso_press.gdshaderinc"

varying highp vec2 sp;

void vertex() {
	sp = press_screen_pos(CANVAS_MATRIX, MODEL_MATRIX, VERTEX);
}

void fragment() {
	vec2 screen = 1.0 / SCREEN_PIXEL_SIZE;
	vec4 f = texture(fine_tex, sp / fine_size);
	vec4 m = texture(mid_tex, sp / mid_size);
	vec4 c = texture(coarse_tex, sp / coarse_px);
	float cov = press_ink(vec4(0.0, 0.0, 0.0, COLOR.a), f, m, c, sp / screen).w;
	COLOR = vec4(mix(vec3(1.0), COLOR.rgb, cov), 1.0);
}
