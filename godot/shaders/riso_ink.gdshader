// Proof — riso ink composite.
// Each texture channel is one ink plate's coverage (R pink, G yellow, B blue, A key),
// rasterised in Rust (proof_raster). Inks are multiplied over whatever is below, like a
// real risograph pass, with per-ink misregistration and paper grain.
shader_type canvas_item;
render_mode blend_mul, unshaded;

uniform vec3 ink0 : source_color = vec3(1.0, 0.36, 0.68);
uniform vec3 ink1 : source_color = vec3(1.0, 0.89, 0.29);
uniform vec3 ink2 : source_color = vec3(0.24, 0.56, 0.82);
uniform vec3 ink3 : source_color = vec3(0.18, 0.23, 0.42);
// Misregistration offsets in screen pixels (key stays put).
uniform vec2 off0 = vec2(1.6, -1.1);
uniform vec2 off1 = vec2(-1.2, 0.8);
uniform vec2 off2 = vec2(0.9, 1.4);
// Print-in "kick": 0 at rest, 1 when a page slams into the press.
uniform float kick = 0.0;
uniform float grain = 0.22;
uniform float starve = 0.12;
uniform sampler2D grain_tex : repeat_enable, filter_linear;

// Node modulate alpha (fades). Note: in fragment() COLOR already includes the texture,
// and our texture channels are ink plates, so read modulate in vertex() instead.
varying float fade;

void vertex() {
	fade = COLOR.a;
}

void fragment() {
	vec2 px = TEXTURE_PIXEL_SIZE;
	float k = 1.0 + kick * 9.0;
	float c0 = texture(TEXTURE, UV - off0 * k * px).r;
	float c1 = texture(TEXTURE, UV - off1 * k * px).g;
	float c2 = texture(TEXTURE, UV - off2 * k * px).b;
	float c3 = texture(TEXTURE, UV).a;
	vec4 g = texture(grain_tex, FRAGCOORD.xy / 256.0);
	float sk = 1.0 - starve * g.g;
	float gk = (1.0 - grain * g.r) * sk;
	float a = fade;
	vec3 col = mix(vec3(1.0), ink0, clamp(c0 * gk * a, 0.0, 1.0));
	col *= mix(vec3(1.0), ink1, clamp(c1 * gk * a, 0.0, 1.0));
	col *= mix(vec3(1.0), ink2, clamp(c2 * gk * a, 0.0, 1.0));
	col *= mix(vec3(1.0), ink3, clamp(c3 * (1.0 - grain * 0.4 * g.r) * sk * a, 0.0, 1.0));
	COLOR = vec4(col, 1.0);
}
