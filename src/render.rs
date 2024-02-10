use super::AsteroidColonies;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{
    task::{
        GlobalTask, Task, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME, EXCAVATE_TIME,
        LABOR_EXCAVATE_TIME, MOVE_ITEM_TIME, MOVE_TIME,
    },
    BuildingType, CellState, WIDTH,
};

const TILE_SIZE: f64 = 32.;
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
                render_bar(context, x, y, t as f64, max_time as f64, "#00af00");
            }

            let inventory_count: usize = building.inventory.iter().map(|item| *item.1).sum();
            if 0 < inventory_count {
                render_bar(
                    context,
                    x,
                    y + TILE_SIZE - BAR_HEIGHT - BAR_MARGIN * 2.,
                    inventory_count as f64,
                    building.type_.capacity() as f64,
                    "#afaf00",
                );
            }
        }

        for task in &self.global_tasks {
            let task_target = match task {
                GlobalTask::BuildPowerGrid(t, pos) => Some((*t, pos, BUILD_POWER_GRID_TIME)),
                GlobalTask::BuildConveyor(t, pos) => Some((*t, pos, BUILD_CONVEYOR_TIME)),
                GlobalTask::BuildBuilding(t, pos, ty) => Some((*t, pos, ty.build_time())),
                GlobalTask::Excavate(t, pos) => Some((*t, pos, LABOR_EXCAVATE_TIME)),
            };

            if let Some((t, pos, max_time)) = task_target {
                render_global_task_bar(context, pos, t, max_time);
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

fn render_bar(context: &CanvasRenderingContext2d, x: f64, y: f64, v: f64, max: f64, color: &str) {
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#7f0000"));
    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from(color));
    context.fill_rect(
        x + BAR_MARGIN,
        y + BAR_MARGIN,
        v * BAR_WIDTH / max,
        BAR_HEIGHT,
    );
}
