mod background;
mod building;
mod construction;
mod conveyor;
mod crews;
mod cursor;
mod global_tasks;
mod path;
mod power_grid;
mod transports;

use super::utils::{enable_buffer, Flatten};
use crate::{
    console_log, js_str,
    render::{BAR_HEIGHT, BAR_MARGIN, BAR_WIDTH},
    AsteroidColonies,
};

use ::asteroid_colonies_logic::{Pos, TILE_SIZE};
use cgmath::{Matrix4, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::WebGlRenderingContext as GL;

use super::{assets::Assets, shader_bundle::ShaderBundle};

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render_gl(&self, gl: &GL, frac_frame: f64) -> Result<(), JsValue> {
        gl.clear_color(0.0, 0.0, 0.5, 1.);
        gl.clear(GL::COLOR_BUFFER_BIT);

        gl.enable(GL::BLEND);
        gl.disable(GL::DEPTH_TEST);

        let ctx = RenderContext::new(self, frac_frame)?;

        self.render_gl_background(gl, &ctx)?;
        self.render_gl_power_grid(gl, &ctx)?;
        self.render_gl_conveyors(gl, &ctx)?;
        self.render_gl_buildings(gl, &ctx)?;
        self.render_gl_crews(gl, &ctx)?;
        self.render_gl_global_tasks(gl, &ctx);
        self.render_gl_constructions(gl, &ctx);
        self.render_gl_conveyor_plan(gl, &ctx);
        self.render_gl_transports(gl, &ctx);

        if let Some(cursor) = self.move_cursor {
            self.render_gl_cursor(gl, &cursor, &ctx, &ctx.assets.tex_move_cursor)?;
        } else if let Some(cursor) = self.cursor {
            self.render_gl_cursor(gl, &cursor, &ctx, &ctx.assets.tex_cursor)?;
        }

        Ok(())
    }
}

/// Cache of common variables throughout the rendering.
struct RenderContext<'a> {
    /// Fractional frame to interpolate objects motions
    frac_frame: f64,
    assets: &'a Assets,
    shader: &'a ShaderBundle,
    offset: [f64; 2],
    scale: Matrix4<f32>,
    to_screen: Matrix4<f32>,
    tile_range: [i32; 4],
}

impl<'a> RenderContext<'a> {
    fn new(ac: &'a AsteroidColonies, frac_frame: f64) -> Result<Self, JsValue> {
        let Some(assets) = ac.gl_assets.as_ref() else {
            console_log!("Warning: gl_assets are not initialized!");
            return Err(js_str!("gl_assets are not initialized"));
        };

        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;

        let vp = &ac.viewport;
        let scale_x = (vp.scale * TILE_SIZE) as f32 / (vp.size[0] as f32);
        let scale_y = (vp.scale * TILE_SIZE) as f32 / (vp.size[1] as f32);
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let ymin = ((-offset[1]).div_euclid(TILE_SIZE)) as i32;
        let ymax = (-offset[1] + vp.size[1] / vp.scale + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        let xmin = ((-offset[0]).div_euclid(TILE_SIZE)) as i32;
        let xmax = (-offset[0] + vp.size[0] / vp.scale + TILE_SIZE).div_euclid(TILE_SIZE) as i32;

        Ok(Self {
            frac_frame,
            assets,
            shader,
            offset,
            scale: Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.),
            to_screen: Matrix4::from_nonuniform_scale(2., -2., 1.)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.)),
            tile_range: [xmin, xmax, ymin, ymax],
        })
    }
}

fn lerp(p0: Pos, p1: Pos, f: f64) -> [f64; 2] {
    [
        p0[0] as f64 * (1. - f) + p1[0] as f64 * f,
        p0[1] as f64 * (1. - f) + p1[1] as f64 * f,
    ]
}

fn render_global_task_bar(
    gl: &GL,
    ctx: &RenderContext,
    [x, y]: [i32; 2],
    width: f32,
    t: f64,
    max_time: f64,
) {
    let Some(ref shader) = ctx.assets.flat_shader else {
        return;
    };
    gl.use_program(Some(&shader.program));
    gl.uniform4f(shader.color_loc.as_ref(), 0.1, 0.1, 0.1, 1.);

    let x = (x as f64 + (ctx.offset[0] + BAR_MARGIN) / TILE_SIZE) as f32;
    let y = (y as f64 + (ctx.offset[1] + BAR_MARGIN) / TILE_SIZE) as f32;
    let sx = (BAR_WIDTH / TILE_SIZE) as f32 * width;
    let sy = (BAR_HEIGHT / TILE_SIZE) as f32;
    let transform = ctx.to_screen
        * ctx.scale
        * Matrix4::from_translation(Vector3::new(x, y, 0.))
        * Matrix4::from_nonuniform_scale(sx, sy, 1.);
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

    gl.uniform4f(shader.color_loc.as_ref(), 0., 0.5, 0., 1.);
    let sx = (BAR_WIDTH / TILE_SIZE * t / max_time) as f32 * width;
    let transform = ctx.to_screen
        * ctx.scale
        * Matrix4::from_translation(Vector3::new(x, y, 0.))
        * Matrix4::from_nonuniform_scale(sx, sy, 1.);
    gl.uniform_matrix4fv_with_f32_array(shader.transform_loc.as_ref(), false, transform.flatten());
    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
}
