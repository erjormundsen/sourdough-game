// Proof — opaque paper backing for cards and tickets (normal blend). The texture's red channel is
// the backing mask; the paper is the same sheet as the page (same tooth, fibres, formation).
shader_type canvas_item;

#include "res://shaders/riso_press.gdshaderinc"

varying float fade;
varying highp vec2 sp;

void vertex() {
	fade = COLOR.a;
	sp = press_screen_pos(CANVAS_MATRIX, MODEL_MATRIX, VERTEX);
}

void fragment() {
	float a = texture(TEXTURE, UV).r * fade;
	vec3 col = paper_color(texture(fine_tex, sp / fine_size), texture(coarse_tex, sp / coarse_px));
	COLOR = vec4(col, a);
}
