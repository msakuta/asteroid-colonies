mod assets;
mod building;
mod construction;
mod info;
mod render;
mod task;
mod transport;
mod utils;

use construction::get_build_menu;
use render::TILE_SIZE;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::{
    assets::Assets,
    building::{Building, BuildingType, Recipe},
    construction::Construction,
    render::TILE_SIZE_I,
    task::GlobalTask,
    transport::Transport,
};

#[macro_export]
macro_rules! hash_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
    { } => {
        ::std::collections::HashMap::new()
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize)]
enum ItemType {
    /// Freshly dug soil from asteroid body. Hardly useful unless refined
    RawOre,
    IronIngot,
    CopperIngot,
    Cilicate,
    Gear,
    Wire,
    Circuit,
    PowerGridComponent,
    ConveyorComponent,
    AssemblerComponent,
}

const WIDTH: usize = 20;
const HEIGHT: usize = 15;

static RECIPES: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
fn recipes() -> &'static [Recipe] {
    RECIPES.get_or_init(|| {
        vec![
            Recipe {
                inputs: hash_map!(ItemType::Wire => 1, ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::PowerGridComponent => 1),
                time: 100.,
            },
            Recipe {
                inputs: hash_map!(ItemType::IronIngot => 2),
                outputs: hash_map!(ItemType::ConveyorComponent => 1),
                time: 120.,
            },
            Recipe {
                inputs: hash_map!(ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::Gear => 2),
                time: 70.,
            },
            Recipe {
                inputs: hash_map!(ItemType::CopperIngot => 1),
                outputs: hash_map!(ItemType::Wire => 2),
                time: 50.,
            },
            Recipe {
                inputs: hash_map!(ItemType::Wire => 1, ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::Circuit => 1),
                time: 120.,
            },
            Recipe {
                inputs: hash_map!(ItemType::Gear => 2, ItemType::Circuit => 2),
                outputs: hash_map!(ItemType::AssemblerComponent => 1),
                time: 200.,
            },
        ]
    })
}

type Pos = [i32; 2];

#[wasm_bindgen]
pub struct AsteroidColonies {
    cursor: Option<Pos>,
    cells: Vec<Cell>,
    buildings: Vec<Building>,
    assets: Assets,
    global_tasks: Vec<GlobalTask>,
    /// Used power for the last tick, in kW
    used_power: usize,
    global_time: usize,
    transports: Vec<Transport>,
    constructions: Vec<Construction>,
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
            Building::new([5, 4], BuildingType::Storage),
            Building::new_inventory(
                [6, 3],
                BuildingType::MediumStorage,
                hash_map!(ItemType::ConveyorComponent => 2, ItemType::PowerGridComponent => 2),
            ),
            Building::new([1, 10], BuildingType::Assembler),
            Building::new([1, 5], BuildingType::Furnace),
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
        for [x, y] in [[1, 7], [1, 8], [1, 9], [4, 4], [4, 5], [4, 6]] {
            cells[x + y * WIDTH].state = CellState::Empty;
            cells[x + y * WIDTH].conveyor = true;
        }
        Ok(Self {
            cursor: None,
            cells,
            buildings,
            assets: Assets::new(image_assets)?,
            global_tasks: vec![],
            used_power: 0,
            global_time: 0,
            transports: vec![],
            constructions: vec![],
        })
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) {
        let ix = x.div_euclid(TILE_SIZE_I);
        let iy = y.div_euclid(TILE_SIZE_I);
        self.cursor = Some([ix, iy]);
    }

    pub fn command(&mut self, com: &str, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(TILE_SIZE_I);
        let iy = y.div_euclid(TILE_SIZE_I);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        match com {
            "excavate" => self.excavate(ix, iy),
            "move" => self.move_(ix, iy),
            "power" => self.build_power_grid(ix, iy),
            "conveyor" => self.conveyor(ix, iy),
            "moveItem" => self.move_item(ix, iy),
            _ => Err(JsValue::from(format!("Unknown command: {}", com))),
        }
    }

    pub fn build(&mut self, x: i32, y: i32, type_: JsValue) -> Result<(), JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building"));
        }
        if !cell.power_grid {
            return Err(JsValue::from("Power grid is required to build"));
        }
        if !cell.conveyor {
            return Err(JsValue::from(
                "Conveyor infrastructure is required to build",
            ));
        }

        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        if self.buildings.iter().any(intersects) {
            return Err(JsValue::from(
                "The destination is already occupied by a building",
            ));
        }

        let type_ = serde_wasm_bindgen::from_value(type_)?;
        if let Some(build) = get_build_menu().iter().find(|it| it.type_ == type_) {
            self.constructions.push(Construction::new(build, [ix, iy]));
            // self.build_building(ix, iy, type_)?;
        }
        Ok(())
    }

    pub fn get_recipes(&self, x: i32, y: i32) -> Result<Vec<JsValue>, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        let Some(assembler) = self.buildings.iter().find(|b| intersects(*b)) else {
            return Err(JsValue::from("The building does not exist at the target"));
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(JsValue::from("The building is not an assembler"));
        }
        recipes()
            .iter()
            .map(|recipe| serde_wasm_bindgen::to_value(recipe))
            .collect::<Result<_, _>>()
            .map_err(JsValue::from)
    }

    pub fn set_recipe(&mut self, x: i32, y: i32, name: &str) -> Result<(), JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        let Some(assembler) = self.buildings.iter().find(|b| intersects(*b)) else {
            return Err(JsValue::from("The building does not exist at the target"));
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(JsValue::from("The building is not an assembler"));
        }
        for recipe in recipes() {
            let Some((key, _)) = recipe.outputs.iter().next() else {
                continue;
            };
            if format!("{:?}", key) == name {
                self.set_building_recipe(ix, iy, recipe)?;
            }
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        let power_demand = self
            .buildings
            .iter()
            .map(|b| b.power().min(0).abs() as usize)
            .sum::<usize>();
        let power_supply = self
            .buildings
            .iter()
            .map(|b| b.power().max(0).abs() as usize)
            .sum::<usize>();
        // let power_load = (power_demand as f64 / power_supply as f64).min(1.);
        let power_ratio = (power_supply as f64 / power_demand as f64).min(1.);
        // A buffer to avoid borrow checker
        let mut moving_items = vec![];
        for i in 0..self.buildings.len() {
            let res = Building::tick(&mut self.buildings, i, &self.cells, &mut self.transports);
            if let Err(e) = res {
                console_log!("Building::tick error: {}", e);
            };
        }
        for building in &mut self.buildings {
            if let Some((item, dest)) = Self::process_task(&mut self.cells, building, power_ratio) {
                moving_items.push((item, dest));
            }
        }

        for (item, item_pos) in moving_items {
            let found = self.buildings.iter_mut().find(|b| b.pos == item_pos);
            if let Some(found) = found {
                *found.inventory.entry(item).or_default() += 1;
            }
        }

        self.process_global_tasks();
        self.process_transports();
        self.process_constructions();

        self.global_time += 1;

        Ok(())
    }
}
