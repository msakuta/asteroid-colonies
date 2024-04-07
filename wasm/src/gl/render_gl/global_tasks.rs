use super::{super::utils::Flatten, render_global_task_bar, RenderContext};
use crate::{gl::utils::enable_buffer, AsteroidColonies};

use ::asteroid_colonies_logic::{
    task::{GlobalTask, LABOR_EXCAVATE_TIME},
    TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};

use web_sys::{WebGlRenderingContext as GL, WebGlTexture};

impl AsteroidColonies {
    pub(super) fn render_gl_global_tasks(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext { assets, .. } = ctx;

        for task in self.game.iter_global_task() {
            match &*task {
                GlobalTask::Excavate(t, pos) => {
                    render_icon(gl, ctx, *pos, &assets.tex_excavate);
                    render_global_task_bar(gl, ctx, *pos, 1., *t, LABOR_EXCAVATE_TIME);
                }
                GlobalTask::Cleanup(pos) => {
                    render_icon(gl, ctx, *pos, &assets.tex_cleanup);
                }
            }
        }
    }
}

fn render_icon(gl: &GL, ctx: &RenderContext, pos: [i32; 2], tex: &WebGlTexture) {
    let RenderContext {
        assets: super::Assets {
            textured_shader: shader,
            ..
        },
        offset,
        to_screen,
        scale,
        ..
    } = ctx;

    let x = (pos[0] as f64 + offset[0] as f64 / TILE_SIZE) as f32;
    let y = (pos[1] as f64 + offset[1] as f64 / TILE_SIZE) as f32;
    gl.use_program(Some(&shader.program));
    gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
    gl.bind_texture(GL::TEXTURE_2D, Some(tex));
    gl.uniform_matrix3fv_with_f32_array(
        shader.tex_transform_loc.as_ref(),
        false,
        Matrix3::identity().flatten(),
    );

    let transform = to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    enable_buffer(gl, &ctx.assets.screen_buffer, 2, shader.vertex_position);
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
}
