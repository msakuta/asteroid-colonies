use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use super::AsteroidColonies;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use asteroid_colonies_logic::{
    building::BuildingType,
    construction::ConstructionType,
    conveyor::Conveyor,
    new_hasher,
    task::{BuildingTask, GlobalTask, EXCAVATE_TIME, LABOR_EXCAVATE_TIME, MOVE_TIME},
    Chunk, Direction, ImageIdx, ItemType, Position, TileState, Tiles, CHUNK_SIZE,
};

pub(crate) const TILE_SIZE: f64 = 32.;
const ITEM_SIZE: f64 = 16.;
pub(crate) const BAR_MARGIN: f64 = 4.;
pub(crate) const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
pub(crate) const BAR_HEIGHT: f64 = 6.;
pub(crate) const SPACE_BIT: u8 = 32;
pub(crate) const NEIGHBOR_BITS: u8 = 0x1f;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#000000"));
        context.clear_rect(0., 0., self.viewport.size[0], self.viewport.size[1]);
        context.set_fill_style(&JsValue::from("#ff0000"));
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];

        let render_power_grid = |context: &CanvasRenderingContext2d, x, y| {
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.assets.img_power_grid,
                0.,
                0.,
                TILE_SIZE,
                TILE_SIZE,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
            )
        };

        let render_conveyor_layer =
            |context: &CanvasRenderingContext2d, x, y, conv: (Direction, Direction)| {
                let (sx, sy) = match conv {
                    (from, to) => {
                        let mut sy = match to {
                            Direction::Left => 0.,
                            Direction::Up => TILE_SIZE,
                            Direction::Right => 2. * TILE_SIZE,
                            Direction::Down => 3. * TILE_SIZE,
                        };
                        let sx = match from {
                            Direction::Left => 0.,
                            Direction::Up => TILE_SIZE,
                            Direction::Right => 2. * TILE_SIZE,
                            Direction::Down => 3. * TILE_SIZE,
                        };
                        if sx <= sy {
                            sy -= TILE_SIZE;
                        }
                        (sx, sy)
                    }
                };
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_conveyor,
                        sx,
                        sy,
                        TILE_SIZE,
                        TILE_SIZE,
                        x,
                        y,
                        TILE_SIZE,
                        TILE_SIZE,
                    )
            };

        let render_conveyor = |context: &CanvasRenderingContext2d,
                               x,
                               y,
                               conv: Conveyor|
         -> Result<(), JsValue> {
            match conv {
                Conveyor::One(from, to) => render_conveyor_layer(context, x, y, (from, to))?,
                Conveyor::Two(first, second) => {
                    render_conveyor_layer(context, x, y, first)?;
                    render_conveyor_layer(context, x, y, second)?;
                }
                Conveyor::Splitter(dir) | Conveyor::Merger(dir) => {
                    let sx = match dir {
                        Direction::Left => 0.,
                        Direction::Up => TILE_SIZE,
                        Direction::Right => 2. * TILE_SIZE,
                        Direction::Down => 3. * TILE_SIZE,
                    };
                    let sy = match conv {
                        Conveyor::Splitter(_) => 3. * TILE_SIZE,
                        _ => 4. * TILE_SIZE,
                    };
                    context
                        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &self.assets.img_conveyor,
                            sx,
                            sy,
                            TILE_SIZE,
                            TILE_SIZE,
                            x,
                            y,
                            TILE_SIZE,
                            TILE_SIZE,
                        )?;
                }
                _ => {}
            };
            Ok(())
        };

        let mut rendered_tiles = 0;
        let mut render_tile = |ix: i32, iy: i32| -> Result<(), JsValue> {
            let y = iy as f64 * TILE_SIZE + offset[1];
            let x = ix as f64 * TILE_SIZE + offset[0];
            let tile = &self.game.tile_at([ix, iy]);
            let (sx, sy) = match tile.state {
                TileState::Empty => (0., TILE_SIZE),
                TileState::Solid => (0., 0.),
                TileState::Space => (0., 2. * TILE_SIZE),
            };
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.assets.img_bg,
                sx,
                sy,
                TILE_SIZE,
                TILE_SIZE,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
            )?;

            let mut render_quarter_tile = |image: u8, xofs, yofs| -> Result<(), JsValue> {
                let srcx = (image & NEIGHBOR_BITS) % 4;
                let srcy = (image & NEIGHBOR_BITS) / 4;
                let bg_y = if image & SPACE_BIT != 0 {
                    2. * TILE_SIZE
                } else {
                    TILE_SIZE
                };
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_bg,
                        0.,
                        bg_y,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        x + xofs,
                        y + yofs,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                    )?;
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_bg,
                        srcx as f64 * TILE_SIZE / 2.,
                        srcy as f64 * TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        x + xofs,
                        y + yofs,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                    )?;
                rendered_tiles += 1;
                Ok(())
            };

            if tile.image_idx.lt & NEIGHBOR_BITS != 0 {
                render_quarter_tile(tile.image_idx.lt, 0., 0.)?;
            }
            if tile.image_idx.lb & NEIGHBOR_BITS != 0 {
                render_quarter_tile(tile.image_idx.lb, 0., TILE_SIZE / 2.)?;
            }
            if tile.image_idx.rb & NEIGHBOR_BITS != 0 {
                render_quarter_tile(tile.image_idx.rb, TILE_SIZE / 2., TILE_SIZE / 2.)?;
            }
            if tile.image_idx.rt & NEIGHBOR_BITS != 0 {
                render_quarter_tile(tile.image_idx.rt, TILE_SIZE / 2., 0.)?;
            }

            if tile.power_grid {
                render_power_grid(context, x, y)?;
            }
            render_conveyor(context, x, y, tile.conveyor)?;
            rendered_tiles += 1;
            Ok(())
        };

        let ymin = ((-offset[1]).div_euclid(TILE_SIZE)) as i32;
        let ymax = (-offset[1] + vp.size[1] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        let xmin = ((-offset[0]).div_euclid(TILE_SIZE)) as i32;
        let xmax = (-offset[0] + vp.size[0] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                render_tile(ix, iy)?;
            }
        }
        // console_log!("rendered_tiles: {}", rendered_tiles);

        let time = self.game.get_global_time();

        for building in self.game.iter_building() {
            let img = self.assets.building_to_img(building.type_);
            let (sx, sy) = match building.type_ {
                BuildingType::Power => ((time / 5 % 2) as f64 * TILE_SIZE, 0.),
                BuildingType::Battery => (
                    building
                        .energy
                        .zip(building.type_.energy_capacity())
                        .map(|(c, max)| (c as f64 / max as f64 * 4.).floor().min(3.) * TILE_SIZE)
                        .unwrap_or(0.),
                    0.,
                ),
                BuildingType::Excavator => {
                    if let BuildingTask::Excavate(_, _) = building.task {
                        ((time % 2 + 1) as f64 * TILE_SIZE, 0.)
                    } else {
                        (0., 0.)
                    }
                }
                BuildingType::Assembler => {
                    if !matches!(building.task, BuildingTask::None) {
                        ((time % 2 + 1) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                BuildingType::Furnace => {
                    if !matches!(building.task, BuildingTask::None) {
                        ((time % 2 + 1) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                _ => (0., 0.),
            };
            let x = building.pos[0] as f64 * TILE_SIZE + offset[0];
            let y = building.pos[1] as f64 * TILE_SIZE + offset[1];
            let size = building.type_.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            use std::f64::consts::PI;
            let draw_rotated = |angle| -> Result<(), JsValue> {
                context.save();
                context.translate(x + TILE_SIZE * 0.5, y + TILE_SIZE * 0.5)?;
                context.rotate(angle)?;
                context.translate(-TILE_SIZE * 0.5, -TILE_SIZE * 0.5)?;
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, sx, sy, width, height, 0., 0., width, height,
                    )?;
                context.restore();
                Ok(())
            };
            match building.direction {
                Some(Direction::Left) => draw_rotated(0.5 * PI)?,
                Some(Direction::Up) => draw_rotated(PI)?,
                Some(Direction::Right) => draw_rotated(-0.5 * PI)?,
                _ => {
                    context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, sx, sy, width, height, x, y, width, height,
                    )?;
                }
            }

            let task_target = match building.task {
                BuildingTask::Excavate(t, _) => Some((t, EXCAVATE_TIME)),
                BuildingTask::Move(t, _) => Some((t, MOVE_TIME)),
                BuildingTask::Assemble { t, max_t, .. } => Some((t, max_t)),
                _ => None,
            };

            if let Some((t, max_time)) = task_target {
                RenderBar {
                    context,
                    x,
                    y,
                    v: t as f64,
                    max: max_time as f64,
                    scale: size[1] as f64,
                    color: "#00af00",
                }
                .render_bar();
            }

            let inventory_count: usize = building.inventory.iter().map(|item| *item.1).sum();
            if 0 < inventory_count {
                RenderBar {
                    context,
                    x,
                    y: y + TILE_SIZE - BAR_HEIGHT - BAR_MARGIN * 2.,
                    v: inventory_count as f64,
                    max: building.type_.capacity() as f64,
                    scale: size[1] as f64,
                    color: "#afaf00",
                }
                .render_bar();
            }

            if let BuildingTask::Move(_, path) = &building.task {
                context.set_stroke_style(&JsValue::from("#ff7f00"));
                context.set_line_width(3.);
                context.begin_path();
                for (i, node) in path
                    .iter()
                    .chain(std::iter::once(&building.pos))
                    .enumerate()
                {
                    let x = (node[0] as f64 + 0.5) * TILE_SIZE + offset[0];
                    let y = (node[1] as f64 + 0.5) * TILE_SIZE + offset[1];
                    if i == 0 {
                        context.move_to(x, y);
                    } else {
                        context.line_to(x, y);
                    }
                }
                context.stroke();
            }
        }

        const CREW_SIZE: f64 = 16.;
        const CREW_OFFSET: f64 = (TILE_SIZE - CREW_SIZE) * 0.5;

        for crew in self.game.iter_crew() {
            let x = crew.pos[0] as f64 * TILE_SIZE + CREW_OFFSET + offset[0];
            let y = crew.pos[1] as f64 * TILE_SIZE + CREW_OFFSET + offset[1];
            let img = &self.assets.img_crew;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., CREW_SIZE, CREW_SIZE, x, y, CREW_SIZE, CREW_SIZE,
            )?;

            if let Some(path) = &crew.path {
                context.set_stroke_style(&JsValue::from("#7f00ff"));
                context.set_line_width(2.);
                context.begin_path();
                let mut first = true;
                for node in path.iter().chain(std::iter::once(&crew.pos)) {
                    let x = (node[0] as f64 + 0.5) * TILE_SIZE + offset[0];
                    let y = (node[1] as f64 + 0.5) * TILE_SIZE + offset[1];
                    if first {
                        first = false;
                        context.move_to(x, y);
                    } else {
                        context.line_to(x, y);
                    }
                }
                context.stroke();
            }
        }

        for task in self.game.iter_global_task() {
            match task {
                GlobalTask::Excavate(t, pos) => {
                    let x = pos[0] as f64 * TILE_SIZE + offset[0];
                    let y = pos[1] as f64 * TILE_SIZE + offset[1];
                    render_global_task_bar(context, [x, y], *t, LABOR_EXCAVATE_TIME);
                }
                GlobalTask::Cleanup(pos) => {
                    let x = pos[0] as f64 * TILE_SIZE + offset[0];
                    let y = pos[1] as f64 * TILE_SIZE + offset[1];
                    context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_cleanup, 0., 0., 32., 32., x, y, 32., 32.)?;
                }
                GlobalTask::MoveItem { src, .. } => {
                    if let Some(bldg) = self.game.get_building(*src) {
                        let x = bldg.pos[0] as f64 * TILE_SIZE + offset[0];
                        let y = bldg.pos[1] as f64 * TILE_SIZE + offset[1];
                        context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &self.assets.img_move_item, 0., 0., 32., 32., x, y, 32., 32.)?;
                    }
                }
            }
        }

        for construction in self.game.iter_construction() {
            let x = construction.pos[0] as f64 * TILE_SIZE + offset[0];
            let y = construction.pos[1] as f64 * TILE_SIZE + offset[1];
            match construction.get_type() {
                ConstructionType::Building(ty) => {
                    let img = self.assets.building_to_img(ty);
                    let size = ty.size();
                    let width = size[0] as f64 * TILE_SIZE;
                    let height = size[1] as f64 * TILE_SIZE;
                    context
                        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img, 0., 0., width, height, x, y, width, height,
                        )?;
                }
                ConstructionType::PowerGrid => render_power_grid(context, x, y)?,
                ConstructionType::Conveyor(conv) => render_conveyor(context, x, y, conv)?,
            }
            let img = if construction.canceling() {
                &self.assets.img_deconstruction
            } else {
                &self.assets.img_construction
            };
            let size = construction.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            const SRC_WIDTH: f64 = 64.;
            const SRC_HEIGHT: f64 = 64.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., SRC_WIDTH, SRC_HEIGHT, x, y, width, height,
            )?;
            render_global_task_bar(
                context,
                [x, y],
                construction.progress(),
                construction.recipe.time,
            );
            // if let Some((t, pos, max_time)) = task_target {
            //     render_global_task_bar(context, pos, t, max_time);
            // }
        }

        for (pos, conv) in self.game.iter_conveyor_plan() {
            let x = pos[0] as f64 * TILE_SIZE + offset[0];
            let y = pos[1] as f64 * TILE_SIZE + offset[1];
            render_conveyor(context, x, y, *conv)?;
        }

        for t in self.game.iter_transport() {
            context.set_stroke_style(&JsValue::from("#ffff00"));
            context.set_line_width(2.);
            context.begin_path();
            for node in &t.path {
                context.line_to(
                    (node[0] as f64 + 0.5) * TILE_SIZE + offset[0],
                    (node[1] as f64 + 0.5) * TILE_SIZE + offset[1],
                );
            }
            context.stroke();
            if let Some(pos) = t.path.last() {
                let (img, sw, sh) = match t.item {
                    ItemType::RawOre => (&self.assets.img_raw_ore, 16., 16.),
                    ItemType::IronIngot => (&self.assets.img_iron_ingot, 16., 16.),
                    ItemType::CopperIngot => (&self.assets.img_copper_ingot, 16., 16.),
                    ItemType::LithiumIngot => (&self.assets.img_lithium_ingot, 16., 16.),
                    ItemType::Cilicate => (&self.assets.img_cilicate, 16., 16.),
                    ItemType::Gear => (&self.assets.img_gear, 32., 32.),
                    ItemType::Wire => (&self.assets.img_wire, 32., 32.),
                    ItemType::Circuit => (&self.assets.img_circuit, 32., 32.),
                    ItemType::Battery => (&self.assets.img_battery_item, 32., 32.),
                    ItemType::PowerGridComponent => (&self.assets.img_power_grid, 32., 32.),
                    ItemType::ConveyorComponent => (&self.assets.img_conveyor_item, 32., 32.),
                    ItemType::AssemblerComponent => {
                        (&self.assets.img_assembler_component, 32., 32.)
                    }
                };
                let tile_offset = (TILE_SIZE as f64 - ITEM_SIZE as f64) / 2.;
                let x = pos[0] as f64 * TILE_SIZE + tile_offset + offset[0];
                let y = pos[1] as f64 * TILE_SIZE + tile_offset + offset[1];
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, 0., 0., sw, sh, x, y, ITEM_SIZE, ITEM_SIZE,
                    )?;
            }
        }

        if self.debug_draw_chunks {
            const CHUNK_TILE_SIZE: f64 = CHUNK_SIZE as f64 * TILE_SIZE;
            for (pos, chunk) in self.game.tiles().chunks() {
                let x = pos.x as f64 * CHUNK_TILE_SIZE + offset[0] + 2.;
                let y = pos.y as f64 * CHUNK_TILE_SIZE + offset[1] + 2.;
                context.set_stroke_style(&JsValue::from(match chunk {
                    Chunk::Tiles(_, _) => "#f00",
                    Chunk::Uniform(_, _) => "#0f0",
                }));
                context.stroke_rect(x, y, CHUNK_TILE_SIZE - 4., CHUNK_TILE_SIZE - 4.);
            }
        }

        if let Some(cursor) = self.cursor {
            let img = &self.assets.img_cursor;
            let x = cursor[0] as f64 * TILE_SIZE + offset[0];
            let y = cursor[1] as f64 * TILE_SIZE + offset[1];
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., TILE_SIZE, TILE_SIZE, x, y, TILE_SIZE, TILE_SIZE,
            )?;
        }

        if let Some(cursor) = self.move_cursor {
            let img = &self.assets.img_move_cursor;
            let x = cursor[0] as f64 * TILE_SIZE + offset[0];
            let y = cursor[1] as f64 * TILE_SIZE + offset[1];
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., TILE_SIZE, TILE_SIZE, x, y, TILE_SIZE, TILE_SIZE,
            )?;
        }

        Ok(())
    }
}

