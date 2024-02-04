use std::fmt::Display;

use wasm_bindgen::JsValue;

use crate::{AsteroidColonies, BuildingType, CellState, WIDTH};

pub(crate) const EXCAVATE_TIME: usize = 10;
pub(crate) const MOVE_TIME: usize = 2;

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

impl AsteroidColonies {
    pub(crate) fn excavate(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if matches!(
            self.cells[ix as usize + iy as usize * WIDTH],
            CellState::Empty
        ) {
            return Err(JsValue::from("Already excavated"));
        }
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if iy == building.pos[1] {
                if ix - building.pos[0] == 1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Right);
                } else if ix - building.pos[0] == -1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Left);
                }
            }
            if ix == building.pos[0] {
                if iy - building.pos[0] == 1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Down);
                } else if iy - building.pos[0] == -1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Up);
                }
            }
        }
        Ok(JsValue::from(true))
    }

    pub(crate) fn move_(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if matches!(
            self.cells[ix as usize + iy as usize * WIDTH],
            CellState::Solid
        ) {
            return Err(JsValue::from("Needs excavation before moving"));
        }
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if iy == building.pos[1] {
                if ix - building.pos[0] == 1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Right);
                } else if ix - building.pos[0] == -1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Left);
                }
            }
            if ix == building.pos[0] {
                if iy - building.pos[0] == 1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Down);
                } else if iy - building.pos[0] == -1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Up);
                }
            }
        }
        Ok(JsValue::from(true))
    }
}
