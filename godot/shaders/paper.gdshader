// Proof — the sheet of uncoated paper everything is printed on: warm stock with tooth, fibres and
// cloudy formation (see riso_press.gdshaderinc).
shader_type canvas_item;

#include "res://shaders/riso_press.gdshaderinc"

varying highp vec2 sp;

void vertex() {
	sp = press_screen_pos(CANVAS_MATRIX, MODEL_MATRIX, VERTEX);
}

void fragment() {
	COLOR = vec4(paper_color(texture(fine_tex, sp / fine_size), texture(coarse_tex, sp / coarse_px)), 1.0);
}
