use std::{collections::HashMap, fmt::Display};

use wasm_bindgen::JsValue;

use crate::{
    building::Recipe, construction::BuildMenuItem, AsteroidColonies, Building, BuildingType, Cell,
    CellState, ItemType, Pos, WIDTH,
};

pub(crate) const EXCAVATE_TIME: f64 = 10.;
pub(crate) const LABOR_EXCAVATE_TIME: f64 = 100.;
pub(crate) const MOVE_TIME: f64 = 2.;
pub(crate) const BUILD_POWER_GRID_TIME: f64 = 5.;
pub(crate) const BUILD_CONVEYOR_TIME: f64 = 10.;
pub(crate) const MOVE_ITEM_TIME: f64 = 2.;
pub(crate) const RAW_ORE_SMELT_TIME: f64 = 30.;

#[derive(Clone, Debug)]
pub(crate) enum Task {
    None,
    Excavate(f64, Direction),
    Move(f64, Vec<Pos>),
    MoveItem {
        t: f64,
        item_type: ItemType,
        dest: [i32; 2],
    },
    Assemble {
        t: f64,
        max_t: f64,
        outputs: HashMap<ItemType, usize>,
    },
    // Smelt(usize),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Excavate(_, _) => write!(f, "Excavate"),
            Self::Move(_, _) => write!(f, "Move"),
            Self::MoveItem { .. } => write!(f, "MoveItem"),
            Self::Assemble { .. } => write!(f, "BuildItem"),
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
    BuildPowerGrid(f64, [i32; 2]),
    BuildConveyor(f64, [i32; 2]),
    BuildBuilding(f64, [i32; 2], &'static BuildMenuItem),
    /// Excavate using human labor. Very slow and inefficient.
    Excavate(f64, [i32; 2]),
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
            if building.type_.capacity() <= building.inventory_size() {
                continue;
            }
            if let Some(dir) = choose_direction(&building.pos, ix, iy) {
                building.task = Task::Excavate(EXCAVATE_TIME, dir);
                return Ok(JsValue::from(true));
            }
        }
        self.global_tasks
            .push(GlobalTask::Excavate(LABOR_EXCAVATE_TIME, [ix, iy]));
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

    pub(super) fn _is_clear(&self, ix: i32, iy: i32, size: [usize; 2]) -> bool {
        for jy in iy..iy + size[1] as i32 {
            for jx in ix..ix + size[0] as i32 {
                let j_cell = &self.cells[jx as usize + jy as usize * WIDTH];
                if matches!(j_cell.state, CellState::Solid) {
                    return false;
                }
            }
        }
        true
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
        power_ratio: f64,
    ) -> Option<(ItemType, [i32; 2])> {
        match building.task {
            Task::Excavate(ref mut t, dir) => {
                const TOTAL_AMOUNT: usize = 5;
                if *t <= 0. {
                    building.task = Task::None;
                    *building
                        .inventory
                        .entry(crate::ItemType::RawOre)
                        .or_default() += TOTAL_AMOUNT;
                    let dir_vec = dir.to_vec();
                    let [x, y] = [building.pos[0] + dir_vec[0], building.pos[1] + dir_vec[1]];
                    cells[x as usize + y as usize * WIDTH].state = CellState::Empty;
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            Task::Move(ref mut t, ref mut path) => {
                if *t <= 0. {
                    if let Some(next) = path.pop() {
                        building.pos = next;
                        *t = MOVE_TIME;
                    } else {
                        building.task = Task::None;
                    }
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            Task::MoveItem {
                ref mut t,
                item_type,
                dest,
            } => {
                if *t <= 0. {
                    building.task = Task::None;
                    let entry = building.inventory.entry(item_type).or_default();
                    if 0 < *entry {
                        *entry -= 1;
                        return Some((item_type, dest));
                    }
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            Task::Assemble {
                ref mut t,
                ref outputs,
                ..
            } => {
                if *t <= 0. {
                    let count = outputs.iter().map(|(_, c)| c).sum::<usize>()
                        + building.inventory.iter().map(|(_, c)| c).sum::<usize>();
                    if count < building.type_.capacity() {
                        for (i, c) in outputs {
                            *building.inventory.entry(*i).or_default() += c;
                        }
                        building.task = Task::None;
                    }
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            _ => {}
        }
        None
    }

    pub(super) fn process_global_tasks(&mut self) {
        let mut workforce: usize = self.buildings.iter().map(|b| b.crews).sum();
        let power_cap: isize = self.buildings.iter().map(|b| b.power()).sum();
        let mut power = power_cap;

        for task in &self.global_tasks {
            match task {
                GlobalTask::BuildPowerGrid(t, pos) if *t <= 0. => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].power_grid = true;
                }
                GlobalTask::BuildConveyor(t, pos) if *t <= 0. => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].conveyor = true;
                }
                GlobalTask::BuildBuilding(t, pos, recipe) if *t <= 0. => {
                    self.buildings.push(Building::new(*pos, recipe.type_));
                }
                GlobalTask::Excavate(t, pos) if *t <= 0. => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state = CellState::Empty;
                    let cabin = self.buildings.iter_mut().find(|b| {
                        matches!(b.type_, BuildingType::CrewCabin)
                            && b.inventory_size() < b.type_.capacity()
                    });
                    if let Some(cabin) = cabin {
                        *cabin.inventory.entry(ItemType::RawOre).or_default() += 1;
                    }
                }
                _ => {}
            }
        }

        const POWER_CONSUMPTION: usize = 200;

        self.global_tasks.retain_mut(|task| match task {
            GlobalTask::BuildPowerGrid(ref mut t, _)
            | GlobalTask::BuildConveyor(ref mut t, _)
            | GlobalTask::BuildBuilding(ref mut t, _, _)
            | GlobalTask::Excavate(ref mut t, _) => {
                // TODO: use power_ratio
                if *t <= 0. {
                    false
                } else {
                    if 0 < workforce && POWER_CONSUMPTION as isize <= power {
                        *t -= 1.;
                        power -= POWER_CONSUMPTION as isize;
                        workforce -= 1;
                    }
                    true
                }
            }
        });

        self.used_power = (power_cap - power) as usize;
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
