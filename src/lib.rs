mod assets;
mod task;
mod utils;

use std::{collections::HashMap, fmt::Display};

use assets::Assets;
use task::GlobalTask;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, CanvasRenderingContext2d};

use crate::task::{Task, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME, EXCAVATE_TIME, MOVE_TIME};

#[macro_export]
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[derive(Clone, Copy)]
enum CellState {
    Solid,
    Empty,
}

#[derive(Clone, Copy)]
struct Cell {
    state: CellState,
    power_grid: bool,
    conveyor: bool,
}

impl Cell {
    fn new() -> Self {
        Self {
            state: CellState::Solid,
            power_grid: false,
            conveyor: false,
        }
    }

    fn building() -> Self {
        Self {
            state: CellState::Empty,
            power_grid: true,
            conveyor: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuildingType {
    Power,
    Excavator,
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ItemType {
    /// Freshly dug soil from asteroid body. Hardly useful unless refined
    Slug,
}

struct Building {
    pos: [i32; 2],
    type_: BuildingType,
    task: Task,
    inventory: HashMap<ItemType, f64>,
}

const WIDTH: usize = 20;
const HEIGHT: usize = 15;

#[wasm_bindgen]
pub struct AsteroidColonies {
    cells: Vec<Cell>,
    buildings: Vec<Building>,
    assets: Assets,
    global_tasks: Vec<GlobalTask>,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(image_assets: js_sys::Array) -> Result<AsteroidColonies, JsValue> {
        let mut cells = vec![Cell::new(); WIDTH * HEIGHT];
        let buildings = vec![
            Building {
                pos: [3, 4],
                type_: BuildingType::Power,
                task: Task::None,
                inventory: HashMap::new(),
            },
            Building {
                pos: [4, 4],
                type_: BuildingType::Excavator,
                task: Task::None,
                inventory: HashMap::new(),
            },
        ];
        for building in &buildings {
            let pos = building.pos;
            cells[pos[0] as usize + pos[1] as usize * WIDTH] = Cell::building();
        }
        Ok(Self {
            cells,
            buildings,
            assets: Assets::new(image_assets)?,
            global_tasks: vec![],
        })
    }

    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        const TILE_SIZE: f64 = 32.;
        const BAR_MARGIN: f64 = 4.;
        const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
        const BAR_HEIGHT: f64 = 6.;

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
            };
            let x = building.pos[0] as f64 * TILE_SIZE;
            let y = building.pos[1] as f64 * TILE_SIZE;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., TILE_SIZE, TILE_SIZE, x, y, TILE_SIZE, TILE_SIZE,
            )?;
            match building.task {
                Task::Excavate(t, _) | Task::Move(t, _) => {
                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#7f0000"));
                    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#007f00"));
                    let max_time = match building.task {
                        Task::Excavate(_, _) => EXCAVATE_TIME,
                        Task::Move(_, _) => MOVE_TIME,
                        _ => unreachable!(),
                    };
                    context.fill_rect(
                        x + BAR_MARGIN,
                        y + BAR_MARGIN,
                        t as f64 * BAR_WIDTH / max_time as f64,
                        BAR_HEIGHT,
                    );
                }
                _ => {}
            }
        }

        for task in &self.global_tasks {
            match task {
                GlobalTask::BuildPowerGrid(t, pos) | GlobalTask::BuildConveyor(t, pos) => {
                    let x = pos[0] as f64 * TILE_SIZE;
                    let y = pos[1] as f64 * TILE_SIZE;

                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#7f0000"));
                    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#007f00"));
                    let max_time = match task {
                        GlobalTask::BuildPowerGrid(_, _) => BUILD_POWER_GRID_TIME,
                        GlobalTask::BuildConveyor(_, _) => BUILD_CONVEYOR_TIME,
                    };
                    context.fill_rect(
                        x + BAR_MARGIN,
                        y + BAR_MARGIN,
                        *t as f64 * BAR_WIDTH / max_time as f64,
                        BAR_HEIGHT,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if let Some(building) = self
            .buildings
            .iter()
            .find(|b| b.pos[0] == ix && b.pos[1] == iy)
        {
            Ok(JsValue::from(format!(
                "{} at {}, {}\nTask: {:?}\nInventory: {:?}",
                building.type_, building.pos[0], building.pos[1], building.task, building.inventory
            )))
        } else {
            Ok(JsValue::from(format!("Empty at {ix}, {iy}")))
        }
    }

    pub fn command(&mut self, com: &str, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        match com {
            "excavate" => self.excavate(ix, iy),
            "move" => self.move_(ix, iy),
            "power" => self.power(ix, iy),
            "conveyor" => self.conveyor(ix, iy),
            _ => Err(JsValue::from(format!("Unknown command: {}", com))),
        }
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        for building in &mut self.buildings {
            Self::process_task(&mut self.cells, building);
        }

        for task in &self.global_tasks {
            match task {
                GlobalTask::BuildPowerGrid(0, pos) => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].power_grid = true;
                }
                GlobalTask::BuildConveyor(0, pos) => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].conveyor = true;
                }
                _ => {}
            }
        }

        self.global_tasks.retain_mut(|task| match task {
            GlobalTask::BuildPowerGrid(ref mut t, _) | GlobalTask::BuildConveyor(ref mut t, _) => {
                if *t == 0 {
                    false
                } else {
                    *t -= 1;
                    true
                }
            }
        });

        Ok(())
    }
}
