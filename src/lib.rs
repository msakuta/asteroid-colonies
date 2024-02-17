mod assets;
mod building;
mod construction;
mod crew;
mod info;
mod render;
mod task;
mod transport;
mod utils;

use construction::get_build_menu;

use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::{
    assets::Assets,
    building::{Building, BuildingType, Recipe},
    construction::{Construction, ConstructionType},
    crew::Crew,
    render::{calculate_back_image, TILE_SIZE},
    task::{GlobalTask, Task, MOVE_TIME},
    transport::{find_path, Transport},
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
    Space,
}

#[derive(Clone, Copy)]
struct Cell {
    state: CellState,
    power_grid: bool,
    conveyor: bool,
    /// The index into the background image for quick rendering
    image_lt: u8,
    image_lb: u8,
    image_rb: u8,
    image_rt: u8,
}

impl Cell {
    fn new() -> Self {
        Self {
            state: CellState::Solid,
            power_grid: false,
            conveyor: false,
            image_lt: 0,
            image_lb: 0,
            image_rb: 0,
            image_rt: 0,
        }
    }

    fn building() -> Self {
        Self {
            state: CellState::Empty,
            power_grid: true,
            conveyor: true,
            image_lt: 8,
            image_lb: 8,
            image_rb: 8,
            image_rt: 8,
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

const WIDTH: usize = 50;
const HEIGHT: usize = 50;

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

struct Viewport {
    /// View offset in pixels
    offset: [f64; 2],
    /// Viewport size in pixels
    size: [f64; 2],
}

#[wasm_bindgen]
pub struct AsteroidColonies {
    cursor: Option<Pos>,
    cells: Vec<Cell>,
    buildings: Vec<Building>,
    crews: Vec<Crew>,
    assets: Assets,
    global_tasks: Vec<GlobalTask>,
    /// Used power for the last tick, in kW
    used_power: usize,
    global_time: usize,
    transports: Vec<Transport>,
    constructions: Vec<Construction>,
    viewport: Viewport,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(
        image_assets: js_sys::Array,
        vp_width: f64,
        vp_height: f64,
    ) -> Result<AsteroidColonies, JsValue> {
        let mut cells = vec![Cell::new(); WIDTH * HEIGHT];
        let r2_thresh = (WIDTH as f64 * 3. / 8.).powi(2);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let r2 = ((x as f64 - WIDTH as f64 / 2.) as f64).powi(2)
                    + ((y as f64 - HEIGHT as f64 / 2.) as f64).powi(2);
                if r2_thresh < r2 {
                    cells[x + y * WIDTH].state = CellState::Space;
                }
            }
        }
        let start_ofs = |pos: [i32; 2]| [pos[0] + 8, pos[1] + 20];
        let buildings = vec![
            Building::new(start_ofs([2, 2]), BuildingType::CrewCabin),
            Building::new(start_ofs([3, 4]), BuildingType::Power),
            Building::new(start_ofs([4, 4]), BuildingType::Excavator),
            Building::new(start_ofs([5, 4]), BuildingType::Storage),
            Building::new_inventory(
                start_ofs([6, 3]),
                BuildingType::MediumStorage,
                hash_map!(ItemType::ConveyorComponent => 2, ItemType::PowerGridComponent => 2),
            ),
            Building::new(start_ofs([1, 10]), BuildingType::Assembler),
            Building::new(start_ofs([1, 5]), BuildingType::Furnace),
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
        for pos in [[1, 7], [1, 8], [1, 9], [4, 4], [4, 5], [4, 6]] {
            let [x, y] = start_ofs(pos);
            let [x, y] = [x as usize, y as usize];
            cells[x + y * WIDTH].state = CellState::Empty;
            cells[x + y * WIDTH].conveyor = true;
            cells[x + y * WIDTH].power_grid = true;
        }
        for pos in [[4, 7], [4, 8]] {
            let [x, y] = start_ofs(pos);
            let [x, y] = [x as usize, y as usize];
            cells[x + y * WIDTH].state = CellState::Empty;
        }
        calculate_back_image(&mut cells);
        Ok(Self {
            cursor: None,
            cells,
            buildings,
            crews: vec![],
            assets: Assets::new(image_assets)?,
            global_tasks: vec![],
            used_power: 0,
            global_time: 0,
            transports: vec![],
            constructions: vec![],
            viewport: Viewport {
                offset: [0.; 2],
                size: [vp_width, vp_height],
            },
        })
    }

    pub fn set_size(&mut self, sx: f64, sy: f64) {
        self.viewport.size = [sx, sy];
    }

    pub fn set_cursor(&mut self, x: f64, y: f64) {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.cursor = Some([ix, iy]);
    }

    pub fn command(&mut self, com: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        match com {
            "excavate" => self.excavate(ix, iy),
            "power" => self.build_power_grid(ix, iy),
            "conveyor" => self.conveyor(ix, iy),
            "moveItem" => self.move_item(ix, iy),
            _ => Err(JsValue::from(format!("Unknown command: {}", com))),
        }
    }

    pub fn move_building(
        &mut self,
        src_x: f64,
        src_y: f64,
        dst_x: f64,
        dst_y: f64,
    ) -> Result<(), JsValue> {
        let ix = (src_x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (src_y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let dx = (dst_x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let dy = (dst_y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let Some(building) = self.buildings.iter_mut().find(|b| b.pos == [ix, iy]) else {
            return Err(JsValue::from("Building does not exist at that position"));
        };
        if !building.type_.is_mobile() {
            return Err(JsValue::from("Building at that position is not mobile"));
        }
        if !matches!(building.task, Task::None) {
            return Err(JsValue::from(
                "The building is busy; wait for the building to finish the current task",
            ));
        }
        let cells = &self.cells;
        let buildings = &self.buildings;

        let intersects = |pos: [i32; 2]| {
            buildings.iter().any(|b| {
                let size = b.type_.size();
                b.pos[0] <= pos[0]
                    && pos[0] < size[0] as i32 + b.pos[0]
                    && b.pos[1] <= pos[1]
                    && pos[1] < size[1] as i32 + b.pos[1]
            })
        };

        let mut path = find_path([ix, iy], [dx, dy], |pos| {
            let cell = &cells[pos[0] as usize + pos[1] as usize * WIDTH];
            !intersects(pos) && matches!(cell.state, CellState::Empty) && cell.power_grid
        })
        .ok_or_else(|| JsValue::from("Failed to find the path"))?;

        // Re-borrow to avoid borrow checker
        let Some(building) = self.buildings.iter_mut().find(|b| b.pos == [ix, iy]) else {
            return Err(JsValue::from("Building does not exist at that position"));
        };
        path.pop();
        building.task = Task::Move(MOVE_TIME, path);
        Ok(())
    }

    pub fn build(&mut self, x: f64, y: f64, type_: JsValue) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }

        let type_: BuildingType = serde_wasm_bindgen::from_value(type_)?;
        let size = type_.size();
        for jy in iy..iy + size[1] as i32 {
            for jx in ix..ix + size[0] as i32 {
                let cell = &self.cells[jx as usize + jy as usize * WIDTH];
                if matches!(cell.state, CellState::Solid) {
                    return Err(JsValue::from("Needs excavation before building"));
                }
                if matches!(cell.state, CellState::Space) {
                    return Err(JsValue::from("You cannot build in space!"));
                }
            }
        }

        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if !cell.power_grid {
            return Err(JsValue::from("Power grid is required to build"));
        }
        if !cell.conveyor {
            return Err(JsValue::from(
                "Conveyor infrastructure is required to build",
            ));
        }

        let intersects = |pos: Pos, o_size: [usize; 2]| {
            pos[0] < ix + size[0] as i32
                && ix < o_size[0] as i32 + pos[0]
                && pos[1] < iy + size[1] as i32
                && iy < o_size[1] as i32 + pos[1]
        };

        if self
            .buildings
            .iter()
            .any(|b| intersects(b.pos, b.type_.size()))
        {
            return Err(JsValue::from(
                "The destination is already occupied by a building",
            ));
        }

        if self
            .constructions
            .iter()
            .any(|c| intersects(c.pos, c.size()))
        {
            return Err(JsValue::from(
                "The destination is already occupied by a construction plan",
            ));
        }

        if let Some(build) = get_build_menu()
            .iter()
            .find(|it| it.type_ == ConstructionType::Building(type_))
        {
            self.constructions.push(Construction::new(build, [ix, iy]));
            // self.build_building(ix, iy, type_)?;
        }
        Ok(())
    }

    pub fn cancel_build(&mut self, x: f64, y: f64) {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;

        if let Some(c) = self.constructions.iter_mut().find(|c| c.pos == [ix, iy]) {
            c.toggle_cancel();
        }
    }

    pub fn get_recipes(&self, x: f64, y: f64) -> Result<Vec<JsValue>, JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
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

    pub fn set_recipe(&mut self, x: f64, y: f64, name: &str) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
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

    pub fn pan(&mut self, x: f64, y: f64) {
        self.viewport.offset[0] += x;
        self.viewport.offset[1] += y;
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        self.process_global_tasks();
        self.process_transports();
        self.process_constructions();
        self.process_buildings();
        self.process_crews();

        self.global_time += 1;

        Ok(())
    }
}
