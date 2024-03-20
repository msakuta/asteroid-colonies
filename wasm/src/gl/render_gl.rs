use crate::gl::utils::enable_buffer;
use crate::gl::utils::Flatten;
use crate::js_str;
use crate::AsteroidColonies;

use crate::console_log;
use asteroid_colonies_logic::Tile;
use asteroid_colonies_logic::TileState;
use asteroid_colonies_logic::TILE_SIZE;
use cgmath::Matrix3;
use cgmath::Matrix4;
use cgmath::SquareMatrix;
use cgmath::Vector2;
use cgmath::Vector3;
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render_gl(&self, gl: &GL) -> Result<(), JsValue> {
        gl.clear_color(0.0, 0.0, 0.5, 1.);
        gl.clear(GL::COLOR_BUFFER_BIT);

        let Some(assets) = self.gl_assets.as_ref() else {
            console_log!("Warning: gl_assets are not initialized!");
            return Err(js_str!("gl_assets are not initialized"));
        };

        gl.enable(GL::BLEND);
        gl.disable(GL::DEPTH_TEST);

        let back_texture_transform =
        // (Matrix3::from_translation(Vector2::new(
        //     -self.viewport.x,
        //     self.viewport_height / self.viewport.scale / TILE_SIZE - self.viewport.y,
        // )) * Matrix3::from_nonuniform_scale(
        //     self.viewport_width / self.viewport.scale,
        //     self.viewport_height / self.viewport.scale,
        // ) * Matrix3::from_nonuniform_scale(1. / TILE_SIZE, -1. / TILE_SIZE))
        //  * Matrix3::from_translation(Vector2::new(-2. * self.viewport.x * self.viewport.scale / TILE_SIZE, 2. * self.viewport.y * self.viewport.scale / TILE_SIZE)))
        Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 8.)
        .cast::<f32>()
        .ok_or_else(|| js_str!("world transform cast failed"))?;

        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_back));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let scale = self.viewport.size[0].min(self.viewport.size[1]) as f32;
        let scale_x = 2. * TILE_SIZE as f32 / (self.viewport.size[0] as f32);
        let scale_y = 2. * TILE_SIZE as f32 / (self.viewport.size[1] as f32);

        let ymin = ((-offset[1] - vp.size[1] / 2.).div_euclid(TILE_SIZE)) as i32;
        let ymax = (-offset[1] + vp.size[1] / 2. + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        let xmin = ((-offset[0] - vp.size[0] / 2.).div_euclid(TILE_SIZE)) as i32;
        let xmax = (-offset[0] + vp.size[0] / 2. + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let (sx, sy) = match self.game.tiles()[[ix, iy]].state {
                    TileState::Empty => (0., 1.),
                    TileState::Solid => (0., 0.),
                    TileState::Space => (0., 2.),
                };
                let tex_transform = back_texture_transform
                    * Matrix3::from_translation(Vector2::new(sx as f32, sy as f32));

                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    tex_transform.flatten(),
                );

                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    (Matrix4::from_nonuniform_scale(scale_x, -scale_y, 1.)
                        * Matrix4::from_translation(Vector3::new(x, y, 0.)))
                    .flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
        }

        Ok(())
    }
}
