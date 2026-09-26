// Proof — the sheet of paper everything is printed on.
shader_type canvas_item;

uniform vec3 paper : source_color = vec3(0.97, 0.95, 0.9);
uniform sampler2D grain_tex : repeat_enable, filter_linear;

void fragment() {
	vec4 g = texture(grain_tex, FRAGCOORD.xy / 256.0);
	vec3 col = paper * (1.0 - 0.035 * g.r - 0.03 * g.g);
	col.b -= 0.004;
	// Soft fibres.
	col *= 1.0 - 0.02 * g.b;
	COLOR = vec4(col, 1.0);
}
