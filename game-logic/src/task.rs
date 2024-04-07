use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::{
    building::{Building, BuildingType},
    construction::Construction,
    crew::proceed_excavate,
    direction::Direction,
    entity::{EntityId, EntitySet},
    game::CalculateBackImage,
    items::ItemType,
    transport::find_path,
    AsteroidColoniesGame, Pos, TileState, Tiles,
};

pub const EXCAVATE_TIME: f64 = 10.;
pub const LABOR_EXCAVATE_TIME: f64 = 100.;
pub const EXCAVATOR_SPEED: f64 = LABOR_EXCAVATE_TIME / EXCAVATE_TIME;
pub const MOVE_TIME: f64 = 2.;
pub const BUILD_POWER_GRID_TIME: f64 = 60.;
pub const BUILD_CONVEYOR_TIME: f64 = 90.;
pub const MOVE_ITEM_TIME: f64 = 2.;
pub(crate) const RAW_ORE_SMELT_TIME: f64 = 30.;
pub(crate) const EXCAVATE_ORE_AMOUNT: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BuildingTask {
    None,
    Excavate(Direction, EntityId),
    Move(f64, Vec<Pos>),
    MoveToExcavate {
        t: f64,
        path: Vec<Pos>,
        dir: Direction,
        /// The target global task id for the excavation
        target: EntityId,
    },
    Assemble {
        t: f64,
        max_t: f64,
        outputs: HashMap<ItemType, usize>,
    },
    // Smelt(usize),
}

