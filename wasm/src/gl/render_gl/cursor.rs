use super::{
    super::utils::{enable_buffer, Flatten},
    RenderContext,
};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::{Pos, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};

use wasm_bindgen::JsValue;
use web_sys::{WebGlRenderingContext as GL, WebGlTexture};

impl AsteroidColonies {
    pub(super) fn render_gl_cursor(
        &self,
        gl: &GL,
        cursor: &Pos,
        ctx: &RenderContext,
        tex: &WebGlTexture,
    ) -> Result<(), JsValue> {
        let RenderContext {
            assets,
            scale,
            to_screen,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(tex));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let x = (cursor[0] as f64 + offset[0] as f64 / TILE_SIZE) as f32;
        let y = (cursor[1] as f64 + offset[1] as f64 / TILE_SIZE) as f32;
        let transform = to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            transform.flatten(),
        );

        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        Ok(())
    }
}
