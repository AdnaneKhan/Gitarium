//! GLSL sources for the single SDF/glyph/solid pipeline. Written in GLSL
//! ES 1.00 so one shader pair drives both WebGL2 and fallback WebGL1
//! contexts; attribute locations are pinned at link time (no layout()).

pub(super) const VERT_SRC: &str = r#"#version 100
attribute vec2 a_pos;
attribute vec2 a_uv;
attribute vec4 a_color;
attribute vec4 a_rect;
attribute vec4 a_param;
uniform vec2 u_res;
varying vec2 v_uv;
varying vec4 v_color;
varying vec4 v_rect;
varying vec4 v_param;
varying vec2 v_pos;
void main() {
    vec2 clip = vec2(a_pos.x * 2.0 / u_res.x - 1.0, 1.0 - a_pos.y * 2.0 / u_res.y);
    gl_Position = vec4(clip, 0.0, 1.0);
    v_uv = a_uv;
    v_color = a_color;
    v_rect = a_rect;
    v_param = a_param;
    v_pos = a_pos;
}
"#;

pub(super) const FRAG_SRC: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 v_uv;
varying vec4 v_color;
varying vec4 v_rect;
varying vec4 v_param;
varying vec2 v_pos;
uniform sampler2D u_tex;

float sdBox(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    float mode = v_param.w;
    if (mode < 0.5) {
        // glyph (atlas is LUMINANCE: .r carries the coverage)
        float a = texture2D(u_tex, v_uv).r;
        gl_FragColor = vec4(v_color.rgb, v_color.a * a);
    } else if (mode < 1.5) {
        // SDF rounded rect: param = radius, feather, border_width
        float d = sdBox(v_pos - v_rect.xy, v_rect.zw, v_param.x);
        float f = v_param.y;
        float outer = 1.0 - smoothstep(-f, f, d);
        float alpha = outer;
        if (v_param.z > 0.0) {
            float inner = 1.0 - smoothstep(-f, f, d + v_param.z);
            alpha = clamp(outer - inner, 0.0, 1.0);
        }
        gl_FragColor = vec4(v_color.rgb, v_color.a * alpha);
    } else if (mode < 2.5) {
        // solid
        gl_FragColor = v_color;
    } else {
        // scanlines: darken every other 2px band
        float band = mod(v_pos.y, 4.0);
        float a = band < 2.0 ? v_color.a : 0.0;
        gl_FragColor = vec4(0.0, 0.0, 0.0, a);
    }
}
"#;
