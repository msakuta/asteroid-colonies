use super::{super::utils::Flatten, RenderContext};
use crate::{
    render::{BAR_HEIGHT, BAR_WIDTH},
    AsteroidColonies,
};

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
            match task {
                GlobalTask::Excavate(t, pos) => {
                    render_icon(gl, ctx, *pos, &assets.tex_excavate);
                    render_global_task_bar(gl, ctx, *pos, *t, LABOR_EXCAVATE_TIME);
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
        shader,
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
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
}

fn render_global_task_bar(gl: &GL, ctx: &RenderContext, [x, y]: [i32; 2], t: f64, max_time: f64) {
    let Some(ref shader) = ctx.assets.flat_shader else {
        return;
    };
    gl.use_program(Some(&shader.program));
    gl.uniform4f(shader.color_loc.as_ref(), 0.5, 0., 0., 1.);

    let x = (x as f64 + ctx.offset[0] / TILE_SIZE) as f32;
    let y = (y as f64 + ctx.offset[1] / TILE_SIZE) as f32;
    let sx = (BAR_WIDTH / TILE_SIZE) as f32;
    let sy = (BAR_HEIGHT / TILE_SIZE * 0.5) as f32;
    let transform = ctx.to_screen
        * ctx.scale
        * Matrix4::from_translation(Vector3::new(x, y, 0.))
        * Matrix4::from_nonuniform_scale(sx, sy, 1.);
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

    gl.uniform4f(shader.color_loc.as_ref(), 0., 0.5, 0., 1.);
    let sx = (BAR_WIDTH / TILE_SIZE * t / max_time) as f32;
    let transform = ctx.to_screen
        * ctx.scale
        * Matrix4::from_translation(Vector3::new(x, y, 0.))
        * Matrix4::from_nonuniform_scale(sx, sy, 1.);
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
}