impl Display for BuildingTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Excavate(_, _) => write!(f, "Excavate"),
            Self::Move(_, _) => write!(f, "Move"),
            Self::MoveToExcavate { .. } => write!(f, "MoveToExcavate"),
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
        // for building in self.buildings.iter_mut() {
        //     if building.type_ != BuildingType::Excavator {
        //         continue;
        //     }
        //     if building.type_.capacity() <= building.inventory_size() {
        //         continue;
        //     }
        //     if let Some(dir) = choose_direction(&building.pos, ix, iy) {
        //         building.direction = Some(dir);
        //         building.task = BuildingTask::Excavate(EXCAVATE_TIME, dir);
        //         return Ok(true);
        //     }
        // }
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
            .insert(GlobalTask::Excavate(LABOR_EXCAVATE_TIME, [ix, iy]));
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
            .insert(Construction::new_power_grid([ix, iy], false));
        Ok(true)
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

    fn process_move(
        t: &mut f64,
        path: &mut Vec<Pos>,
        power_ratio: f64,
        pos: &mut Pos,
        direction: &mut Option<Direction>,
    ) -> bool {
        let next_t = *t - power_ratio;
        if next_t <= 0. {
            if let Some(next) = path.pop() {
                *pos = next;
                let next_next = path.last().copied();
                if let Some(next_next) = next_next {
                    *direction =
                        Direction::from_vec([next_next[0] - pos[0], next_next[1] - pos[1]]);
                }
                *t = next_t + MOVE_TIME;
                false
            } else {
                true
            }
        } else {
            if let Some(next) = path.last() {
                *direction = Direction::from_vec([next[0] - pos[0], next[1] - pos[1]]);
            }
            *t = next_t;
            false
        }
    }

    pub(super) fn process_task(
        tiles: &mut Tiles,
        building: &mut Building,
        buildings: &EntitySet<Building>,
        global_tasks: &mut EntitySet<GlobalTask>,
        power_ratio: f64,
        _calculate_back_image: Option<&mut CalculateBackImage>,
    ) -> Option<(ItemType, [i32; 2])> {
        match building.task {
            BuildingTask::Excavate(_, gt_id) => {
                let Some(GlobalTask::Excavate(t, _)) = global_tasks.get_mut(gt_id) else {
                    building.task = BuildingTask::None;
                    return None;
                };
                if !proceed_excavate(t, EXCAVATOR_SPEED * power_ratio, &mut building.inventory) {
                    building.task = BuildingTask::None;
                }
            }
            BuildingTask::Move(ref mut t, ref mut path) => {
                if Self::process_move(
                    t,
                    path,
                    power_ratio,
                    &mut building.pos,
                    &mut building.direction,
                ) {
                    building.task = BuildingTask::None;
                }
            }
            BuildingTask::MoveToExcavate {
                ref mut t,
                ref mut path,
                dir,
                target,
            } => {
                if Self::process_move(
                    t,
                    path,
                    power_ratio,
                    &mut building.pos,
                    &mut building.direction,
                ) {
                    building.direction = Some(dir);
                    building.task = BuildingTask::Excavate(dir, target);
                }
            }
            BuildingTask::Assemble {
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
                        building.task = BuildingTask::None;
                    }
                } else {
                    *t = (*t - power_ratio).max(0.);
                }
            }
            BuildingTask::None => {
                if matches!(building.type_, BuildingType::Excavator) {
                    for (gt_id, gt) in global_tasks.items() {
                        Self::process_excavate_global_task(building, buildings, tiles, gt_id, &*gt);
                    }
                }
            }
        }
        None
    }

    fn process_excavate_global_task(
        building: &mut Building,
        buildings: &EntitySet<Building>,
        tiles: &Tiles,
        gt_id: EntityId,
        gt: &GlobalTask,
    ) -> Option<()> {
        let GlobalTask::Excavate(_, task_pos) = *gt else {
            return None;
        };
        // console_log!(
        //     "bldg {:?}: GloblTask::Excavate: {:?}",
        //     building.pos,
        //     task_pos
        // );

        let intersects = |pos: [i32; 2]| {
            buildings.iter().any(|b| {
                let size = b.type_.size();
                b.pos[0] <= pos[0]
                    && pos[0] < size[0] as i32 + b.pos[0]
                    && b.pos[1] <= pos[1]
                    && pos[1] < size[1] as i32 + b.pos[1]
            })
        };

        let path = find_path(building.pos, task_pos, |pos| {
            let tile = &tiles[pos];
            !intersects(pos) && matches!(tile.state, TileState::Empty) && tile.power_grid
                || pos == task_pos
        });
        // console_log!("         GloblTask::Excavate: path= {:?}", path);
        if let Some(mut path) = path {
            if path.len() <= 2 {
                if let Some(d) = choose_direction(&building.pos, &task_pos) {
                    building.direction = Some(d);
                    building.task = BuildingTask::Excavate(d, gt_id);
                }
            } else {
                let last_pos = path.remove(0);
                let next_to_last_pos = path.first()?;
                if let Some(d) = choose_direction(next_to_last_pos, &last_pos) {
                    // console_log!(
                    //     "         assigning BuildingTask::MoveToExcavate path={:?}, dir={:?}",
                    //     path,
                    //     d
                    // );
                    building.task = BuildingTask::MoveToExcavate {
                        t: MOVE_TIME,
                        path,
                        dir: d,
                        target: gt_id,
                    };
                }
            }
        }
        None
    }

    pub(super) fn process_global_tasks(&mut self) {
        for task in &self.global_tasks {
            match &*task {
                GlobalTask::Excavate(t, pos) if *t <= 0. => {
                    self.tiles[*pos].state = TileState::Empty;
                    if let Some(ref f) = self.calculate_back_image {
                        f(&mut self.tiles);
                    }
                }
                _ => {}
            }
        }

        self.global_tasks.retain(|task| match task {
            GlobalTask::Excavate(ref mut t, _) => !(*t <= 0.),
            GlobalTask::Cleanup(pos) => self.transports.iter().any(|t| t.path.last() == Some(pos)),
        });
    }
}

fn choose_direction(pos: &[i32; 2], &[ix, iy]: &[i32; 2]) -> Option<Direction> {
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
