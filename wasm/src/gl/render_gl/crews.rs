use super::{
    super::utils::{enable_buffer, Flatten},
    lerp,
    path::render_path,
    RenderContext,
};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::TILE_SIZE;
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_crews(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            frac_frame,
            assets,
            offset,
            scale,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.use_program(Some(&shader.program));
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_crew));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        for crew in self.game.iter_crew() {
            gl.use_program(Some(&shader.program));
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_crew));
            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                Matrix3::identity().flatten(),
            );
            enable_buffer(gl, &assets.screen_buffer, 2, shader.vertex_position);

            let [x, y] = if let Some(next) = crew.path.as_ref().and_then(|p| p.last()) {
                lerp(crew.pos, *next, *frac_frame)
            } else {
                [crew.pos[0] as f64, crew.pos[1] as f64]
            };
            let x = (x + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y + offset[1] as f64 / TILE_SIZE) as f32;
            let transform = ctx.to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x as f32, y as f32, 0.))
                * Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.))
                * Matrix4::from_scale(0.5)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            if let Some(path) = &crew.path {
                render_path(gl, ctx, path, &[1., 0., 1., 1.]);
            }
        }
        Ok(())
    }
}
