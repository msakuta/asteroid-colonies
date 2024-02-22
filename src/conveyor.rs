use std::cmp::Ordering;

use crate::{
    console_log, construction::Construction, render::TILE_SIZE, task::Direction, AsteroidColonies,
    Cell, WIDTH,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Conveyor can stack up to 2 levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum Conveyor {
    None,
    One(Direction, Direction),
    Two((Direction, Direction), (Direction, Direction)),
}

impl Conveyor {
    #[allow(dead_code)]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn from(&self) -> Option<Direction> {
        match self {
            Self::None => None,
            Self::One(from, _) => Some(*from),
            Self::Two((from, _), _) => Some(*from),
        }
    }

    pub fn to(&self) -> Option<Direction> {
        match self {
            Self::None => None,
            Self::One(_, to) => Some(*to),
            Self::Two((_, to), _) => Some(*to),
        }
    }

    pub fn has_from(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(from, _) => from == dir,
            Self::Two((from1, _), (from2, _)) => from1 == dir || from2 == dir,
        }
    }

    pub fn has_to(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(_, to) => to == dir,
            Self::Two((_, to1), (_, to2)) => to1 == dir || to2 == dir,
        }
    }

    pub fn has(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(from, to) => from == dir || to == dir,
            Self::Two((from1, to1), (from2, to2)) => {
                from1 == dir || to1 == dir || from2 == dir || to2 == dir
            }
        }
    }

    /// Returns whether it has the second level (vertical intersection)
    pub fn has_two(&self) -> bool {
        matches!(self, Self::Two(_, _))
    }
}

#[wasm_bindgen]
impl AsteroidColonies {
    /// Preview or stage conveyor build plan.
    pub fn preview_build_conveyor(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        preview: bool,
    ) -> Result<(), JsValue> {
        use {Conveyor::*, Direction::*};
        let ix0 = (x0 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy0 = (y0 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let ix1 = (x1 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy1 = (y1 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let x_rev = ix1.cmp(&ix0);
        let y_rev = iy1.cmp(&iy0);

        self.conveyor_preview.clear();

        let mut prev_from = Option::None;

        let pos = [ix0, iy0];
        let cell = &self.cells[pos[0] as usize + pos[1] as usize * WIDTH];
        if let Some(from) = self
            .conveyor_staged
            .get(&pos)
            .and_then(|c| c.from())
            .or_else(|| cell.conveyor.from())
        {
            console_log!("conv from: {:?}", from);
            prev_from = Some(from);
        }

        let mut convs = vec![];
        if matches!(y_rev, Ordering::Less) {
            convs.extend((iy1..=iy0).rev().map(|iy| [ix0, iy]));
        } else {
            convs.extend((iy0..=iy1).map(|iy| [ix0, iy]));
        }
        if matches!(x_rev, Ordering::Less) {
            convs.extend((ix1..=ix0).rev().map(|ix| [ix, iy1]));
        } else {
            convs.extend((ix0..=ix1).map(|ix| [ix, iy1]));
        }

        let filter_conv = |cell: &Cell, staged, conv| match (cell.conveyor, conv) {
            (One(Left, Right), (Up, Down) | (Down, Up)) => Two((Left, Right), conv),
            (One(Right, Left), (Up, Down) | (Down, Up)) => Two((Right, Left), conv),
            (One(Up, Down), (Left, Right) | (Right, Left)) => Two((Up, Down), conv),
            (One(Down, Up), (Left, Right) | (Right, Left)) => Two((Down, Up), conv),
            _ => match (staged, conv) {
                (One(Left, Right), (Up, Down) | (Down, Up)) => Two((Left, Right), conv),
                (One(Right, Left), (Up, Down) | (Down, Up)) => Two((Right, Left), conv),
                (One(Up, Down), (Left, Right) | (Right, Left)) => Two((Up, Down), conv),
                (One(Down, Up), (Left, Right) | (Right, Left)) => Two((Down, Up), conv),
                _ => One(conv.0, conv.1),
            },
        };

        // console_log!("conv pos ix0: {ix0}, ix1: {ix1}, xrev: {x_rev}, iy0: {iy0}, iy1: {iy1}, yrev: {y_rev}, {:?}", convs);
        for (pos0, pos1) in convs.iter().zip(convs.iter().skip(1)) {
            let cell = &self.cells[pos0[0] as usize + pos0[1] as usize * WIDTH];
            let staged = self.conveyor_staged.get(pos0).copied().unwrap_or(None);
            let Some(to) = Direction::from_vec([pos1[0] - pos0[0], pos1[1] - pos0[1]]) else {
                continue;
            };
            let from = prev_from.unwrap_or_else(|| to.reverse());
            prev_from = Some(to.reverse());
            let conv = filter_conv(cell, staged, (from, to));
            console_log!("pos {:?} conv {:?}", pos0, conv);
            // console_log!("conv {:?}: {:?}", pos1, conv);
            self.conveyor_preview.insert(*pos0, conv);
        }

        if let Some((pos, prev_from)) = convs.last().zip(prev_from) {
            let cell = &self.cells[pos[0] as usize + pos[1] as usize * WIDTH];
            let staged = self.conveyor_staged.get(pos).copied().unwrap_or(None);
            let to = self
                .conveyor_staged
                .get(pos)
                .and_then(|c| c.to())
                .or_else(|| cell.conveyor.to())
                .unwrap_or_else(|| prev_from.reverse());
            let conv = filter_conv(cell, staged, (prev_from, to));
            self.conveyor_preview.insert(*pos, conv);
        }

        if !preview {
            self.conveyor_staged.extend(self.conveyor_preview.drain());
        }
        Ok(())
    }

    pub fn cancel_build_conveyor(&mut self, preview: bool) {
        if !preview {
            self.conveyor_staged.clear();
        }
        self.conveyor_preview.clear();
    }

    pub fn commit_build_conveyor(&mut self) {
        for (pos, conv) in self.conveyor_staged.drain() {
            self.constructions
                .push(Construction::new_conveyor(pos, conv));
        }
        self.conveyor_preview.clear();
    }
}
