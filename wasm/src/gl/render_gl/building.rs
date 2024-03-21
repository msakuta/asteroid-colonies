use super::{super::utils::Flatten, enable_buffer, lerp, RenderContext};
use crate::AsteroidColonies;

use ::asteroid_colonies_logic::{
    building::{Building, BuildingType},
    task::{Task, MOVE_TIME},
    Direction, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use wasm_bindgen::JsValue;
use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_buildings(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            frac_frame,
            assets,
            shader,
            offset,
            scale,
            tile_range,
            ..
        } = ctx;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        let set_texture_transform = |tx, ty, sx, sy| {
            let tex_transform = Matrix3::from_nonuniform_scale(sx, sy)
                * Matrix3::from_translation(Vector2::new(tx as f32, ty as f32));

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );
        };

        let render_bldg = |building: &Building| {
            let [x, y] = if let Task::Move(move_time, next) = &building.task {
                next.last()
                    .map(|next| {
                        lerp(
                            building.pos,
                            *next,
                            (MOVE_TIME - move_time + frac_frame * self.game.get_power_ratio())
                                / MOVE_TIME,
                        )
                    })
                    .unwrap_or_else(|| [building.pos[0] as f64, building.pos[1] as f64])
            } else {
                [building.pos[0] as f64, building.pos[1] as f64]
                // [crew.pos[0] as f64, crew.pos[1] as f64]
            };
            let [sx, sy] = building.type_.size();
            let direction = building.direction;
            let x = (x as f64 + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y as f64 + offset[1] as f64 / TILE_SIZE) as f32;
            use std::f32::consts::PI;
            let rot = match direction {
                Some(Direction::Left) => 0.5 * PI,
                Some(Direction::Up) => PI,
                Some(Direction::Right) => -0.5 * PI,
                _ => 0.,
            };
            let transform = ctx.to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_nonuniform_scale(sx as f32, sy as f32, 1.)
                * Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.))
                * Matrix4::from_angle_z(Rad(rot))
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        let time = self.game.get_global_time();

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for building in self.game.iter_building() {
            if building.pos[0] < xmin
                || xmax < building.pos[0]
                || building.pos[1] < ymin
                || ymax < building.pos[1]
            {
                continue;
            }
            match building.type_ {
                BuildingType::Power => {
                    let (sx, sy) = ((time / 5 % 2) as f32, 0.);
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_atomic_battery));
                    set_texture_transform(sx, sy, 0.5, 1.);
                    render_bldg(&building);
                }
                BuildingType::Battery => {
                    let sx = building
                        .energy
                        .zip(building.type_.energy_capacity())
                        .map(|(c, max)| (c as f64 / max as f64 * 4.).floor().min(3.))
                        .unwrap_or(0.);
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_battery));
                    set_texture_transform(sx as f32, 0., 0.25, 1.);
                    render_bldg(&building);
                }
                BuildingType::Excavator => {
                    let sx = if let Task::Excavate(_, _) = building.task {
                        (time % 2 + 1) as f32
                    } else {
                        0.
                    };
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_excavator));
                    set_texture_transform(sx, 0., 1. / 3., 1.);
                    render_bldg(&building);
                }
                BuildingType::Assembler => {
                    let sx = if !matches!(building.task, Task::None) {
                        (time % 2 + 1) as f32
                    } else {
                        0.
                    };
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_assembler));
                    set_texture_transform(sx, 0., 1. / 3., 1.);
                    render_bldg(&building);
                }
                BuildingType::Furnace => {
                    let sx = if !matches!(building.task, Task::None) {
                        (time % 2 + 1) as f32
                    } else {
                        0.
                    };
                    gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_furnace));
                    set_texture_transform(sx, 0., 1. / 3., 1.);
                    render_bldg(&building);
                }
                _ => {
                    if let Some(tex) = assets.building_to_tex(building.type_) {
                        gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                        set_texture_transform(0., 0., 1., 1.);
                        render_bldg(&building);
                    }
                }
            }
        }

        Ok(())
    }
}
