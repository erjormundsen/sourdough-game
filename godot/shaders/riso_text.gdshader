// Proof — text printed in key ink (multiply) with grain.
shader_type canvas_item;
render_mode blend_mul, unshaded;

uniform sampler2D grain_tex : repeat_enable, filter_linear;

void fragment() {
	vec4 g = texture(grain_tex, FRAGCOORD.xy / 256.0);
	float cov = COLOR.a * (1.0 - 0.18 * g.r);
	COLOR = vec4(mix(vec3(1.0), COLOR.rgb, cov), 1.0);
}