fn render_global_task_bar(
    context: &CanvasRenderingContext2d,
    [x, y]: [f64; 2],
    t: f64,
    max_time: f64,
) {
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#7f0000"));
    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#007f00"));
    context.fill_rect(
        x + BAR_MARGIN,
        y + BAR_MARGIN,
        t * BAR_WIDTH / max_time,
        BAR_HEIGHT,
    );
}

struct RenderBar<'a> {
    context: &'a CanvasRenderingContext2d,
    x: f64,
    y: f64,
    v: f64,
    max: f64,
    scale: f64,
    color: &'a str,
}

impl<'a> RenderBar<'a> {
    fn render_bar(&self) {
        let context = self.context;
        let (x, y) = (self.x, self.y);
        context.set_stroke_style(&JsValue::from("#000"));
        context.set_fill_style(&JsValue::from("#7f0000"));
        context.fill_rect(
            x + BAR_MARGIN,
            y + BAR_MARGIN,
            BAR_WIDTH * self.scale,
            BAR_HEIGHT,
        );
        context.set_stroke_style(&JsValue::from("#000"));
        context.set_fill_style(&JsValue::from(self.color));
        context.fill_rect(
            x + BAR_MARGIN,
            y + BAR_MARGIN,
            self.v * BAR_WIDTH * self.scale / self.max,
            BAR_HEIGHT,
        );
    }
}

