use super::{
    super::utils::{enable_buffer, vertex_buffer_data, Flatten},
    RenderContext,
};
use crate::render::TILE_SIZE;

use web_sys::WebGlRenderingContext as GL;

use cgmath::{vec2, InnerSpace, Matrix4, Vector2, Vector3};

const PATH_WIDTH: f32 = 0.05;

pub(super) fn render_path(gl: &GL, ctx: &RenderContext, path: &[[i32; 2]], color: &[f32; 4]) {
    let Some(ref shader) = ctx.assets.flat_shader else {
        return;
    };
    gl.use_program(Some(&shader.program));
    let mut vertices = Vec::with_capacity(path.len() * 2);
    let mut add_vertex = |pos: Vector2<f32>, normal: Vector2<f32>| {
        vertices.extend_from_slice(&[
            pos[0] + 0.5 + normal[0] * PATH_WIDTH,
            pos[1] + 0.5 + normal[1] * PATH_WIDTH,
            pos[0] + 0.5 - normal[0] * PATH_WIDTH,
            pos[1] + 0.5 - normal[1] * PATH_WIDTH,
        ]);
    };
    let mut first = true;
    let mut last = None;
    for ((prev, cur), next) in path
        .iter()
        .zip(path.iter().skip(1))
        .zip(path.iter().skip(2))
    {
        let prev = vec2(prev[0], prev[1]).cast::<f32>().unwrap();
        let cur = vec2(cur[0], cur[1]).cast::<f32>().unwrap();
        let next = vec2(next[0], next[1]).cast::<f32>().unwrap();
        let prev_delta = cur - prev;
        let next_delta = next - cur;
        let prev_normal = vec2(prev_delta[1], -prev_delta[0]);
        let next_normal = vec2(next_delta[1], -next_delta[0]);
        let cur_normal = (prev_normal + next_normal).normalize();
        if !first {
            first = false;
            add_vertex(prev, prev_normal);
        }
        add_vertex(cur, cur_normal);
        last = Some((next, next_normal));
    }
    if let Some((pos, normal)) = last {
        add_vertex(pos, normal);
    }

    enable_buffer(&gl, &ctx.assets.path_buffer, 2, shader.vertex_position);
    vertex_buffer_data(gl, &vertices);

    gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), color);

    let x = ctx.offset[0] / TILE_SIZE;
    let y = ctx.offset[1] / TILE_SIZE;
    let transform =
        ctx.to_screen * ctx.scale * Matrix4::from_translation(Vector3::new(x as f32, y as f32, 0.));
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_STRIP, 0, vertices.len() as i32 / 2);
}
