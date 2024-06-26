use super::{super::utils::Flatten, enable_buffer, lerp, path::render_path, RenderContext};
use crate::{
    gl::shader_bundle::ShaderBundle,
    render::{BAR_HEIGHT, BAR_MARGIN, BAR_WIDTH, TILE_SIZE},
    AsteroidColonies,
};

use ::asteroid_colonies_logic::{
    building::{Building, BuildingType},
    task::{BuildingTask, MOVE_TIME},
    Direction, ItemType,
};
use cgmath::{Matrix3, Matrix4, Rad, SquareMatrix, Vector2, Vector3};
use wasm_bindgen::JsValue;
use web_sys::WebGlRenderingContext as GL;

impl AsteroidColonies {
    pub(super) fn render_gl_buildings(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            frac_frame,
            assets,
            offset,
            scale,
            tile_range,
            ..
        } = ctx;

        let shader = &assets.textured_shader;

        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        let render_bldg = |building: &Building| {
            let pos = match &building.task {
                BuildingTask::Move(move_time, path)
                | BuildingTask::MoveToExcavate {
                    t: move_time, path, ..
                } => path
                    .last()
                    .map(|next| {
                        lerp(
                            building.pos,
                            *next,
                            (MOVE_TIME - move_time + frac_frame * self.game.get_power_ratio())
                                / MOVE_TIME,
                        )
                    })
                    .unwrap_or_else(|| [building.pos[0] as f64, building.pos[1] as f64]),
                _ => {
                    [building.pos[0] as f64, building.pos[1] as f64]
                    // [crew.pos[0] as f64, crew.pos[1] as f64]
                }
            };
            let [sx, sy] = building.type_.size();
            let direction = building.direction;
            let x = (pos[0] + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (pos[1] + offset[1] as f64 / TILE_SIZE) as f32;
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
            pos
        };

        // Render objects in the building perimeter. Subject to the culling, if the screen is out of view.
        let render_main = |building: &Building| {
            gl.use_program(Some(&shader.program));
            gl.uniform1f(shader.alpha_loc.as_ref(), 1.);

            enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

            let [x, y] = if render_gl_building_texture(gl, ctx, building) {
                render_bldg(&building)
            } else {
                [building.pos[0] as f64, building.pos[1] as f64]
            };

            let render_item = |item: &ItemType| {
                gl.bind_texture(GL::TEXTURE_2D, Some(assets.item_to_tex(*item)));
                let x = (x + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (y + offset[1] as f64 / TILE_SIZE) as f32;
                let size = building.type_.size();
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    Matrix3::identity().flatten(),
                );
                let transform = ctx.to_screen
                    * scale
                    * Matrix4::from_translation(Vector3::new(x as f32, y as f32, 0.))
                    * Matrix4::from_translation(Vector3::new(
                        0.5 * size[0] as f32 - 0.25,
                        0.5 * size[1] as f32 - 0.25,
                        0.,
                    ))
                    * Matrix4::from_nonuniform_scale(0.5, 0.5, 1.);
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    transform.flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            };

            if let Some(ref r) = building.recipe {
                if let Some((item, _)) = r.outputs.iter().next() {
                    render_item(item);
                }
            } else if let BuildingTask::Assemble { ref outputs, .. } = building.task {
                if let Some((item, _)) = outputs.iter().next() {
                    render_item(item);
                }
            }

            let task_target = match building.task {
                BuildingTask::Move(t, _) => Some((t, MOVE_TIME)),
                BuildingTask::Assemble { t, max_t, .. } => Some((t, max_t)),
                _ => None,
            };

            let flat_shader = &assets.flat_shader;

            if let Some((t, max_time)) = task_target {
                RenderBar {
                    gl,
                    ctx,
                    x,
                    y,
                    v: t as f64,
                    max: max_time as f64,
                    scale: building.type_.size()[0] as f64,
                    color: [0., 0.75, 0., 1.],
                    shader: flat_shader,
                }
                .render_bar();
            }

            let inventory_count: usize = building.inventory.iter().map(|item| *item.1).sum();
            if 0 < inventory_count {
                RenderBar {
                    gl,
                    ctx,
                    x: x + (1. - (BAR_WIDTH) / TILE_SIZE) / 2.,
                    y: y + (1. - (BAR_HEIGHT + BAR_MARGIN * 2.) / TILE_SIZE),
                    v: inventory_count as f64,
                    max: building.type_.capacity() as f64,
                    scale: building.type_.size()[0] as f64,
                    color: [0.75, 0.75, 0., 1.],
                    shader: flat_shader,
                }
                .render_bar();
            }
        };

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for building in self.game.iter_building() {
            if building.intersects_rect(
                [xmin, ymin],
                [(xmax - xmin) as usize, (ymax - ymin) as usize],
            ) {
                render_main(&building);
            }

            if let BuildingTask::Move(_, path) | BuildingTask::MoveToExcavate { path, .. } =
                &building.task
            {
                render_path(gl, ctx, path, &[1., 0.5, 0.0, 1.]);
            }
        }

        // Reset to the default shader for next rendering
        gl.use_program(Some(&shader.program));

        Ok(())
    }
}

