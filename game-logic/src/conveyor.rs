use std::{cmp::Ordering, hash::Hash};

use crate::{
    console_log, construction::Construction, push_pull::TileSampler, task::Direction,
    AsteroidColoniesGame, Tile,
};
use serde::{Deserialize, Serialize};

/// Conveyor can stack up to 2 levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conveyor {
    None,
    One(Direction, Direction),
    Two((Direction, Direction), (Direction, Direction)),
    /// Assume a splitter splits to the other 3 directions
    Splitter(Direction),
    /// Assume a splitter merges from the other 3 directions
    Merger(Direction),
}

impl Hash for Conveyor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // It's annoying to define hash logic for all cases, but we need to do it
        // manually to ensure it's compatible among CPU architectures, namely
        // Wasm32 and x64.
        match self {
            Self::None => 0u8.hash(state),
            Self::One(a, b) => {
                1u8.hash(state);
                a.hash(state);
                b.hash(state);
            }
            Self::Two((a, b), (c, d)) => {
                2u8.hash(state);
                a.hash(state);
                b.hash(state);
                c.hash(state);
                d.hash(state);
            }
            Self::Splitter(a) => {
                3u8.hash(state);
                a.hash(state);
            }
            Self::Merger(a) => {
                4u8.hash(state);
                a.hash(state);
            }
        }
    }
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
            Self::Splitter(from) => Some(*from),
            Self::Merger(to) => Some(to.reverse()),
        }
    }

    pub fn to(&self) -> Option<Direction> {
        match self {
            Self::None => None,
            Self::One(_, to) => Some(*to),
            Self::Two((_, to), _) => Some(*to),
            Self::Splitter(from) => Some(from.reverse()),
            Self::Merger(to) => Some(*to),
        }
    }

    pub fn has_from(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(from, _) => from == dir,
            Self::Two((from1, _), (from2, _)) => from1 == dir || from2 == dir,
            Self::Splitter(from) => from == dir,
            Self::Merger(to) => to != dir,
        }
    }

    pub fn has_to(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(_, to) => to == dir,
            Self::Two((_, to1), (_, to2)) => to1 == dir || to2 == dir,
            Self::Splitter(from) => from != dir,
            Self::Merger(to) => to == dir,
        }
    }

    /// Has any connection to the given direction, regardless of direction
    pub fn has(&self, dir: Direction) -> bool {
        match *self {
            Self::None => false,
            Self::One(from, to) => from == dir || to == dir,
            Self::Two((from1, to1), (from2, to2)) => {
                from1 == dir || to1 == dir || from2 == dir || to2 == dir
            }
            Self::Splitter(from) => from == dir,
            Self::Merger(to) => to == dir,
        }
    }

    /// Returns whether it has the second level (vertical intersection)
    pub fn has_two(&self) -> bool {
        matches!(self, Self::Two(_, _))
    }
}

impl AsteroidColoniesGame {
    /// Preview or stage conveyor build plan.
    pub fn preview_build_conveyor(
        &mut self,
        ix0: i32,
        iy0: i32,
        ix1: i32,
        iy1: i32,
        preview: bool,
    ) -> Result<(), String> {
        use {Conveyor::*, Direction::*};
        let x_rev = ix1.cmp(&ix0);
        let y_rev = iy1.cmp(&iy0);

        self.conveyor_preview.clear();

        let mut prev_from = Option::None;

        let pos = [ix0, iy0];
        let tile = &self.tiles[pos];
        if let Some(from) = self
            .conveyor_staged
            .get(&pos)
            .and_then(|c| c.from())
            .or_else(|| tile.conveyor.from())
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

        let filter_conv = |tile: &Tile, staged, conv| match (tile.conveyor, conv) {
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
            let tile = &self.tiles[*pos0];
            let staged = self.conveyor_staged.get(pos0).copied().unwrap_or(None);
            let Some(to) = Direction::from_vec([pos1[0] - pos0[0], pos1[1] - pos0[1]]) else {
                continue;
            };
            let from = prev_from.unwrap_or_else(|| to.reverse());
            prev_from = Some(to.reverse());
            let conv = filter_conv(tile, staged, (from, to));
            console_log!("pos {:?} conv {:?}", pos0, conv);
            // console_log!("conv {:?}: {:?}", pos1, conv);
            self.conveyor_preview.insert(*pos0, conv);
        }

        if let Some((pos, prev_from)) = convs.last().zip(prev_from) {
            let tile = &self.tiles[*pos];
            let staged = self.conveyor_staged.get(pos).copied().unwrap_or(None);
            let to = self
                .conveyor_staged
                .get(pos)
                .and_then(|c| c.to())
                .or_else(|| tile.conveyor.to())
                .unwrap_or_else(|| prev_from.reverse());
            let conv = filter_conv(tile, staged, (prev_from, to));
            self.conveyor_preview.insert(*pos, conv);
        }

        if !preview {
            self.conveyor_staged.extend(self.conveyor_preview.drain());
        }
        Ok(())
    }

    pub fn build_splitter(&mut self, ix0: i32, iy0: i32) {
        use {Conveyor::*, Direction::*};

        let filter_conv = |tile: Conveyor, staged| match tile {
            One(from, _) => Splitter(from),
            Two((from1, _), (_, _)) => Splitter(from1),
            _ => match staged {
                One(from, _) => Splitter(from),
                Two((from1, _), _) => Splitter(from1),
                _ => Splitter(Left),
            },
        };

        let pos0 = [ix0, iy0];
        let tile = self
            .tiles
            .at([ix0, iy0])
            .map(|c| c.conveyor)
            .unwrap_or(None);
        let staged = self.conveyor_staged.get(&pos0).copied().unwrap_or(None);

        self.conveyor_staged
            .insert([ix0, iy0], filter_conv(tile, staged));
    }

    pub fn build_merger(&mut self, ix0: i32, iy0: i32) {
        use {Conveyor::*, Direction::*};

        let pos0 = [ix0, iy0];
        let tile = self
            .tiles
            .at([ix0, iy0])
            .map(|c| c.conveyor)
            .unwrap_or(None);
        let staged = self.conveyor_staged.get(&pos0).copied().unwrap_or(None);

        let filtered = match tile {
            One(_, to) => Merger(to),
            Two((_, to1), _) => Merger(to1),
            _ => match staged {
                One(_, to) => Merger(to),
                Two((_, to1), _) => Merger(to1),
                _ => Merger(Right),
            },
        };

        self.conveyor_staged.insert([ix0, iy0], filtered);
    }

    pub fn cancel_build_conveyor(&mut self, preview: bool) {
        if !preview {
            self.conveyor_staged.clear();
        }
        self.conveyor_preview.clear();
    }

    pub fn commit_build_conveyor(&mut self) -> Vec<Construction> {
        for (pos, conv) in self.conveyor_staged.iter() {
            self.constructions
                .push(Construction::new_conveyor(*pos, *conv));
        }
        self.conveyor_preview.clear();
        std::mem::take(&mut self.conveyor_staged)
            .into_iter()
            .map(|(pos, conv)| Construction::new_conveyor(pos, conv))
            .collect()
    }
}
