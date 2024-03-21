use super::{super::utils::Flatten, lerp, RenderContext};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::TILE_SIZE;
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_crews(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            frac_frame,
            shader,
            assets,
            offset,
            scale,
            ..
        } = ctx;

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_crew));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        for crew in self.game.iter_crew() {
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

            // if let Some(path) = &crew.path {
            //     context.set_stroke_style(&JsValue::from("#7f00ff"));
            //     context.set_line_width(2.);
            //     context.begin_path();
            //     let mut first = true;
            //     for node in path.iter().chain(std::iter::once(&crew.pos)) {
            //         let x = (node[0] as f64 + 0.5) * TILE_SIZE + offset[0];
            //         let y = (node[1] as f64 + 0.5) * TILE_SIZE + offset[1];
            //         if first {
            //             first = false;
            //             context.move_to(x, y);
            //         } else {
            //             context.line_to(x, y);
            //         }
            //     }
            //     context.stroke();
            // }
        }
        Ok(())
    }
}