/// Set up texture matrix for textured_shader for this building and return true on success.
/// The building may be an actual building or a ghost (construction plan).
/// We use [`BuildingLike`] trait to allow sharing logic among actual and ghost buildings,
/// because some building uses custom texture coordinates for animation.
pub(super) fn render_gl_building_texture(
    gl: &GL,
    ctx: &RenderContext,
    building: &impl BuildingLike,
) -> bool {
    let assets = &ctx.assets;
    let time = ctx.view_time / 0.1;
    let shader = &assets.textured_shader;
    let set_texture_transform = |tx, ty, sx, sy| {
        let tex_transform = Matrix3::from_nonuniform_scale(sx, sy)
            * Matrix3::from_translation(Vector2::new(tx as f32, ty as f32));

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            tex_transform.flatten(),
        );
    };

    match building.get_type() {
        BuildingType::Power => {
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_atomic_battery));
            let (sx, sy) = ((time / 5. % 2.).floor() as f32, 0.);
            set_texture_transform(sx, sy, 0.5, 1.);
        }
        BuildingType::Battery => {
            let sx = building
                .get_energy()
                .zip(building.get_type().energy_capacity())
                .map(|(c, max)| (c as f64 / max as f64 * 4.).floor().min(3.))
                .unwrap_or(0.);
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_battery));
            set_texture_transform(sx as f32, 0., 0.25, 1.);
        }
        BuildingType::Excavator => {
            let sx = if let BuildingTask::Excavate(_, _) = building.get_task() {
                ((time % 2.).floor() + 1.) as f32
            } else {
                0.
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_excavator));
            set_texture_transform(sx, 0., 1. / 3., 1.);
        }
        BuildingType::Assembler => {
            let sx = if !matches!(building.get_task(), BuildingTask::None) {
                ((time % 2.).floor() + 1.) as f32
            } else {
                0.
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_assembler));
            set_texture_transform(sx, 0., 1. / 3., 1.);
        }
        BuildingType::Furnace => {
            let sx = if !matches!(building.get_task(), BuildingTask::None) {
                ((time % 2.).floor() + 1.) as f32
            } else {
                0.
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_furnace));
            set_texture_transform(sx, 0., 1. / 3., 1.);
        }
        _ => {
            if let Some(tex) = assets.building_to_tex(building.get_type()) {
                gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                set_texture_transform(0., 0., 1., 1.);
            } else {
                return false;
            }
        }
    }
    true
}

/// Mockable building, used for construction ghosts
pub(super) trait BuildingLike {
    fn get_type(&self) -> BuildingType;
    fn get_energy(&self) -> Option<usize>;
    fn get_task(&self) -> &BuildingTask;
}

impl BuildingLike for Building {
    fn get_type(&self) -> BuildingType {
        self.type_
    }
    fn get_energy(&self) -> Option<usize> {
        self.energy
    }
    fn get_task(&self) -> &BuildingTask {
        &self.task
    }
}

impl BuildingLike for BuildingType {
    fn get_type(&self) -> BuildingType {
        *self
    }
    fn get_energy(&self) -> Option<usize> {
        None
    }
    fn get_task(&self) -> &BuildingTask {
        &BuildingTask::None
    }
}

struct RenderBar<'a> {
    gl: &'a GL,
    ctx: &'a RenderContext<'a>,
    x: f64,
    y: f64,
    v: f64,
    max: f64,
    scale: f64,
    color: [f32; 4],
    shader: &'a ShaderBundle,
}

impl<'a> RenderBar<'a> {
    fn render_bar(&self) {
        let shader = self.shader;
        self.gl.use_program(Some(&shader.program));
        self.gl
            .uniform4f(shader.color_loc.as_ref(), 0.1, 0.1, 0.1, 1.);

        let x = (self.x + self.ctx.offset[0] / TILE_SIZE) as f32;
        let y = (self.y + self.ctx.offset[1] / TILE_SIZE) as f32;
        let sx = (BAR_WIDTH / TILE_SIZE * self.scale) as f32;
        let sy = (BAR_HEIGHT / TILE_SIZE * self.scale * 0.5) as f32;
        let transform = self.ctx.to_screen
            * self.ctx.scale
            * Matrix4::from_translation(Vector3::new(x, y, 0.))
            * Matrix4::from_nonuniform_scale(sx, sy, 1.);
        self.gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            transform.flatten(),
        );
        self.gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        self.gl
            .uniform4fv_with_f32_array(shader.color_loc.as_ref(), &self.color);
        let sx = (BAR_WIDTH / TILE_SIZE * self.scale * self.v / self.max) as f32;
        let transform = self.ctx.to_screen
            * self.ctx.scale
            * Matrix4::from_translation(Vector3::new(x, y, 0.))
            * Matrix4::from_nonuniform_scale(sx, sy, 1.);
        self.gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            transform.flatten(),
        );
        self.gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
    }
}
