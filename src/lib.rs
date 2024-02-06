mod assets;
mod building;
mod render;
mod task;
mod utils;

use assets::Assets;
use task::GlobalTask;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::building::{Building, BuildingType};

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ItemType {
    /// Freshly dug soil from asteroid body. Hardly useful unless refined
    Slug,
    PowerGridComponent,
    ConveyorComponent,
}

impl ItemType {
    const fn build_time(&self) -> usize {
        match self {
            Self::Slug => 0,
            Self::PowerGridComponent => 10,
            Self::ConveyorComponent => 20,
        }
    }
}

const WIDTH: usize = 20;
const HEIGHT: usize = 15;

#[wasm_bindgen]
pub struct AsteroidColonies {
    cells: Vec<Cell>,
    buildings: Vec<Building>,
    assets: Assets,
    global_tasks: Vec<GlobalTask>,
    /// Used power for the last tick, in kW
    used_power: usize,
    global_time: usize,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(image_assets: js_sys::Array) -> Result<AsteroidColonies, JsValue> {
        let mut cells = vec![Cell::new(); WIDTH * HEIGHT];
        let buildings = vec![
            Building::new([2, 2], BuildingType::CrewCabin),
            Building::new([3, 4], BuildingType::Power),
            Building::new([4, 4], BuildingType::Excavator),
            Building::new([3, 5], BuildingType::Storage),
            Building::new([3, 6], BuildingType::Assembler),
        ];
        for building in &buildings {
            let pos = building.pos;
            let size = building.type_.size();
            for iy in 0..size[1] {
                let y = pos[1] as usize + iy;
                for ix in 0..size[0] {
                    let x = pos[0] as usize + ix;
                    cells[x + y * WIDTH] = Cell::building();
                }
            }
        }
        Ok(Self {
            cells,
            buildings,
            assets: Assets::new(image_assets)?,
            global_tasks: vec![],
            used_power: 0,
            global_time: 0,
        })
    }

    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        let intersects = |b: &&Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };
        let building_str = if let Some(building) = self.buildings.iter().find(intersects) {
            format!(
                "{} at {}, {}\nTask: {:?}\nInventory: {:?}\nCrews: {} / {}",
                building.type_,
                building.pos[0],
                building.pos[1],
                building.task,
                building.inventory,
                building.crews,
                building.type_.max_crews()
            )
        } else {
            format!("Empty at {ix}, {iy}")
        };
        let crew_str = format!(
            "\nTotal crew: {}",
            self.buildings.iter().map(|b| b.crews).sum::<usize>()
        );
        // We want to count power generation and consumption separately
        let power_capacity_str = format!(
            "\nTotal power capacity: {} kW",
            self.buildings
                .iter()
                .map(|b| b.power().max(0))
                .sum::<isize>()
        );
        let power_consumed = self
            .buildings
            .iter()
            .map(|b| b.power().min(0))
            .sum::<isize>()
            .abs() as usize;
        let power_used_str = format!("\nUsed power: {} kW", self.used_power + power_consumed);
        Ok(JsValue::from(
            building_str + &crew_str + &power_capacity_str + &power_used_str,
        ))
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
            "power" => self.build_power_grid(ix, iy),
            "conveyor" => self.conveyor(ix, iy),
            "moveItem" => self.move_item(ix, iy),
            "buildPowerPlant" => self.build_building(ix, iy, BuildingType::Power),
            "buildStorage" => self.build_building(ix, iy, BuildingType::Storage),
            "assemble" => self.assemble(ix, iy, ItemType::PowerGridComponent),
            "assembleConveyor" => self.assemble(ix, iy, ItemType::ConveyorComponent),
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

        let mut workforce: usize = self.buildings.iter().map(|b| b.crews).sum();
        let power_cap: isize = self.buildings.iter().map(|b| b.power()).sum();
        let mut power = power_cap;

        for task in &self.global_tasks {
            match task {
                GlobalTask::BuildPowerGrid(0, pos) => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].power_grid = true;
                }
                GlobalTask::BuildConveyor(0, pos) => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].conveyor = true;
                }
                GlobalTask::BuildBuilding(0, pos, type_) => {
                    self.buildings.push(Building::new(*pos, *type_));
                }
                _ => {}
            }
        }

        const POWER_CONSUMPTION: usize = 200;

        self.global_tasks.retain_mut(|task| match task {
            GlobalTask::BuildPowerGrid(ref mut t, _)
            | GlobalTask::BuildConveyor(ref mut t, _)
            | GlobalTask::BuildBuilding(ref mut t, _, _) => {
                if *t == 0 {
                    false
                } else {
                    if 0 < workforce && POWER_CONSUMPTION as isize <= power {
                        *t -= 1;
                        power -= POWER_CONSUMPTION as isize;
                        workforce -= 1;
                    }
                    true
                }
            }
        });

        self.used_power = (power_cap - power) as usize;

        self.global_time += 1;

        Ok(())
    }
}
