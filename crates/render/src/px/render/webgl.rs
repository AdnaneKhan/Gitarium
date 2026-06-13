//! WebGL backend: one shader, one vertex buffer, one draw call. SDF math
//! for rounded rects / borders / glows lives in the fragment shader. The
//! code sticks to the WebGL1 feature subset (GLSL ES 1.00, LUMINANCE
//! atlas, no VAOs) so a single path runs on a WebGL2 context when
//! available and falls back to WebGL1 when it isn't.

use wasm_bindgen::JsCast;
use web_sys::WebGlRenderingContext as GL;
use web_sys::{HtmlCanvasElement, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture, WebGlUniformLocation};

use super::super::atlas::{Atlas, ATLAS_SIZE, COLOR_ATLAS};
use super::super::draw::{DrawList, FLOATS_PER_VERT};
use super::super::theme;
use super::shaders::{FRAG_SRC, VERT_SRC};

/// Vertex attribute names, in the index order the pointer setup uses.
/// GLSL 100 has no layout(location=); the indices are pinned at link time.
const ATTRIBS: [&str; 5] = ["a_pos", "a_uv", "a_color", "a_rect", "a_param"];

pub(super) struct State {
    gl: GL,
    _program: WebGlProgram,
    vbo: WebGlBuffer,
    tex: WebGlTexture,
    emoji_tex: WebGlTexture,
    u_res: WebGlUniformLocation,
}

/// Acquire a context, preferring WebGL2 and falling back to WebGL1. Both
/// kinds are driven through the WebGL1 bindings: web-sys methods are
/// structural (dispatched by name on the receiver), and every call this
/// backend makes exists with identical name, signature, and semantics on
/// both context types, so a WebGL2 context works through the WebGL1
/// interface directly — hence the unchecked cast.
fn acquire_context(canvas: &HtmlCanvasElement) -> Result<GL, String> {
    for kind in ["webgl2", "webgl", "experimental-webgl"] {
        if let Ok(Some(ctx)) = canvas.get_context(kind) {
            return Ok(ctx.unchecked_into::<GL>());
        }
    }
    Err("WebGL unavailable (tried webgl2, webgl, experimental-webgl)".into())
}

pub(super) fn init(canvas: &HtmlCanvasElement) -> Result<State, String> {
    let gl = acquire_context(canvas)?;

    let program = link(&gl, VERT_SRC, FRAG_SRC)?;
    gl.use_program(Some(&program));

    // Attribute state lives in the context's default vertex array; with
    // one program and one VBO bound for the renderer's lifetime, the
    // pointers set once here stay valid (no VAO needed — WebGL1 has none).
    let vbo = gl.create_buffer().ok_or("vbo")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
    let stride = (FLOATS_PER_VERT * 4) as i32;
    let offsets = [(0u32, 2i32, 0i32), (1, 2, 8), (2, 4, 16), (3, 4, 32), (4, 4, 48)];
    for (loc, size, off) in offsets {
        gl.vertex_attrib_pointer_with_i32(loc, size, GL::FLOAT, false, stride, off);
        gl.enable_vertex_attrib_array(loc);
    }

    let mk_tex = |unit: u32| -> Result<WebGlTexture, String> {
        let t = gl.create_texture().ok_or("texture")?;
        gl.active_texture(unit);
        gl.bind_texture(GL::TEXTURE_2D, Some(&t));
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        Ok(t)
    };
    // Unit 0: coverage glyph atlas. Unit 1: RGBA color-emoji atlas.
    let emoji_tex = mk_tex(GL::TEXTURE1)?;
    let tex = mk_tex(GL::TEXTURE0)?;

    let u_res = gl.get_uniform_location(&program, "u_res").ok_or("u_res")?;
    if let Some(u_tex) = gl.get_uniform_location(&program, "u_tex") {
        gl.uniform1i(Some(&u_tex), 0);
    }
    if let Some(u_emoji) = gl.get_uniform_location(&program, "u_emoji") {
        gl.uniform1i(Some(&u_emoji), 1);
    }
    gl.enable(GL::BLEND);
    gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

    Ok(State { gl, _program: program, vbo, tex, emoji_tex, u_res })
}

impl State {
    pub(super) fn flush(&mut self, atlas: &mut Atlas, dl: &DrawList, w: f32, h: f32) {
        let gl = &self.gl;
        gl.viewport(0, 0, w as i32, h as i32);
        let bg = theme::BG0;
        gl.clear_color(bg[0], bg[1], bg[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);

        if atlas.dirty {
            gl.bind_texture(GL::TEXTURE_2D, Some(&self.tex));
            gl.pixel_storei(GL::UNPACK_ALIGNMENT, 1);
            // LUMINANCE rather than R8/RED: valid on WebGL1 and WebGL2
            // alike, and samples as (L, L, L, 1) so the shader's .r read
            // sees the same coverage value.
            let _ = gl
                .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                    GL::TEXTURE_2D,
                    0,
                    GL::LUMINANCE as i32,
                    ATLAS_SIZE as i32,
                    ATLAS_SIZE as i32,
                    0,
                    GL::LUMINANCE,
                    GL::UNSIGNED_BYTE,
                    Some(&atlas.pixels),
                );
            atlas.dirty = false;
        }

        if atlas.color_dirty {
            gl.active_texture(GL::TEXTURE1);
            gl.bind_texture(GL::TEXTURE_2D, Some(&self.emoji_tex));
            gl.pixel_storei(GL::UNPACK_ALIGNMENT, 1);
            let _ = gl
                .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                    GL::TEXTURE_2D,
                    0,
                    GL::RGBA as i32,
                    COLOR_ATLAS as i32,
                    COLOR_ATLAS as i32,
                    0,
                    GL::RGBA,
                    GL::UNSIGNED_BYTE,
                    Some(&atlas.color_pixels),
                );
            gl.active_texture(GL::TEXTURE0);
            atlas.color_dirty = false;
        }

        gl.uniform2f(Some(&self.u_res), w, h);
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        // SAFETY: no allocation between view creation and the copying call.
        unsafe {
            let view = js_sys::Float32Array::view(&dl.verts);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
        }
        gl.draw_arrays(GL::TRIANGLES, 0, (dl.verts.len() / FLOATS_PER_VERT) as i32);
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
    // Pin attribute indices to the order the vertex pointers use; must
    // happen before linking to take effect.
    for (i, name) in ATTRIBS.iter().enumerate() {
        gl.bind_attrib_location(&p, i as u32, name);
    }
    gl.link_program(&p);
    if gl.get_program_parameter(&p, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(p)
    } else {
        Err(gl.get_program_info_log(&p).unwrap_or_else(|| "link failed".into()))
    }
}
