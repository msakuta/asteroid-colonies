use std::fmt::Display;

use wasm_bindgen::JsValue;

use crate::{AsteroidColonies, Building, BuildingType, Cell, CellState, WIDTH};

pub(crate) const EXCAVATE_TIME: usize = 10;
pub(crate) const MOVE_TIME: usize = 2;
pub(crate) const BUILD_POWER_GRID_TIME: usize = 5;
pub(crate) const BUILD_CONVEYOR_TIME: usize = 10;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Task {
    None,
    Excavate(usize, Direction),
    Move(usize, Direction),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Excavate(_, _) => write!(f, "Excavate"),
            Self::Move(_, _) => write!(f, "Move"),
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
        if self
            .buildings
            .iter()
            .any(|b| b.pos[0] == ix && b.pos[1] == iy)
        {
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

    pub(crate) fn power(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        let cell = &self.cells[ix as usize + iy as usize * WIDTH];
        if matches!(cell.state, CellState::Solid) {
            return Err(JsValue::from("Needs excavation before building power grid"));
        }
        if cell.power_grid {
            return Err(JsValue::from(
                "Power grid is already installed in this cell",
            ));
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
            if src_cell.power_grid {
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
                self.global_tasks
                    .push(GlobalTask::BuildConveyor(BUILD_CONVEYOR_TIME, [ix, iy]));
                return Ok(JsValue::from(true));
            }
        }
        Err(JsValue::from("No nearby power grid"))
    }

    pub(super) fn process_task(cells: &mut [Cell], building: &mut Building) {
        match building.task {
            Task::Excavate(ref mut t, dir) => {
                if *t == 0 {
                    building.task = Task::None;
                    *building.inventory.entry(crate::ItemType::Slug).or_default() += 1.;
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
            _ => {}
        }
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
