use std::{collections::HashMap, fmt::Display};

use wasm_bindgen::JsValue;

use crate::{
    building::Recipe,
    construction::{BuildMenuItem, Construction, ConstructionType},
    render::calculate_back_image,
    transport::find_path,
    AsteroidColonies, Building, BuildingType, Cell, CellState, ItemType, Pos, WIDTH,
};

pub(crate) const EXCAVATE_TIME: f64 = 10.;
pub(crate) const LABOR_EXCAVATE_TIME: f64 = 100.;
pub(crate) const MOVE_TIME: f64 = 2.;
pub(crate) const BUILD_POWER_GRID_TIME: f64 = 60.;
pub(crate) const BUILD_CONVEYOR_TIME: f64 = 90.;
pub(crate) const MOVE_ITEM_TIME: f64 = 2.;
pub(crate) const RAW_ORE_SMELT_TIME: f64 = 30.;
pub(crate) const EXCAVATE_ORE_AMOUNT: usize = 5;

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
    Build(f64, [i32; 2], &'static BuildMenuItem),
    /// Excavate using human labor. Very slow and inefficient.
    Excavate(f64, [i32; 2]),
}

impl AsteroidColonies {
    pub(crate) fn excavate(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if !matches!(
            self.cells[ix as usize + iy as usize * WIDTH].state,
            CellState::Solid
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
        if self
            .buildings
            .iter()
            .find(|b| {
                matches!(b.type_, BuildingType::CrewCabin)
                    && find_path(b.pos, [ix, iy], |pos| {
                        matches!(
                            self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                            CellState::Empty
                        ) || pos == [ix, iy]
                    })
                    .is_some()
            })
            .is_none()
        {
            return Err(JsValue::from(
                "No crew cabin that can reach the position found",
            ));
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
        if matches!(cell.state, CellState::Space) {
            return Err(JsValue::from("You cannot build power grid in space!"));
        }
        if cell.power_grid {
            return Err(JsValue::from(
                "Power grid is already installed in this cell",
            ));
        }
        self.constructions
            .push(Construction::new_power_grid([ix, iy]));
        Ok(JsValue::from(true))
    }

    pub(crate) fn conveyor(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building conveyor"));
        }
        if matches!(cell.state, CellState::Space) {
            return Err(JsValue::from("You cannot build conveyor in space!"));
        }
        if cell.conveyor {
            return Err(JsValue::from("Conveyor is already installed in this cell"));
        }
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
                self.constructions
                    .push(Construction::new_conveyor([ix, iy]));
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
                if *t <= 0. {
                    building.task = Task::None;
                    *building
                        .inventory
                        .entry(crate::ItemType::RawOre)
                        .or_default() += EXCAVATE_ORE_AMOUNT;
                    let dir_vec = dir.to_vec();
                    let [x, y] = [building.pos[0] + dir_vec[0], building.pos[1] + dir_vec[1]];
                    cells[x as usize + y as usize * WIDTH].state = CellState::Empty;
                    calculate_back_image(cells);
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
        let power_cap: isize = self.buildings.iter().map(|b| b.power()).sum();
        let power = power_cap;

        for task in &self.global_tasks {
            match task {
                GlobalTask::Build(t, pos, recipe) if *t <= 0. => match recipe.type_ {
                    ConstructionType::Building(ty) => {
                        self.buildings.push(Building::new(*pos, ty));
                    }
                    ConstructionType::PowerGrid => {
                        self.cells[pos[0] as usize + pos[1] as usize * WIDTH].power_grid = true;
                    }
                    ConstructionType::Conveyor => {
                        self.cells[pos[0] as usize + pos[1] as usize * WIDTH].conveyor = true;
                    }
                },
                GlobalTask::Excavate(t, pos) if *t <= 0. => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state = CellState::Empty;
                    calculate_back_image(&mut self.cells);
                }
                _ => {}
            }
        }

        self.global_tasks.retain_mut(|task| match task {
            GlobalTask::Build(ref mut t, _, _) => !(*t <= 0.),
            GlobalTask::Excavate(ref mut t, _) => !(*t <= 0.),
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
