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
            offset,
            scale,
            tile_range,
            ..
        } = ctx;
        let back_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 8.);
        let mt_shader = &assets.multi_textured_shader;

        gl.use_program(Some(&mt_shader.program));
        gl.uniform1f(mt_shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(mt_shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_back));

        let size = 128;

        self.render_bg_sampler(size, gl, ctx)?;
        self.render_bg_modulate(size, gl, ctx)?;

        let shader = &assets.textured_shader;

        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        let total_pixels = size as f32 * TILE_SIZE as f32;
        let bg_scale = 1. / self.viewport.scale as f32 / total_pixels;

        let tex_transform = Matrix3::from_translation(Vector2::new(
            -1. * offset[0] as f32 / total_pixels,
            -1. * offset[1] as f32 / total_pixels,
        )) * Matrix3::from_nonuniform_scale(
            self.viewport.size[0] as f32 * bg_scale,
            self.viewport.size[1] as f32 * bg_scale,
        );

        gl.uniform_matrix3fv_with_f32_array(
            mt_shader.tex_transform_loc.as_ref(),
            false,
            tex_transform.flatten(),
        );

        gl.uniform_matrix4fv_with_f32_array(
            mt_shader.transform_loc.as_ref(),
            false,
            ctx.to_screen.flatten(),
            // Matrix4::identity().flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

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

        // Render the background in one polygon
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_back));

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                let tile = self.game.tile_at([ix, iy]);
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

    fn render_bg_sampler(&self, size: usize, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext { assets, .. } = ctx;
        let mt_shader = &assets.multi_textured_shader;

        let mut buf = vec![0u8; size * size];
        for iy in 0..size {
            for ix in 0..size {
                let tile = &self.game.tiles()[[ix as i32, iy as i32]];
                buf[ix + iy * size] = match tile.state {
                    TileState::Solid => 0,
                    TileState::Empty => 127,
                    _ => 255,
                };
            }
        }

        let existing_buf = assets.bg_sampler_buf.take();

        gl.active_texture(GL::TEXTURE1);

        gl.uniform1i(mt_shader.texture2_loc.as_ref(), 1);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_bg_sampler));
        if buf != existing_buf {
            // tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array
            // let internal_format = GL::RGBA as i32;
            let format = GL::LUMINANCE;
            let type_ = GL::UNSIGNED_BYTE;
            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                0,
                0,
                size as i32,
                size as i32,
                format,
                type_,
                Some(&buf),
            )?;
        }
        assets.bg_sampler_buf.set(buf);
        Ok(())
    }

    fn render_bg_modulate(&self, size: usize, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext { assets, .. } = ctx;
        let mt_shader = &assets.multi_textured_shader;

        let enabled = self.draw_ore_overlay;

        gl.uniform1i(
            mt_shader.locations.draw_ore_overlay.as_ref(),
            enabled as i32,
        );
        if !enabled {
            return Ok(());
        }

        let mut buf = vec![0u8; 3 * size * size];
        for iy in 0..size {
            for ix in 0..size {
                let tile = &self.game.tiles()[[ix as i32, iy as i32]];
                let start = (ix + iy * size) * 3;
                buf[start..start + 3].copy_from_slice(&match tile.state {
                    TileState::Solid => [
                        (tile.ores.copper * 63. + 191.) as u8,
                        (tile.ores.lithium * 63. + 191.) as u8,
                        (tile.ores.iron * 63. + 191.) as u8,
                    ],
                    _ => [255; 3],
                });
            }
        }

        let existing_buf = assets.bg_modulate_buf.take();

        gl.active_texture(GL::TEXTURE2);

        gl.uniform1i(mt_shader.locations.texture_ores.as_ref(), 2);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_bg_modulate));
        if buf != existing_buf {
            // tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array
            // let internal_format = GL::RGBA as i32;
            let format = GL::RGB;
            let type_ = GL::UNSIGNED_BYTE;
            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                GL::TEXTURE_2D,
                0,
                0,
                0,
                size as i32,
                size as i32,
                format,
                type_,
                Some(&buf),
            )?;
        }
        assets.bg_modulate_buf.set(buf);
        Ok(())
    }
}
