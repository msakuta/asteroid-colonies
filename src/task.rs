use std::{collections::HashMap, fmt::Display};

use wasm_bindgen::JsValue;

use crate::{
    building::Recipe, AsteroidColonies, Building, BuildingType, Cell, CellState, ItemType, WIDTH,
};

pub(crate) const EXCAVATE_TIME: usize = 10;
pub(crate) const MOVE_TIME: usize = 2;
pub(crate) const BUILD_POWER_GRID_TIME: usize = 5;
pub(crate) const BUILD_CONVEYOR_TIME: usize = 10;
pub(crate) const MOVE_ITEM_TIME: usize = 2;
pub(crate) const BUILD_POWER_PLANT_TIME: usize = 50;
pub(crate) const BUILD_EXCAVATOR_TIME: usize = 100;
pub(crate) const BUILD_CREW_CABIN_TIME: usize = 500;
pub(crate) const BUILD_STORAGE_TIME: usize = 20;
pub(crate) const BUILD_ASSEMBLER_TIME: usize = 100;
pub(crate) const BUILD_FURNACE_TIME: usize = 100;
pub(crate) const IRON_INGOT_SMELT_TIME: usize = 50;

#[derive(Clone, Debug)]
pub(crate) enum Task {
    None,
    Excavate(usize, Direction),
    Move(usize, Direction),
    MoveItem {
        t: usize,
        item_type: ItemType,
        dest: [i32; 2],
    },
    Assemble(usize, HashMap<ItemType, usize>),
    // Smelt(usize),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Excavate(_, _) => write!(f, "Excavate"),
            Self::Move(_, _) => write!(f, "Move"),
            Self::MoveItem { .. } => write!(f, "MoveItem"),
            Self::Assemble(_, _) => write!(f, "BuildItem"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    pub(crate) fn to_vec(&self) -> [i32; 2] {
        match self {
            Self::Left => [-1, 0],
            Self::Up => [0, -1],
            Self::Right => [1, 0],
            Self::Down => [0, 1],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GlobalTask {
    BuildPowerGrid(usize, [i32; 2]),
    BuildConveyor(usize, [i32; 2]),
    BuildBuilding(usize, [i32; 2], BuildingType),
}

impl AsteroidColonies {
    pub(crate) fn excavate(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if matches!(
            self.cells[ix as usize + iy as usize * WIDTH].state,
            CellState::Empty
        ) {
            return Err(JsValue::from("Already excavated"));
        }
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if building.type_.capacity() <= building.inventory.iter().map(|(_, v)| *v).sum() {
                continue;
            }
            if let Some(dir) = choose_direction(&building.pos, ix, iy) {
                building.task = Task::Excavate(EXCAVATE_TIME, dir);
            }
        }
        Ok(JsValue::from(true))
    }

    pub(crate) fn move_(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before moving"));
        }
        if !cell.power_grid {
            return Err(JsValue::from("Power grid is required to move"));
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
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if let Some(dir) = choose_direction(&building.pos, ix, iy) {
                building.task = Task::Move(MOVE_TIME, dir);
            }
        }
        Ok(JsValue::from(true))
    }

    pub(crate) fn build_power_grid(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building power grid"));
        }
        if cell.power_grid {
            return Err(JsValue::from(
                "Power grid is already installed in this cell",
            ));
        }
        let no_power_grid = || JsValue::from("Power grid item does not exist in nearby structures");
        let Some(building) = self
            .buildings
            .iter_mut()
            .find(|b| 0 < *b.inventory.get(&ItemType::PowerGridComponent).unwrap_or(&0))
        else {
            return Err(no_power_grid());
        };
        for dir in [
            Direction::Left,
            Direction::Up,
            Direction::Right,
            Direction::Down,
        ] {
            let dir_vec = dir.to_vec();
            let src_pos = [ix + dir_vec[0], iy + dir_vec[1]];
            let src_cell = &self.cells[src_pos[0] as usize + src_pos[1] as usize * WIDTH];
            if src_cell.power_grid {
                *building
                    .inventory
                    .get_mut(&ItemType::PowerGridComponent)
                    .ok_or_else(no_power_grid)? -= 1;
                self.global_tasks
                    .push(GlobalTask::BuildPowerGrid(BUILD_POWER_GRID_TIME, [ix, iy]));
                return Ok(JsValue::from(true));
            }
        }
        Err(JsValue::from("No nearby power grid"))
    }

    pub(crate) fn conveyor(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building conveyor"));
        }
        if cell.conveyor {
            return Err(JsValue::from("Conveyor is already installed in this cell"));
        }
        let no_conveyor = || JsValue::from("Conveyor item does not exist in nearby structures");
        let Some(building) = self
            .buildings
            .iter_mut()
            .find(|b| 0 < *b.inventory.get(&ItemType::ConveyorComponent).unwrap_or(&0))
        else {
            return Err(no_conveyor());
        };
        for dir in [
            Direction::Left,
            Direction::Up,
            Direction::Right,
            Direction::Down,
        ] {
            let dir_vec = dir.to_vec();
            let src_pos = [ix + dir_vec[0], iy + dir_vec[1]];
            let src_cell = &self.cells[src_pos[0] as usize + src_pos[1] as usize * WIDTH];
            if src_cell.conveyor {
                *building
                    .inventory
                    .get_mut(&ItemType::ConveyorComponent)
                    .ok_or_else(no_conveyor)? -= 1;
                self.global_tasks
                    .push(GlobalTask::BuildConveyor(BUILD_CONVEYOR_TIME, [ix, iy]));
                return Ok(JsValue::from(true));
            }
        }
        Err(JsValue::from("No nearby power grid"))
    }

