use std::{collections::HashMap, fmt::Display};

use serde::Serialize;

use crate::{
    building::{Building, BuildingType, Recipe},
    construction::Construction,
    game::CalculateBackImage,
    transport::find_path,
    AsteroidColoniesGame, Cell, CellState, ItemType, Pos, WIDTH,
};

pub const EXCAVATE_TIME: f64 = 10.;
pub const LABOR_EXCAVATE_TIME: f64 = 100.;
pub const MOVE_TIME: f64 = 2.;
pub const BUILD_POWER_GRID_TIME: f64 = 60.;
pub const BUILD_CONVEYOR_TIME: f64 = 90.;
pub const MOVE_ITEM_TIME: f64 = 2.;
pub(crate) const RAW_ORE_SMELT_TIME: f64 = 30.;
pub(crate) const EXCAVATE_ORE_AMOUNT: usize = 5;

#[derive(Clone, Debug)]
pub enum Task {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    pub(crate) const fn all() -> [Direction; 4] {
        [Self::Left, Self::Up, Self::Right, Self::Down]
    }

    pub(crate) fn to_vec(&self) -> [i32; 2] {
        match self {
            Self::Left => [-1, 0],
            Self::Up => [0, -1],
            Self::Right => [1, 0],
            Self::Down => [0, 1],
        }
    }

    pub(crate) fn from_vec(v: [i32; 2]) -> Option<Self> {
        Some(match (v[0].signum(), v[1].signum()) {
            (-1, _) => Self::Left,
            (1, _) => Self::Right,
            (0, -1) => Self::Up,
            (0, 1) => Self::Down,
            _ => return None,
        })
    }

    pub(crate) fn reverse(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GlobalTask {
    /// Excavate using human labor. Very slow and inefficient.
    Excavate(f64, [i32; 2]),
}

impl AsteroidColoniesGame {
    pub fn excavate(&mut self, ix: i32, iy: i32) -> Result<bool, String> {
        if !matches!(
            self.cells[ix as usize + iy as usize * WIDTH].state,
            CellState::Solid
        ) {
            return Err("Already excavated".to_string());
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
                return Ok(true);
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
            return Err(String::from(
                "No crew cabin that can reach the position found",
            ));
        }
        self.global_tasks
            .push(GlobalTask::Excavate(LABOR_EXCAVATE_TIME, [ix, iy]));
        Ok(true)
    }

    pub fn build_power_grid(&mut self, ix: i32, iy: i32) -> Result<bool, String> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(String::from("Needs excavation before building power grid"));
        }
        if matches!(cell.state, CellState::Space) {
            return Err(String::from("You cannot build power grid in space!"));
        }
        if cell.power_grid {
            return Err(String::from("Power grid is already installed in this cell"));
        }
        self.constructions
            .push(Construction::new_power_grid([ix, iy]));
        Ok(true)
    }

    pub fn move_item(&mut self, ix: i32, iy: i32) -> Result<bool, String> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(String::from("Needs excavation before building conveyor"));
        }
        if !cell.conveyor.is_some() {
            return Err(String::from("Conveyor is needed to move items"));
        }
        let Some(_dest) = self
            .buildings
            .iter()
            .find(|b| b.pos[0] == ix && b.pos[1] == iy)
        else {
            return Err(String::from("Needs a building at the destination"));
        };
        for building in &mut self.buildings {
            if 0 < *building.inventory.get(&ItemType::RawOre).unwrap_or(&0) {
                building.task = Task::MoveItem {
                    t: MOVE_ITEM_TIME,
                    item_type: ItemType::RawOre,
                    dest: [ix, iy],
                };
                return Ok(true);
            }
        }
        Err(String::from("No structure to send from"))
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
    ) -> Result<bool, String> {
        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        let Some(assembler) = self.buildings.iter_mut().find(|b| intersects(*b)) else {
            return Err(String::from("The building does not exist at the target"));
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(String::from("The building is not an assembler"));
        }
        assembler.recipe = Some(recipe);
        Ok(true)
    }

    pub(super) fn process_task(
        cells: &mut [Cell],
        building: &mut Building,
        power_ratio: f64,
        calculate_back_image: Option<&mut CalculateBackImage>,
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
                    if let Some(f) = calculate_back_image {
                        f(cells);
                    }
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
                GlobalTask::Excavate(t, pos) if *t <= 0. => {
                    self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state = CellState::Empty;
                    if let Some(ref mut f) = self.calculate_back_image {
                        f(&mut self.cells);
                    }
                }
                _ => {}
            }
        }

        self.global_tasks.retain_mut(|task| match task {
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
