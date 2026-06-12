//! WebGL2 backend for the pixel UI: one shader, one vertex buffer, one
//! draw call. SDF math for rounded rects / borders / glows lives in the
//! fragment shader.

use wasm_bindgen::JsCast;
use web_sys::WebGl2RenderingContext as GL;
use web_sys::{HtmlCanvasElement, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture, WebGlUniformLocation};

use super::atlas::{Atlas, ATLAS_SIZE};
use super::draw::{DrawList, FLOATS_PER_VERT};
use super::theme;

const VERT_SRC: &str = r#"#version 300 es
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_color;
layout(location=3) in vec4 a_rect;
layout(location=4) in vec4 a_param;
uniform vec2 u_res;
out vec2 v_uv;
out vec4 v_color;
out vec4 v_rect;
out vec4 v_param;
out vec2 v_pos;
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

const FRAG_SRC: &str = r#"#version 300 es
precision highp float;
in vec2 v_uv;
in vec4 v_color;
in vec4 v_rect;
in vec4 v_param;
in vec2 v_pos;
uniform sampler2D u_tex;
out vec4 o_color;

float sdBox(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    float mode = v_param.w;
    if (mode < 0.5) {
        // glyph
        float a = texture(u_tex, v_uv).r;
        o_color = vec4(v_color.rgb, v_color.a * a);
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
        o_color = vec4(v_color.rgb, v_color.a * alpha);
    } else if (mode < 2.5) {
        // solid
        o_color = v_color;
    } else {
        // scanlines: darken every other 2px band
        float band = mod(v_pos.y, 4.0);
        float a = band < 2.0 ? v_color.a : 0.0;
        o_color = vec4(0.0, 0.0, 0.0, a);
    }
}
"#;

pub struct Renderer {
    gl: GL,
    canvas: HtmlCanvasElement,
    _program: WebGlProgram,
    vbo: WebGlBuffer,
    tex: WebGlTexture,
    u_res: WebGlUniformLocation,
    pub atlas: Atlas,
    pub dl: DrawList,
}

impl Renderer {
    pub fn new(canvas_id: &str) -> Result<Self, String> {
        let document = web_sys::window()
            .ok_or("no window")?
            .document()
            .ok_or("no document")?;
        let canvas: HtmlCanvasElement = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| format!("canvas #{} not found", canvas_id))?
            .dyn_into()
            .map_err(|_| "element is not a canvas".to_string())?;
        let gl: GL = canvas
            .get_context("webgl2")
            .map_err(|e| format!("{:?}", e))?
            .ok_or("WebGL2 unavailable")?
            .dyn_into()
            .map_err(|_| "WebGL2 context cast failed".to_string())?;

        let program = link(&gl, VERT_SRC, FRAG_SRC)?;
        gl.use_program(Some(&program));

        let vao = gl.create_vertex_array().ok_or("vao")?;
        gl.bind_vertex_array(Some(&vao));
        let vbo = gl.create_buffer().ok_or("vbo")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        let stride = (FLOATS_PER_VERT * 4) as i32;
        let offsets = [(0u32, 2i32, 0i32), (1, 2, 8), (2, 4, 16), (3, 4, 32), (4, 4, 48)];
        for (loc, size, off) in offsets {
            gl.vertex_attrib_pointer_with_i32(loc, size, GL::FLOAT, false, stride, off);
            gl.enable_vertex_attrib_array(loc);
        }

        let tex = gl.create_texture().ok_or("texture")?;
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);

        let u_res = gl.get_uniform_location(&program, "u_res").ok_or("u_res")?;
        if let Some(u_tex) = gl.get_uniform_location(&program, "u_tex") {
            gl.uniform1i(Some(&u_tex), 0);
        }
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        Ok(Renderer {
            gl,
            canvas,
            _program: program,
            vbo,
            tex,
            u_res,
            atlas: Atlas::new()?,
            dl: DrawList::new(),
        })
    }

    pub fn size(&self) -> (f32, f32) {
        (self.canvas.width() as f32, self.canvas.height() as f32)
    }

    pub fn flush(&mut self) {
        let (w, h) = self.size();
        let gl = &self.gl;
        gl.viewport(0, 0, w as i32, h as i32);
        let bg = theme::BG0;
        gl.clear_color(bg[0], bg[1], bg[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);

        if self.atlas.dirty {
            gl.bind_texture(GL::TEXTURE_2D, Some(&self.tex));
            gl.pixel_storei(GL::UNPACK_ALIGNMENT, 1);
            let _ = gl
                .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                    GL::TEXTURE_2D,
                    0,
                    GL::R8 as i32,
                    ATLAS_SIZE as i32,
                    ATLAS_SIZE as i32,
                    0,
                    GL::RED,
                    GL::UNSIGNED_BYTE,
                    Some(&self.atlas.pixels),
                );
            self.atlas.dirty = false;
        }

        gl.uniform2f(Some(&self.u_res), w, h);
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        // SAFETY: no allocation between view creation and the copying call.
        unsafe {
            let view = js_sys::Float32Array::view(&self.dl.verts);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.draw_arrays(GL::TRIANGLES, 0, (self.dl.verts.len() / FLOATS_PER_VERT) as i32);
    }
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, String> {
    let s = gl.create_shader(kind).ok_or("create_shader")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(s)
    } else {
        Err(gl.get_shader_info_log(&s).unwrap_or_else(|| "compile failed".into()))
    }
}

fn link(gl: &GL, v: &str, f: &str) -> Result<WebGlProgram, String> {
    let vs = compile(gl, GL::VERTEX_SHADER, v)?;
    let fs = compile(gl, GL::FRAGMENT_SHADER, f)?;
    let p = gl.create_program().ok_or("create_program")?;
    gl.attach_shader(&p, &vs);
    gl.attach_shader(&p, &fs);
    gl.link_program(&p);
    if gl.get_program_parameter(&p, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(p)
    } else {
        Err(gl.get_program_info_log(&p).unwrap_or_else(|| "link failed".into()))
    }
}
