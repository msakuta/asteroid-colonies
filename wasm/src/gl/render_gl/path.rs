use super::{
    super::utils::{vertex_buffer_data, Flatten},
    RenderContext,
};
use crate::render::TILE_SIZE;

use web_sys::WebGlRenderingContext as GL;

use cgmath::{vec2, InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector2, Vector3};

const PATH_WIDTH: f32 = 0.1;
const TEX_SCROLL_SCALE: f32 = 0.5;

pub(super) fn render_path(gl: &GL, ctx: &RenderContext, path: &[[i32; 2]], color: &[f32; 4]) {
    let shader = &ctx.assets.vertex_textured_shader;
    gl.use_program(Some(&shader.program));
    gl.bind_texture(GL::TEXTURE_2D, Some(&ctx.assets.tex_path));

    gl.uniform_matrix3fv_with_f32_array(
        shader.tex_transform_loc.as_ref(),
        false,
        Matrix3::identity().flatten(),
    );

    let mut vertices = Vec::with_capacity(path.len() * 8);
    let mut add_vertex = |pos: Vector2<f32>, normal: Vector2<f32>, t: f32| {
        let t_coord = -TEX_SCROLL_SCALE * t - (ctx.view_time % 1.) as f32;
        vertices.extend_from_slice(&[
            pos[0] + 0.5 + normal[0] * PATH_WIDTH,
            pos[1] + 0.5 + normal[1] * PATH_WIDTH,
            t_coord,
            0.,
            pos[0] + 0.5 - normal[0] * PATH_WIDTH,
            pos[1] + 0.5 - normal[1] * PATH_WIDTH,
            t_coord,
            1.,
        ]);
    };
    let mut first = true;
    let mut last = None;
    for (t, ((prev, cur), next)) in path
        .iter()
        .zip(path.iter().skip(1))
        .zip(path.iter().skip(2))
        .enumerate()
    {
        let prev = vec2(prev[0], prev[1]).cast::<f32>().unwrap();
        let cur = vec2(cur[0], cur[1]).cast::<f32>().unwrap();
        let next = vec2(next[0], next[1]).cast::<f32>().unwrap();
        let prev_delta = cur - prev;
        let next_delta = next - cur;
        let prev_normal = vec2(prev_delta[1], -prev_delta[0]);
        let next_normal = vec2(next_delta[1], -next_delta[0]);
        let cur_normal = (prev_normal + next_normal).normalize();
        if first {
            first = false;
            add_vertex(prev, prev_normal, 0.);
        }
        add_vertex(cur, cur_normal, (t + 1) as f32);
        last = Some((next, next_normal, t + 2));
    }
    if let Some((pos, normal, t)) = last {
        add_vertex(pos, normal, t as f32);
    }

    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&ctx.assets.path_buffer));
    let attrib_size = std::mem::size_of::<[f32; 2]>() as i32;
    gl.vertex_attrib_pointer_with_i32(
        shader.vertex_position,
        2,
        GL::FLOAT,
        false,
        attrib_size * 2,
        0,
    );
    gl.enable_vertex_attrib_array(shader.vertex_position);
    gl.vertex_attrib_pointer_with_i32(
        shader.tex_coord_position,
        2,
        GL::FLOAT,
        false,
        attrib_size * 2,
        attrib_size,
    );
    gl.enable_vertex_attrib_array(shader.tex_coord_position);

    vertex_buffer_data(gl, &vertices);

    gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), color);

    let x = ctx.offset[0] / TILE_SIZE;
    let y = ctx.offset[1] / TILE_SIZE;
    let transform =
        ctx.to_screen * ctx.scale * Matrix4::from_translation(Vector3::new(x as f32, y as f32, 0.));
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_STRIP, 0, vertices.len() as i32 / 4);
}
