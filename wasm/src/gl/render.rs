use crate::AsteroidColonies;

use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render_gl(&self, gl: &GL) {
        gl.clear_color(0.0, 0.0, 0.5, 1.);
        gl.clear(GL::COLOR_BUFFER_BIT);
    }
}