pub(crate) fn calculate_back_image(tiles: &mut Tiles) {
    let keys: HashSet<Position> = tiles
        .chunks()
        .iter()
        .filter_map(|(pos, chunk)| {
            if matches!(chunk, Chunk::Tiles(_, _)) || has_edge(tiles, pos) {
                Some(*pos)
            } else {
                None
            }
        })
        .collect();
    for pos in keys {
        let image_idxs = calculate_back_image_chunk(tiles, &pos);
        let Some(chunk) = tiles.chunks_mut().get_mut(&pos) else {
            continue;
        };
        match chunk {
            Chunk::Tiles(tiles, _) => {
                for (tile, idx) in tiles.iter_mut().zip(image_idxs.iter()) {
                    tile.image_idx = *idx;
                }
            }
            Chunk::Uniform(tile, _) => {
                let mut tiles = vec![*tile; CHUNK_SIZE * CHUNK_SIZE];
                for (tile, idx) in tiles.iter_mut().zip(image_idxs.iter()) {
                    tile.image_idx = *idx;
                }
                let mut hasher = new_hasher();
                for tile in &tiles {
                    tile.hash(&mut hasher);
                }
                *chunk = Chunk::Tiles(tiles, hasher.finish());
            }
        }
    }
}

const CHUNK_SIZE_I: i32 = CHUNK_SIZE as i32;

