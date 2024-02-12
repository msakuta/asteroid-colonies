use super::AsteroidColonies;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{
    task::{
        GlobalTask, Task, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME, EXCAVATE_TIME,
        LABOR_EXCAVATE_TIME, MOVE_ITEM_TIME, MOVE_TIME,
    },
    BuildingType, CellState, ItemType, WIDTH,
};

const TILE_SIZE: f64 = 32.;
const ITEM_SIZE: f64 = 16.;
const BAR_MARGIN: f64 = 4.;
const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
const BAR_HEIGHT: f64 = 6.;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#ff0000"));
        for (i, cell) in self.cells.iter().enumerate() {
            let iy = i / WIDTH;
            let y = iy as f64 * TILE_SIZE;
            let ix = i % WIDTH;
            let x = ix as f64 * TILE_SIZE;
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
        }

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
            let x = building.pos[0] as f64 * TILE_SIZE;
            let y = building.pos[1] as f64 * TILE_SIZE;
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

            if let Some(path) = &building.output_path {
                context.set_stroke_style(&JsValue::from("#ffff00"));
                context.set_line_width(2.);
                context.begin_path();
                for node in path {
                    context.line_to(
                        (node[0] as f64 + 0.5) * TILE_SIZE,
                        (node[1] as f64 + 0.5) * TILE_SIZE,
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
                render_global_task_bar(context, pos, t, max_time);
            }
        }

        for construction in &self.constructions {
            let img = &self.assets.img_construction;
            let x = construction.pos[0] as f64 * TILE_SIZE;
            let y = construction.pos[1] as f64 * TILE_SIZE;
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
                    ItemType::Gear => (&self.assets.img_gear, 32., 32.),
                    ItemType::Wire => (&self.assets.img_wire, 32., 32.),
                    ItemType::Circuit => (&self.assets.img_circuit, 32., 32.),
                    ItemType::PowerGridComponent => (&self.assets.img_power_grid, 32., 32.),
                    ItemType::ConveyorComponent => (&self.assets.img_conveyor, 32., 32.),
                    ItemType::AssemblerComponent => (&self.assets.img_assembler, 32., 32.),
                };
                let offset = (TILE_SIZE as f64 - ITEM_SIZE as f64) / 2.;
                let x = pos[0] as f64 * TILE_SIZE + offset;
                let y = pos[1] as f64 * TILE_SIZE + offset;
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, 0., 0., sw, sh, x, y, ITEM_SIZE, ITEM_SIZE,
                    )?;
            }
        }

        Ok(())
    }
}

fn render_global_task_bar(
    context: &CanvasRenderingContext2d,
    pos: &[i32; 2],
    t: usize,
    max_time: usize,
) {
    let x = pos[0] as f64 * TILE_SIZE;
    let y = pos[1] as f64 * TILE_SIZE;

    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#7f0000"));
    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#007f00"));
    context.fill_rect(
        x + BAR_MARGIN,
        y + BAR_MARGIN,
        t as f64 * BAR_WIDTH / max_time as f64,
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
