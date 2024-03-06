use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::{
    building::{Building, BuildingType, Recipe},
    construction::Construction,
    direction::Direction,
    game::CalculateBackImage,
    transport::find_path,
    AsteroidColoniesGame, ItemType, Pos, TileState, Tiles,
};

pub const EXCAVATE_TIME: f64 = 10.;
pub const LABOR_EXCAVATE_TIME: f64 = 100.;
pub const MOVE_TIME: f64 = 2.;
pub const BUILD_POWER_GRID_TIME: f64 = 60.;
pub const BUILD_CONVEYOR_TIME: f64 = 90.;
pub const MOVE_ITEM_TIME: f64 = 2.;
pub(crate) const RAW_ORE_SMELT_TIME: f64 = 30.;
pub(crate) const EXCAVATE_ORE_AMOUNT: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum GlobalTask {
    /// Excavate using human labor. Very slow and inefficient.
    Excavate(f64, [i32; 2]),
    Cleanup(Pos),
}

impl AsteroidColoniesGame {
    pub fn excavate(&mut self, ix: i32, iy: i32) -> Result<bool, String> {
        if !matches!(self.tiles[[ix, iy]].state, TileState::Solid) {
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
                building.direction = Some(dir);
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
                        matches!(self.tiles[pos].state, TileState::Empty) || pos == [ix, iy]
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
        let tile = &self.tiles[[ix, iy]];
        if matches!(tile.state, TileState::Solid) {
            return Err(String::from("Needs excavation before building power grid"));
        }
        if matches!(tile.state, TileState::Space) {
            return Err(String::from("You cannot build power grid in space!"));
        }
        if tile.power_grid {
            return Err(String::from("Power grid is already installed in this tile"));
        }
        self.constructions
            .push(Construction::new_power_grid([ix, iy]));
        Ok(true)
    }

    pub fn move_item(&mut self, ix: i32, iy: i32) -> Result<bool, String> {
        let tile = &self.tiles[[ix, iy]];
        if matches!(tile.state, TileState::Solid) {
            return Err(String::from("Needs excavation before building conveyor"));
        }
        if !tile.conveyor.is_some() {
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
                let j_tile = &self.tiles[[jx, jy]];
                if matches!(j_tile.state, TileState::Solid) {
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
        recipe: Option<&Recipe>,
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
        assembler.recipe = recipe.cloned();
        Ok(true)
    }

    pub(super) fn process_task(
        tiles: &mut Tiles,
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
                    let pos = [building.pos[0] + dir_vec[0], building.pos[1] + dir_vec[1]];
                    tiles[pos].state = TileState::Empty;
                    if let Some(f) = calculate_back_image {
                        f(tiles);
                    }
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            Task::Move(ref mut t, ref mut path) => {
                if *t <= 0. {
                    if let Some(next) = path.pop() {
                        let direction = Direction::from_vec([
                            next[0] - building.pos[0],
                            next[1] - building.pos[1],
                        ]);
                        building.pos = next;
                        building.direction = direction;
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
                    self.tiles[*pos].state = TileState::Empty;
                    if let Some(ref f) = self.calculate_back_image {
                        f(&mut self.tiles);
                    }
                }
                _ => {}
            }
        }

        self.global_tasks.retain_mut(|task| match task {
            GlobalTask::Excavate(ref mut t, _) => !(*t <= 0.),
            GlobalTask::Cleanup(pos) => self.transports.iter().any(|t| t.path.last() == Some(pos)),
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