/// Determine if the chunk designated by `pos` has an edge that is adjacent to a non-solid edge.
/// It is necessary to non-uniformify this chunk if this function returns true.
fn has_edge(tiles: &Tiles, pos: &Position) -> bool {
    for x in -1..=CHUNK_SIZE_I {
        let top = [x + pos.x * CHUNK_SIZE_I, pos.y * CHUNK_SIZE_I];
        let beyond_top = [x + pos.x * CHUNK_SIZE_I, pos.y * CHUNK_SIZE_I - 1];
        if tiles[top].state != tiles[beyond_top].state {
            return true;
        }
        let bottom = [x + pos.x * CHUNK_SIZE_I, (pos.y + 1) * CHUNK_SIZE_I - 1];
        let beyond_bottom = [x + pos.x * CHUNK_SIZE_I, (pos.y + 1) * CHUNK_SIZE_I];
        if tiles[bottom].state != tiles[beyond_bottom].state {
            return true;
        }
    }
    for y in -1..=CHUNK_SIZE_I {
        let left = [pos.x * CHUNK_SIZE_I, y + pos.y * CHUNK_SIZE_I];
        let beyond_left = [pos.x * CHUNK_SIZE_I - 1, y + pos.y * CHUNK_SIZE_I];
        if tiles[left].state != tiles[beyond_left].state {
            return true;
        }
        let right = [(pos.x + 1) * CHUNK_SIZE_I - 1, y + pos.y * CHUNK_SIZE_I];
        let beyond_bottom = [(pos.x + 1) * CHUNK_SIZE_I, y + pos.y * CHUNK_SIZE_I];
        if tiles[right].state != tiles[beyond_bottom].state {
            return true;
        }
    }
    false
}

