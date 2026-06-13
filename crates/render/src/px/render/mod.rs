//! Rendering backends for the pixel UI. The view layer emits one quad per
//! primitive into the DrawList — positions pre-clipped, and the full SDF
//! parameters (mode, rect, radius, feather, border width) ride on every
//! vertex — so the stream decodes losslessly back into primitives. WebGL
//! (2 preferred, 1 as fallback) draws it in a single call; machines with
//! no WebGL at all get a Canvas2D interpreter of the same stream.

mod canvas2d;
mod glyphs;
mod shaders;
mod webgl;

use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use super::atlas::Atlas;
use super::draw::DrawList;

enum Backend {
    WebGl(webgl::State),
    Canvas(canvas2d::State),
}

pub struct Renderer {
    backend: Backend,
    canvas: HtmlCanvasElement,
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
        let backend = match webgl::init(&canvas) {
            Ok(s) => Backend::WebGl(s),
            // No usable WebGL (no GPU, software GL disabled): software-
            // render the same quad stream through Canvas2D.
            Err(gl_err) => match canvas2d::init(&canvas) {
                Ok(s) => Backend::Canvas(s),
                Err(e) => return Err(format!("{}; canvas2d: {}", gl_err, e)),
            },
        };
        Ok(Renderer {
            backend,
            canvas,
            atlas: Atlas::new()?,
            dl: DrawList::new(),
        })
    }

    pub fn size(&self) -> (f32, f32) {
        (self.canvas.width() as f32, self.canvas.height() as f32)
    }

    pub fn flush(&mut self) {
        let (w, h) = self.size();
        match &mut self.backend {
            Backend::WebGl(s) => s.flush(&mut self.atlas, &self.dl, w, h),
            Backend::Canvas(s) => s.flush(&mut self.atlas, &self.dl, w, h),
        }
    }
}
