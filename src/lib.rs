mod assets;
mod render;
mod task;
mod utils;

use std::{collections::HashMap, fmt::Display};

use assets::Assets;
use task::GlobalTask;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::task::Task;

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
    Storage,
}

impl BuildingType {
    fn capacity(&self) -> usize {
        match self {
            Self::Power => 3,
            Self::Excavator => 3,
            Self::Storage => 10,
        }
    }
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
            Self::Storage => write!(f, "Storage"),
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
    inventory: HashMap<ItemType, usize>,
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
            Building {
                pos: [3, 5],
                type_: BuildingType::Storage,
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
            "moveItem" => self.move_item(ix, iy),
            _ => Err(JsValue::from(format!("Unknown command: {}", com))),
        }
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        // A buffer to avoid borrow checker
        let mut moving_items = vec![];
        for building in &mut self.buildings {
            if let Some((item, dest)) = Self::process_task(&mut self.cells, building) {
                moving_items.push((item, dest));
            }
        }

        for (item, item_pos) in moving_items {
            let found = self.buildings.iter_mut().find(|b| b.pos == item_pos);
            if let Some(found) = found {
                *found.inventory.entry(item).or_default() += 1;
            }
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
