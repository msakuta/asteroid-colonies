use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector2, Vector3};
use wasm_bindgen::JsValue;
use web_sys::WebGlRenderingContext as GL;

use super::{super::utils::Flatten, enable_buffer, RenderContext};
use crate::AsteroidColonies;
use ::asteroid_colonies_logic::{Conveyor, Direction, TILE_SIZE};

impl AsteroidColonies {
    pub(super) fn render_gl_conveyor(
        &self,
        gl: &GL,
        ctx: &RenderContext,
        x: i32,
        y: i32,
        conv: Conveyor,
    ) {
        let RenderContext {
            assets,
            offset,
            scale,
            ..
        } = ctx;

        let conveyor_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 5.);

        let render_tile = |x, y| {
            let x = (x as f64 + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y as f64 + offset[1] as f64 / TILE_SIZE) as f32;
            let transform =
                ctx.to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                assets.textured_shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        let set_texture_transform = |sx, sy| {
            let tex_transform = conveyor_texture_transform
                * Matrix3::from_translation(Vector2::new(sx as f32, sy as f32));

            gl.uniform_matrix3fv_with_f32_array(
                assets.textured_shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );
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
    }

    pub(super) fn render_gl_conveyors(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            assets, tile_range, ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_conveyor));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let conv = self.game.tiles()[[ix, iy]].conveyor;
                self.render_gl_conveyor(gl, ctx, ix, iy, conv);
            }
        }

        Ok(())
    }

    pub(super) fn render_gl_conveyor_plan(&self, gl: &GL, ctx: &RenderContext) {
        let assets = &ctx.assets;
        gl.use_program(Some(&assets.textured_shader.program));
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_conveyor));
        gl.uniform1f(assets.textured_shader.alpha_loc.as_ref(), 0.5);
        for (pos, conv) in self.game.iter_conveyor_plan() {
            self.render_gl_conveyor(gl, ctx, pos[0], pos[1], *conv);
        }
    }
}
