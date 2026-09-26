// Proof — opaque paper backing for cards and tickets (normal blend).
// The texture's red channel is the backing mask.
shader_type canvas_item;

uniform vec3 paper : source_color = vec3(0.97, 0.95, 0.9);
uniform sampler2D grain_tex : repeat_enable, filter_linear;

varying float fade;

void vertex() {
	fade = COLOR.a;
}

void fragment() {
	float a = texture(TEXTURE, UV).r * fade;
	vec4 g = texture(grain_tex, FRAGCOORD.xy / 256.0);
	vec3 col = paper * (1.0 - 0.035 * g.r - 0.03 * g.g);
	COLOR = vec4(col, a);
}
