use super::{
    super::utils::{enable_buffer, Flatten},
    RenderContext,
};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::TILE_SIZE;
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_power_grid(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            assets,
            offset,
            scale,
            tile_range,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_power_grid));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                if !self.game.tiles()[[ix, iy]].power_grid {
                    continue;
                }
                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                let transform =
                    ctx.to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    transform.flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
        }

        Ok(())
    }
}
