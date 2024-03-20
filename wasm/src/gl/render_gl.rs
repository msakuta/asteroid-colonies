use crate::{
    console_log,
    gl::utils::{enable_buffer, Flatten},
    js_str,
    render::{NEIGHBOR_BITS, SPACE_BIT},
    AsteroidColonies,
};

use asteroid_colonies_logic::{
    building::{Building, BuildingType},
    construction::ConstructionType,
    task::{Task, MOVE_TIME},
    Conveyor, Direction, ItemType, Pos, TileState, Transport, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, SquareMatrix, Vector2, Vector3};
use wasm_bindgen::prelude::*;

use web_sys::{WebGlRenderingContext as GL, WebGlTexture};

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
        self.render_gl_constructions(gl, &ctx);
        self.render_gl_conveyor_plan(gl, &ctx);
        self.render_gl_transports(gl, &ctx);
        self.render_gl_buildings(gl, &ctx)?;
        self.render_gl_crews(gl, &ctx)?;

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

impl AsteroidColonies {
    fn render_gl_background(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            assets,
            shader,
            offset,
            scale,
            tile_range,
            ..
        } = ctx;
        let back_texture_transform = Matrix3::<f32>::from_nonuniform_scale(1. / 4., 1. / 8.);

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_back));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        let mut rendered_tiles = 0;

        let mut render_quarter_tile = |image: u8, x, y| -> Result<(), JsValue> {
            let srcx = ((image & NEIGHBOR_BITS) % 4) as f32;
            let srcy = ((image & NEIGHBOR_BITS) / 4) as f32;
            let bg_y = if image & SPACE_BIT != 0 { 2. } else { 1. };
            let tex_transform = back_texture_transform
                * Matrix3::from_translation(Vector2::new(0., bg_y))
                * Matrix3::from_scale(0.5);

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );

            let transform = ctx.to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_scale(0.5);
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            let tex_transform = back_texture_transform
                * Matrix3::from_scale(0.5)
                * Matrix3::from_translation(Vector2::new(srcx, srcy));

            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                tex_transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            rendered_tiles += 1;
            Ok(())
        };

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                let tile = self.game.tile_at([ix, iy]);
                let (sx, sy) = match tile.state {
                    TileState::Empty => (0., 1.),
                    TileState::Solid => (0., 0.),
                    TileState::Space => (0., 2.),
                };
                let tex_transform = back_texture_transform
                    * Matrix3::from_translation(Vector2::new(sx as f32, sy as f32));

                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    tex_transform.flatten(),
                );

                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                let transform =
                    ctx.to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    transform.flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

                if tile.image_idx.lt & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.lt, x, y)?;
                }
                if tile.image_idx.lb & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.lb, x, y + 0.5)?;
                }
                if tile.image_idx.rb & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.rb, x + 0.5, y + 0.5)?;
                }
                if tile.image_idx.rt & NEIGHBOR_BITS != 0 {
                    render_quarter_tile(tile.image_idx.rt, x + 0.5, y)?;
                }
            }
        }

        // console_log!("rendered_tiles: {}", rendered_tiles);

        Ok(())
    }

    fn render_gl_power_grid(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
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

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_power_grid));
        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let [xmin, xmax, ymin, ymax] = *tile_range;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                if !self.game.tiles()[[ix, iy]].power_grid {
                    continue;
                }
                let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
                let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
                let transform =
                    ctx.to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    transform.flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
        }

        Ok(())
    }

    fn render_gl_conveyor(&self, gl: &GL, ctx: &RenderContext, x: i32, y: i32, conv: Conveyor) {
        let RenderContext {
            shader,
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
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        };

        let set_texture_transform = |sx, sy| {
            let tex_transform = conveyor_texture_transform
                * Matrix3::from_translation(Vector2::new(sx as f32, sy as f32));

            gl.uniform_matrix3fv_with_f32_array(
                ctx.shader.tex_transform_loc.as_ref(),
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

    fn render_gl_conveyors(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            assets,
            shader,
            tile_range,
            ..
        } = ctx;

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

    fn render_gl_buildings(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
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

    fn render_gl_crews(&self, gl: &GL, ctx: &RenderContext) -> Result<(), JsValue> {
        let RenderContext {
            frac_frame,
            shader,
            assets,
            offset,
            scale,
            ..
        } = ctx;

        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_crew));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        for crew in self.game.iter_crew() {
            let [x, y] = if let Some(next) = crew.path.as_ref().and_then(|p| p.last()) {
                lerp(crew.pos, *next, *frac_frame)
            } else {
                [crew.pos[0] as f64, crew.pos[1] as f64]
            };
            let x = (x + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (y + offset[1] as f64 / TILE_SIZE) as f32;
            let transform = ctx.to_screen
                * scale
                * Matrix4::from_translation(Vector3::new(x as f32, y as f32, 0.))
                * Matrix4::from_translation(Vector3::new(0.5, 0.5, 0.))
                * Matrix4::from_scale(0.5)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.));
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                transform.flatten(),
            );
            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

            // if let Some(path) = &crew.path {
            //     context.set_stroke_style(&JsValue::from("#7f00ff"));
            //     context.set_line_width(2.);
            //     context.begin_path();
            //     let mut first = true;
            //     for node in path.iter().chain(std::iter::once(&crew.pos)) {
            //         let x = (node[0] as f64 + 0.5) * TILE_SIZE + offset[0];
            //         let y = (node[1] as f64 + 0.5) * TILE_SIZE + offset[1];
            //         if first {
            //             first = false;
            //             context.move_to(x, y);
            //         } else {
            //             context.line_to(x, y);
            //         }
            //     }
            //     context.stroke();
            // }
        }
        Ok(())
    }

    fn render_gl_constructions(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext {
            assets,
            shader,
            to_screen,
            offset,
            scale,
            ..
        } = ctx;

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        for construction in self.game.iter_construction() {
            let [ix, iy] = construction.pos;
            let x = (ix as f64 + offset[0] as f64 / TILE_SIZE) as f32;
            let y = (iy as f64 + offset[1] as f64 / TILE_SIZE) as f32;
            match construction.get_type() {
                ConstructionType::Building(ty) => {
                    let Some(tex) = assets.building_to_tex(ty) else {
                        continue;
                    };
                    let size = ty.size();
                    let width = size[0] as f32;
                    let height = size[1] as f32;
                    gl.bind_texture(GL::TEXTURE_2D, Some(tex));
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
            gl.bind_texture(GL::TEXTURE_2D, Some(tex));
            gl.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                Matrix3::identity().flatten(),
            );
            let size = construction.size();
            let width = size[0] as f32;
            let height = size[1] as f32;
            // const SRC_WIDTH: f64 = 64.;
            // const SRC_HEIGHT: f64 = 64.;
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
            // context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            //     img, 0., 0., SRC_WIDTH, SRC_HEIGHT, x, y, width, height,
            // )?;
            // render_global_task_bar(
            //     context,
            //     [x, y],
            //     construction.progress(),
            //     construction.recipe.time,
            // );
            // if let Some((t, pos, max_time)) = task_target {
            //     render_global_task_bar(context, pos, t, max_time);
            // }
        }
    }

    fn render_gl_conveyor_plan(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext { shader, assets, .. } = ctx;
        gl.bind_texture(GL::TEXTURE_2D, Some(&assets.tex_conveyor));
        gl.uniform1f(shader.alpha_loc.as_ref(), 0.5);
        for (pos, conv) in self.game.iter_conveyor_plan() {
            self.render_gl_conveyor(gl, ctx, pos[0], pos[1], *conv);
        }
    }

    fn render_gl_transports(&self, gl: &GL, ctx: &RenderContext) {
        let RenderContext {
            frac_frame,
            assets,
            shader,
            to_screen,
            offset,
            scale,
            ..
        } = ctx;

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
            let tex = match t.item {
                ItemType::RawOre => &assets.tex_raw_ore,
                ItemType::IronIngot => &assets.tex_iron_ingot,
                ItemType::CopperIngot => &assets.tex_copper_ingot,
                ItemType::LithiumIngot => &assets.tex_lithium_ingot,
                ItemType::Cilicate => &assets.tex_cilicate,
                ItemType::Gear => &assets.tex_gear,
                ItemType::Wire => &assets.tex_wire,
                ItemType::Circuit => &assets.tex_circuit,
                ItemType::Battery => &assets.tex_battery_item,
                ItemType::PowerGridComponent => &assets.tex_power_grid,
                ItemType::ConveyorComponent => &assets.tex_conveyor_item,
                ItemType::AssemblerComponent => &assets.tex_assembler_component,
            };
            gl.bind_texture(GL::TEXTURE_2D, Some(tex));
            let [x, y] = if 2 <= t.path.len() {
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

        for t in self.game.iter_transport() {
            // context.set_stroke_style(&JsValue::from("#ffff00"));
            // context.set_line_width(2.);
            // context.begin_path();
            // for node in &t.path {
            //     context.line_to(
            //         (node[0] as f64 + 0.5) * TILE_SIZE + offset[0],
            //         (node[1] as f64 + 0.5) * TILE_SIZE + offset[1],
            //     );
            // }
            // context.stroke();
            render_transport(&t);
        }
    }

    fn render_gl_cursor(
        &self,
        gl: &GL,
        cursor: &Pos,
        ctx: &RenderContext,
        tex: &WebGlTexture,
    ) -> Result<(), JsValue> {
        let RenderContext {
            shader,
            assets,
            scale,
            to_screen,
            ..
        } = ctx;

        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);
        gl.uniform1i(shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(tex));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::identity().flatten(),
        );

        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];
        let x = (cursor[0] as f64 + offset[0] as f64 / TILE_SIZE) as f32;
        let y = (cursor[1] as f64 + offset[1] as f64 / TILE_SIZE) as f32;
        let transform = to_screen * scale * Matrix4::from_translation(Vector3::new(x, y, 0.));

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            transform.flatten(),
        );

        enable_buffer(&gl, &assets.screen_buffer, 2, shader.vertex_position);
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        Ok(())
    }
}

fn lerp(p0: Pos, p1: Pos, f: f64) -> [f64; 2] {
    [
        p0[0] as f64 * (1. - f) + p1[0] as f64 * f,
        p0[1] as f64 * (1. - f) + p1[1] as f64 * f,
    ]
}
