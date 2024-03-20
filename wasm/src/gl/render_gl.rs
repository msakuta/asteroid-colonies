use crate::{
    console_log,
    gl::utils::{enable_buffer, Flatten},
    js_str, AsteroidColonies,
};

use asteroid_colonies_logic::{Conveyor, Direction, Pos, TileState, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector2, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

use super::assets::Assets;

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

        let to_screen = Matrix4::from_nonuniform_scale(2., -2., 1.)
            * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));

        self.render_gl_background(gl, assets, &to_screen)?;
        self.render_gl_power_grid(gl, assets, &to_screen)?;
        self.render_gl_conveyor(gl, assets, &to_screen)?;

        if let Some(cursor) = self.cursor {
            self.render_gl_cursor(gl, assets, &cursor, &to_screen)?;
        }

        Ok(())
    }

    fn render_tile_range(&self) -> [i32; 4] {
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let ymin = ((-offset[1]).div_euclid(TILE_SIZE)) as i32;
        let ymax = (-offset[1] + vp.size[1] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        let xmin = ((-offset[0]).div_euclid(TILE_SIZE)) as i32;
        let xmax = (-offset[0] + vp.size[0] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        [xmin, xmax, ymin, ymax]
    }

    fn render_gl_background(
        &self,
        gl: &GL,
        assets: &Assets,
        to_screen: &Matrix4<f32>,
    ) -> Result<(), JsValue> {
        let back_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 8.);

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
        let scale_x = TILE_SIZE as f32 / (self.viewport.size[0] as f32);
        let scale_y = TILE_SIZE as f32 / (self.viewport.size[1] as f32);

        let [xmin, xmax, ymin, ymax] = self.render_tile_range();
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
                let transform = to_screen
                    * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.)
                    * Matrix4::from_translation(Vector3::new(x, y, 0.));
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

    fn render_gl_power_grid(
        &self,
        gl: &GL,
        assets: &Assets,
        to_screen: &Matrix4<f32>,
    ) -> Result<(), JsValue> {
        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_power_grid));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let scale_x = TILE_SIZE as f32 / (self.viewport.size[0] as f32);
        let scale_y = TILE_SIZE as f32 / (self.viewport.size[1] as f32);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let [xmin, xmax, ymin, ymax] = self.render_tile_range();
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                if !self.game.tiles()[[ix, iy]].power_grid {
                    continue;
                }
                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                let transform = to_screen
                    * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.)
                    * Matrix4::from_translation(Vector3::new(x, y, 0.));
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

    fn render_gl_conveyor(
        &self,
        gl: &GL,
        assets: &Assets,
        to_screen: &Matrix4<f32>,
    ) -> Result<(), JsValue> {
        let conveyor_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 5.);

        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_conveyor));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let scale_x = TILE_SIZE as f32 / (self.viewport.size[0] as f32);
        let scale_y = TILE_SIZE as f32 / (self.viewport.size[1] as f32);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let set_texture_transform = |sx, sy| {
            let tex_transform = conveyor_texture_transform
                * Matrix3::from_translation(Vector2::new(sx as f32, sy as f32));

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );
        };

        let render_tile = |x, y| {
            let x = (x as f64 + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y as f64 + offset[1] as f64 / TILE_SIZE) as f32;
            let transform = to_screen
                * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        let render_conveyor_layer = |x, y, conv: (Direction, Direction)| {
            let (sx, sy) = match conv {
                (from, to) => {
                    let mut sy = match to {
                        Direction::Left => 0.,
                        Direction::Up => 1.,
                        Direction::Right => 2.,
                        Direction::Down => 3.,
                    };
                    let sx = match from {
                        Direction::Left => 0.,
                        Direction::Up => 1.,
                        Direction::Right => 2.,
                        Direction::Down => 3.,
                    };
                    if sx <= sy {
                        sy -= 1.;
                    }
                    (sx, sy)
                }
            };
            set_texture_transform(sx, sy);
            render_tile(x, y);
        };

        let render_conveyor = |x, y, conv: Conveyor| -> Result<(), JsValue> {
            match conv {
                Conveyor::One(from, to) => render_conveyor_layer(x, y, (from, to)),
                Conveyor::Two(first, second) => {
                    render_conveyor_layer(x, y, first);
                    render_conveyor_layer(x, y, second);
                }
                Conveyor::Splitter(dir) | Conveyor::Merger(dir) => {
                    let sx = match dir {
                        Direction::Left => 0.,
                        Direction::Up => 1.,
                        Direction::Right => 2.,
                        Direction::Down => 3.,
                    };
                    let sy = match conv {
                        Conveyor::Splitter(_) => 3.,
                        _ => 4.,
                    };
                    set_texture_transform(sx, sy);
                    render_tile(x, y);
                }
                _ => {}
            };
            Ok(())
        };

        let [xmin, xmax, ymin, ymax] = self.render_tile_range();
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let conv = self.game.tiles()[[ix, iy]].conveyor;
                render_conveyor(ix, iy, conv)?;
            }
        }

        Ok(())
    }

    fn render_gl_cursor(
        &self,
        gl: &GL,
        assets: &Assets,
        cursor: &Pos,
        to_screen: &Matrix4<f32>,
    ) -> Result<(), JsValue> {
        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_cursor));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let scale_x = TILE_SIZE as f32 / (self.viewport.size[0] as f32);
        let scale_y = TILE_SIZE as f32 / (self.viewport.size[1] as f32);
        let x = (cursor[0] as f64 + offset[0] as f64 / TILE_SIZE) as f32;
        let y = (cursor[1] as f64 + offset[1] as f64 / TILE_SIZE) as f32;
        let transform = to_screen
            * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.)
            * Matrix4::from_translation(Vector3::new(x, y, 0.));

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