    pub(crate) fn move_item(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building conveyor"));
        }
        if !cell.conveyor {
            return Err(JsValue::from("Conveyor is needed to move items"));
        }
        let Some(_dest) = self
            .buildings
            .iter()
            .find(|b| b.pos[0] == ix && b.pos[1] == iy)
        else {
            return Err(JsValue::from("Needs a building at the destination"));
        };
        for building in &mut self.buildings {
            if 0 < *building.inventory.get(&ItemType::RawOre).unwrap_or(&0) {
                building.task = Task::MoveItem {
                    t: MOVE_ITEM_TIME,
                    item_type: ItemType::RawOre,
                    dest: [ix, iy],
                };
                return Ok(JsValue::from(true));
            }
        }
        Err(JsValue::from("No structure to send from"))
    }

    pub(super) fn build_building(
        &mut self,
        ix: i32,
        iy: i32,
        type_: BuildingType,
    ) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building a building"));
        }
        if !cell.conveyor {
            return Err(JsValue::from("Conveyor is needed to build a building"));
        }
        if self
            .buildings
            .iter()
            .any(|b| b.pos[0] == ix && b.pos[1] == iy)
        {
            return Err(JsValue::from("A building already exists at the target"));
        }
        self.global_tasks.push(GlobalTask::BuildBuilding(
            type_.build_time(),
            [ix, iy],
            type_,
        ));
        Ok(JsValue::from(true))
    }

    pub(super) fn set_building_recipe(
        &mut self,
        ix: i32,
        iy: i32,
        recipe: &'static Recipe,
    ) -> Result<JsValue, JsValue> {
        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        let Some(assembler) = self.buildings.iter_mut().find(|b| intersects(*b)) else {
            return Err(JsValue::from("The building does not exist at the target"));
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(JsValue::from("The building is not an assembler"));
        }
        assembler.recipe = Some(recipe);
        Ok(JsValue::from(true))
    }

    pub(super) fn process_task(
        cells: &mut [Cell],
        building: &mut Building,
    ) -> Option<(ItemType, [i32; 2])> {
        match building.task {
            Task::Excavate(ref mut t, dir) => {
                if *t == 0 {
                    building.task = Task::None;
                    *building
                        .inventory
                        .entry(crate::ItemType::RawOre)
                        .or_default() += 1;
                    let dir_vec = dir.to_vec();
                    let [x, y] = [building.pos[0] + dir_vec[0], building.pos[1] + dir_vec[1]];
                    cells[x as usize + y as usize * WIDTH].state = CellState::Empty;
                } else {
                    *t -= 1;
                }
            }
            Task::Move(ref mut t, dir) => {
                if *t == 0 {
                    building.task = Task::None;
                    let dir_vec = dir.to_vec();
                    building.pos[0] += dir_vec[0];
                    building.pos[1] += dir_vec[1];
                } else {
                    *t -= 1;
                }
            }
            Task::MoveItem {
                ref mut t,
                item_type,
                dest,
            } => {
                if *t == 0 {
                    building.task = Task::None;
                    let entry = building.inventory.entry(item_type).or_default();
                    if 0 < *entry {
                        *entry -= 1;
                        return Some((item_type, dest));
                    }
                } else {
                    *t -= 1;
                }
            }
            Task::Assemble(ref mut t, ref items) => {
                if *t == 0 {
                    let count = items.iter().map(|(_, c)| c).sum::<usize>()
                        + building.inventory.iter().map(|(_, c)| c).sum::<usize>();
                    if count < building.type_.capacity() {
                        for (i, c) in items {
                            *building.inventory.entry(*i).or_default() += c;
                        }
                        building.task = Task::None;
                    }
                } else {
                    *t -= 1;
                }
            }
            _ => {}
        }
        None
    }
}

fn choose_direction(pos: &[i32; 2], ix: i32, iy: i32) -> Option<Direction> {
    if iy == pos[1] {
        if ix - pos[0] == 1 {
            return Some(Direction::Right);
        } else if ix - pos[0] == -1 {
            return Some(Direction::Left);
        }
    }
    if ix == pos[0] {
        if iy - pos[1] == 1 {
            return Some(Direction::Down);
        } else if iy - pos[1] == -1 {
            return Some(Direction::Up);
        }
    }
    None
}
