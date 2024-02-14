use super::AsteroidColonies;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{
    console_log,
    task::{
        GlobalTask, Task, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME, EXCAVATE_TIME,
        LABOR_EXCAVATE_TIME, MOVE_ITEM_TIME, MOVE_TIME,
    },
    BuildingType, Cell, CellState, ItemType, HEIGHT, WIDTH,
};

pub(crate) const TILE_SIZE: f64 = 32.;
const ITEM_SIZE: f64 = 16.;
const BAR_MARGIN: f64 = 4.;
const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
const BAR_HEIGHT: f64 = 6.;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#000000"));
        context.clear_rect(0., 0., WIDTH as f64 * TILE_SIZE, HEIGHT as f64 * TILE_SIZE);
        context.set_fill_style(&JsValue::from("#ff0000"));
        let vp = &self.viewport;
        let mut rendered_cells = 0;
        let mut render_cell = |ix: i32, iy: i32| -> Result<(), JsValue> {
            if ix < 0 || (WIDTH as i32) < ix || iy < 0 || (HEIGHT as i32) < iy {
                return Ok(());
            }
            let cell = &self.cells[ix as usize + iy as usize * WIDTH];
            // let iy = i / WIDTH;
            let y = iy as f64 * TILE_SIZE + self.viewport.offset[1];
            // let ix = i % WIDTH;
            let x = ix as f64 * TILE_SIZE + self.viewport.offset[0];
            let (sx, sy) = match cell.state {
                CellState::Empty => (3. * TILE_SIZE, 3. * TILE_SIZE),
                CellState::Solid => (0., 0.),
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

            let mut render_quarter_tile = |image, xofs, yofs| -> Result<(), JsValue> {
                let srcx = image % 4;
                let srcy = image / 4;
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
                rendered_cells += 1;
                Ok(())
            };

            if cell.image_lt != 0 {
                render_quarter_tile(cell.image_lt, 0., 0.)?;
            }
            if cell.image_lb != 0 {
                render_quarter_tile(cell.image_lb, 0., TILE_SIZE / 2.)?;
            }
            if cell.image_rb != 0 {
                render_quarter_tile(cell.image_rb, TILE_SIZE / 2., TILE_SIZE / 2.)?;
            }
            if cell.image_rt != 0 {
                render_quarter_tile(cell.image_rt, TILE_SIZE / 2., 0.)?;
            }

            if cell.power_grid {
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_power_grid,
                        0.,
                        0.,
                        TILE_SIZE,
                        TILE_SIZE,
                        x,
                        y,
                        TILE_SIZE,
                        TILE_SIZE,
                    )?;
            }
            if cell.conveyor {
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_conveyor,
                        0.,
                        0.,
                        TILE_SIZE,
                        TILE_SIZE,
                        x,
                        y,
                        TILE_SIZE,
                        TILE_SIZE,
                    )?;
            }
            rendered_cells += 1;
            Ok(())
        };

        let ymin = ((-vp.offset[1]).div_euclid(TILE_SIZE)).max(0.) as i32;
        let ymax = ((-vp.offset[1] + vp.size[1] + TILE_SIZE).div_euclid(TILE_SIZE) as i32)
            .min(HEIGHT as i32);
        let xmin = ((-vp.offset[0]).div_euclid(TILE_SIZE)).max(0.) as i32;
        let xmax = ((-vp.offset[0] + vp.size[0] + TILE_SIZE).div_euclid(TILE_SIZE) as i32)
            .min(WIDTH as i32);
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                render_cell(ix, iy)?;
            }
        }
        console_log!("rendered_cells: {}", rendered_cells);

        for building in &self.buildings {
            let img = match building.type_ {
                BuildingType::Power => &self.assets.img_power,
                BuildingType::Excavator => &self.assets.img_excavator,
                BuildingType::Storage => &self.assets.img_storage,
                BuildingType::MediumStorage => &self.assets.img_medium_storage,
                BuildingType::CrewCabin => &self.assets.img_crew_cabin,
                BuildingType::Assembler => &self.assets.img_assembler,
                BuildingType::Furnace => &self.assets.img_furnace,
            };
            let (sx, sy) = match building.type_ {
                BuildingType::Assembler => {
                    if !matches!(building.task, Task::None) {
                        ((self.global_time % 4) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                BuildingType::Furnace => {
                    if !matches!(building.task, Task::None) {
                        ((self.global_time % 2 + 1) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                _ => (0., 0.),
            };
            let x = building.pos[0] as f64 * TILE_SIZE + vp.offset[0];
            let y = building.pos[1] as f64 * TILE_SIZE + vp.offset[1];
            let size = building.type_.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, sx, sy, width, height, x, y, width, height,
            )?;

            let task_target = match building.task {
                Task::Excavate(t, _) => Some((t, EXCAVATE_TIME)),
                Task::Move(t, _) => Some((t, MOVE_TIME)),
                Task::MoveItem { t, .. } => Some((t, MOVE_ITEM_TIME)),
                Task::Assemble { t, max_t, .. } => Some((t, max_t)),
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

            if let Task::Move(_, path) = &building.task {
                context.set_stroke_style(&JsValue::from("#ff7f00"));
                context.set_line_width(3.);
                context.begin_path();
                for node in std::iter::once(&building.pos).chain(path.iter()) {
                    context.line_to(
                        (node[0] as f64 + 0.5) * TILE_SIZE + vp.offset[0],
                        (node[1] as f64 + 0.5) * TILE_SIZE + vp.offset[1],
                    );
                }
                context.stroke();
            }
        }

        for task in &self.global_tasks {
            let task_target = match task {
                GlobalTask::BuildPowerGrid(t, pos) => Some((*t, pos, BUILD_POWER_GRID_TIME)),
                GlobalTask::BuildConveyor(t, pos) => Some((*t, pos, BUILD_CONVEYOR_TIME)),
                GlobalTask::BuildBuilding(t, pos, recipe) => Some((*t, pos, recipe.time)),
                GlobalTask::Excavate(t, pos) => Some((*t, pos, LABOR_EXCAVATE_TIME)),
            };

            if let Some((t, pos, max_time)) = task_target {
                let x = pos[0] as f64 * TILE_SIZE + vp.offset[0];
                let y = pos[1] as f64 * TILE_SIZE + vp.offset[1];
                render_global_task_bar(context, [x, y], t, max_time);
            }
        }

        for construction in &self.constructions {
            let img = &self.assets.img_construction;
            let x = construction.pos[0] as f64 * TILE_SIZE + vp.offset[0];
            let y = construction.pos[1] as f64 * TILE_SIZE + vp.offset[1];
            let size = construction.type_.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            const SRC_WIDTH: f64 = 64.;
            const SRC_HEIGHT: f64 = 64.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., SRC_WIDTH, SRC_HEIGHT, x, y, width, height,
            )?;
            // if let Some((t, pos, max_time)) = task_target {
            //     render_global_task_bar(context, pos, t, max_time);
            // }
        }

        for t in &self.transports {
            context.set_stroke_style(&JsValue::from("#ffff00"));
            context.set_line_width(2.);
            context.begin_path();
            for node in &t.path {
                context.line_to(
                    (node[0] as f64 + 0.5) * TILE_SIZE,
                    (node[1] as f64 + 0.5) * TILE_SIZE,
                );
            }
            context.stroke();
            if let Some(pos) = t.path.last() {
                let (img, sw, sh) = match t.item {
                    ItemType::RawOre => (&self.assets.img_raw_ore, 16., 16.),
                    ItemType::IronIngot => (&self.assets.img_iron_ingot, 16., 16.),
                    ItemType::CopperIngot => (&self.assets.img_copper_ingot, 16., 16.),
                    ItemType::Cilicate => (&self.assets.img_cilicate, 16., 16.),
                    ItemType::Gear => (&self.assets.img_gear, 32., 32.),
                    ItemType::Wire => (&self.assets.img_wire, 32., 32.),
                    ItemType::Circuit => (&self.assets.img_circuit, 32., 32.),
                    ItemType::PowerGridComponent => (&self.assets.img_power_grid, 32., 32.),
                    ItemType::ConveyorComponent => (&self.assets.img_conveyor, 32., 32.),
                    ItemType::AssemblerComponent => (&self.assets.img_assembler, 32., 32.),
                };
                let offset = (TILE_SIZE as f64 - ITEM_SIZE as f64) / 2.;
                let x = pos[0] as f64 * TILE_SIZE + offset + vp.offset[0];
                let y = pos[1] as f64 * TILE_SIZE + offset + vp.offset[1];
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, 0., 0., sw, sh, x, y, ITEM_SIZE, ITEM_SIZE,
                    )?;
            }
        }

        if let Some(cursor) = self.cursor {
            let img = &self.assets.img_cursor;
            let x = cursor[0] as f64 * TILE_SIZE + vp.offset[0];
            let y = cursor[1] as f64 * TILE_SIZE + vp.offset[1];
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

#[allow(clippy::many_single_char_names)]
pub(crate) fn calculate_back_image(ret: &mut [Cell]) {
    for uy in 0..HEIGHT {
        let y = uy as i32;
        for ux in 0..WIDTH {
            let x = ux as i32;
            if matches!(ret[(ux + uy * WIDTH) as usize].state, CellState::Empty) {
                let cell = &mut ret[(ux + uy * WIDTH) as usize];
                cell.image_lt = 8;
                cell.image_lb = 8;
                cell.image_rb = 8;
                cell.image_rt = 8;
                continue;
            }
            let get_at = |x: i32, y: i32| {
                if x < 0 || WIDTH as i32 <= x || y < 0 || HEIGHT as i32 <= y {
                    false
                } else {
                    matches!(ret[x as usize + y as usize * WIDTH].state, CellState::Empty)
                }
            };
            let l = get_at(x - 1, y) as u8;
            let t = get_at(x, y - 1) as u8;
            let r = get_at(x + 1, y) as u8;
            let b = get_at(x, y + 1) as u8;
            let lt = get_at(x - 1, y - 1) as u8;
            let rt = get_at(x + 1, y - 1) as u8;
            let rb = get_at(x + 1, y + 1) as u8;
            let lb = get_at(x - 1, y + 1) as u8;
            let cell = &mut ret[(ux + uy * WIDTH) as usize];
            cell.image_lt = match (l, lt, t) {
                (1, _, 1) => 2,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 2 + 4 * 4,
                (0, 1, 0) => 3 + 3 * 4,
                _ => 0,
            };
            cell.image_lb = match (b, lb, l) {
                (1, _, 1) => 2 + 4,
                (0, _, 1) => 2 + 4 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 3 + 2 * 4,
                _ => 0,
            };
            cell.image_rb = match (b, rb, r) {
                (1, _, 1) => 3 + 4,
                (0, _, 1) => 3 + 5 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 2 + 2 * 4,
                _ => 0,
            };
            cell.image_rt = match (r, rt, t) {
                (1, _, 1) => 3,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 3 + 5 * 4,
                (0, 1, 0) => 2 + 3 * 4,
                _ => 0,
            };
        }
    }
}
