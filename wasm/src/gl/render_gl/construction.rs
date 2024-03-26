use super::{
    super::utils::Flatten, building::render_gl_building_texture, render_global_task_bar,
    RenderContext,
};
use crate::{gl::utils::enable_buffer, AsteroidColonies};

use ::asteroid_colonies_logic::{construction::ConstructionType, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_constructions(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext {
            assets,
            to_screen,
            offset,
            scale,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        let view_time = ctx.view_time;
        let alpha = triangle(view_time as f32);

        for construction in self.game.iter_construction() {
            let [ix, iy] = construction.pos;
            let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;

            gl.use_program(Some(&shader.program));
            enable_buffer(gl, &assets.screen_buffer, 2, shader.vertex_position);
            gl.uniform1f(shader.alpha_loc.as_ref(), 0.5);

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                Matrix3::identity().flatten(),
            );

            match construction.get_type() {
                ConstructionType::Building(ty) => {
                    if !render_gl_building_texture(gl, ctx, &ty) {
                        continue;
                    }
                    let size = ty.size();
                    let width = size[0] as f32;
                    let height = size[1] as f32;
                    let transform = to_screen
                        * scale
                        * Matrix4::from_translation(Vector3::new(x, y, 0.))
                        * Matrix4::from_nonuniform_scale(width, height, 1.);
                    gl.uniform_matrix4fv_with_f32_array(
                        shader.transform_loc.as_ref(),
                        false,
                        transform.flatten(),
                    );
                    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                }
                ConstructionType::PowerGrid => {
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_power_grid));
                    let transform =
                        to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
                    gl.uniform_matrix4fv_with_f32_array(
                        shader.transform_loc.as_ref(),
                        false,
                        transform.flatten(),
                    );
                    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                }
                ConstructionType::Conveyor(conv) => {
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_conveyor));
                    self.render_gl_conveyor(gl, ctx, ix, iy, conv);
                }
            }
            let tex = if construction.canceling() {
                &assets.tex_deconstruction
            } else {
                &assets.tex_construction
            };
            gl.uniform1f(shader.alpha_loc.as_ref(), alpha);
            gl.bind_texture(GL::TEXTURE_2D, Some(tex));
            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                Matrix3::identity().flatten(),
            );
            let size = construction.size();
            let width = size[0] as f32;
            let height = size[1] as f32;
            let transform = to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_nonuniform_scale(width, height, 1.);
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            if 0. < construction.progress() {
                render_global_task_bar(
                    gl,
                    ctx,
                    [ix, iy],
                    size[0] as f32,
                    construction.progress(),
                    construction.recipe.time,
                );
            }
        }
    }
}

fn triangle(f: f32) -> f32 {
    let view_time_mod = f.rem_euclid(2.);
    if view_time_mod < 1. {
        view_time_mod
    } else {
        2. - view_time_mod
    }
}

#[test]
fn a() {
    for i in -10..20 {
        let d = i as f32 / 4.;
        println!("f({d:5})= {}", triangle(d));
    }
}