#[allow(clippy::many_single_char_names)]
fn calculate_back_image_chunk(tiles: &Tiles, pos: &Position) -> Vec<ImageIdx> {
    let mut ret = vec![ImageIdx::new(); CHUNK_SIZE * CHUNK_SIZE];
    for uy in 0..CHUNK_SIZE {
        let y = uy as i32 + pos.y * CHUNK_SIZE as i32;
        for ux in 0..CHUNK_SIZE {
            let x = ux as i32 + pos.x * CHUNK_SIZE as i32;
            if !matches!(tiles[[x, y]].state, TileState::Solid) {
                continue;
            }
            let get_at = |x: i32, y: i32| {
                let state = tiles[[x, y]].state;
                (
                    !matches!(state, TileState::Solid) as u8,
                    matches!(state, TileState::Space) as u8,
                )
            };
            let l = get_at(x - 1, y);
            let t = get_at(x, y - 1);
            let r = get_at(x + 1, y);
            let b = get_at(x, y + 1);
            let lt = get_at(x - 1, y - 1);
            let rt = get_at(x + 1, y - 1);
            let rb = get_at(x + 1, y + 1);
            let lb = get_at(x - 1, y + 1);

            // Voting filter. If 2 neighboring tiles out of 3 around a corner is space, its background
            // should be space too.
            let vote = |a, b, c| if 2 <= a + b + c { SPACE_BIT } else { 0 };

            // Encode information of neighboring tiles around a quater piece of a tile into a byte.
            // Lower 5 bits (0-31) contains the offset of the image coordinates,
            // and the 6th bit indicates if it's Space (or Empty if unset)
            let tile = &mut ret[ux + uy * CHUNK_SIZE];
            tile.lt = match (l.0, lt.0, t.0) {
                (1, _, 1) => 2,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 2 + 4 * 4,
                (0, 1, 0) => 3 + 3 * 4,
                _ => 0,
            } | vote(l.1, lt.1, t.1);
            tile.lb = match (b.0, lb.0, l.0) {
                (1, _, 1) => 2 + 4,
                (0, _, 1) => 2 + 4 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 3 + 2 * 4,
                _ => 0,
            } | vote(b.1, lb.1, l.1);
            tile.rb = match (b.0, rb.0, r.0) {
                (1, _, 1) => 3 + 4,
                (0, _, 1) => 3 + 5 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 2 + 2 * 4,
                _ => 0,
            } | vote(b.1, rb.1, r.1);
            tile.rt = match (r.0, rt.0, t.0) {
                (1, _, 1) => 3,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 3 + 5 * 4,
                (0, 1, 0) => 2 + 3 * 4,
                _ => 0,
            } | vote(r.1, rt.1, t.1);
        }
    }
    ret
}
