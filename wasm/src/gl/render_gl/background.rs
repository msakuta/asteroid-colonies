use super::{
    super::utils::{enable_buffer, Flatten},
    RenderContext,
};
use crate::{
    render::{NEIGHBOR_BITS, SPACE_BIT},
    AsteroidColonies,
};

use ::asteroid_colonies_logic::{TileState, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_background(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            assets,
            shader,
            offset,
            scale,
            tile_range,
            ..
        } = ctx;
        let back_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 8.);

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_back));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        let mut rendered_tiles = 0;

        let mut render_quarter_tile = |image: u8, x, y| -> Result<(), JsValue> {
            let srcx = ((image & NEIGHBOR_BITS) % 4) as f32;
            let srcy = ((image & NEIGHBOR_BITS) / 4) as f32;
            let bg_y = if image & SPACE_BIT != 0 { 2. } else { 1. };
            let tex_transform = back_texture_transform
                * Matrix3::from_translation(Vector2::new(0., bg_y))
                * Matrix3::from_scale(0.5);

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );

            let transform = ctx.to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_scale(0.5);
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            let tex_transform = back_texture_transform
                * Matrix3::from_scale(0.5)
                * Matrix3::from_translation(Vector2::new(srcx, srcy));

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            rendered_tiles += 1;
            Ok(())
        };

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let tile = self.game.tile_at([ix, iy]);
                let (sx, sy) = match tile.state {
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
                let transform =
                    ctx.to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    transform.flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

                if tile.image_idx.lt & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.lt, x, y)?;
                }
                if tile.image_idx.lb & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.lb, x, y + 0.5)?;
                }
                if tile.image_idx.rb & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.rb, x + 0.5, y + 0.5)?;
                }
                if tile.image_idx.rt & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.rt, x + 0.5, y)?;
                }
            }
        }

        // console_log!("rendered_tiles: {}", rendered_tiles);

        Ok(())
    }
}
