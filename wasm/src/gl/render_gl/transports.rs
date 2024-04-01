use super::{
    super::utils::Flatten,
    lerp,
    path::{prepare_render_path, render_path},
    RenderContext,
};
use crate::{gl::utils::enable_buffer, AsteroidColonies};

use ::asteroid_colonies_logic::{Transport, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, SquareMatrix, Vector3};

use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_transports(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext {
            frac_frame,
            assets,
            to_screen,
            offset,
            scale,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.0);
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let render_transport = |t: &Transport| {
            let Some(&pos) = t.path.last() else {
                return;
            };
            let tex = assets.item_to_tex(t.item);
            gl.bind_texture(GL::TEXTURE_2D, Some(tex));
            let [x, y] = if 2 <= t.path.len() && !t.is_blocked {
                lerp(pos, t.path[t.path.len() - 2], *frac_frame)
            } else {
                [pos[0] as f64, pos[1] as f64]
            };
            let x = (x + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y + offset[1] as f64 / TILE_SIZE) as f32;
            let transform = to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.))
                * Matrix4::from_scale(0.5)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        prepare_render_path(gl, ctx);

        for t in self.game.iter_transport() {
            render_path(gl, ctx, &t.path, &[1., 1., 0., 1.]);
        }

        gl.use_program(Some(&shader.program));
        for t in self.game.iter_transport() {
            enable_buffer(gl, &assets.screen_buffer, 2, shader.vertex_position);
            render_transport(&t);
        }
    }
}
