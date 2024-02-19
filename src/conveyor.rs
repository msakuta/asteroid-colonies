use crate::{
    console_log, construction::Construction, render::TILE_SIZE, task::Direction, AsteroidColonies,
    WIDTH,
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
        let mut ix0 = (x0 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let ix_start = ix0;
        let mut iy0 = (y0 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let mut ix1 = (x1 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let mut iy1 = (y1 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let iy_end = iy1;
        let mut x_rev = false;
        let mut y_rev = false;
        if iy1 < iy0 {
            y_rev = true;
            std::mem::swap(&mut iy1, &mut iy0);
        } else {
            iy0 += 1;
        }
        if ix1 < ix0 {
            x_rev = true;
            std::mem::swap(&mut ix1, &mut ix0);
        } else {
            ix0 += 1;
        }

        let conv_v = if y_rev {
            (Direction::Down, Direction::Up)
        } else {
            (Direction::Up, Direction::Down)
        };
        let conv_h = if x_rev {
            (Direction::Right, Direction::Left)
        } else {
            (Direction::Left, Direction::Right)
        };

        self.conveyor_preview.clear();

        let mut convs = (iy0..iy1)
            .map(|iy| ([ix_start, iy], conv_v))
            .collect::<Vec<_>>();
        if convs.is_empty() {
            if let One(from, _) = &self.cells[ix_start as usize + iy_end as usize * WIDTH].conveyor
            {
                convs.push(([ix_start, iy_end], (*from, conv_h.1)));
            }
        } else {
            convs.push(([ix_start, iy_end], (conv_v.0, conv_h.1)));
        }
        convs.extend((ix0..ix1).map(|ix| ([ix, iy_end], conv_h)));
        console_log!("conv pos ix0: {ix0}, ix1: {ix1}, xrev: {x_rev}, iy0: {iy0}, iy1: {iy1}, yrev: {y_rev}, {:?}", convs);
        for (pos1, conv) in &convs {
            let cell = &self.cells[pos1[0] as usize + pos1[1] as usize * WIDTH];
            let staged = self.conveyor_staged.get(pos1).copied().unwrap_or(None);
            let conv = match (cell.conveyor, conv) {
                (One(Left, Right), (Up, Down) | (Down, Up)) => Two((Left, Right), *conv),
                (One(Right, Left), (Up, Down) | (Down, Up)) => Two((Right, Left), *conv),
                (One(Up, Down), (Left, Right) | (Right, Left)) => Two((Up, Down), *conv),
                (One(Down, Up), (Left, Right) | (Right, Left)) => Two((Down, Up), *conv),
                _ => match (staged, conv) {
                    (One(Left, Right), (Up, Down) | (Down, Up)) => Two((Left, Right), *conv),
                    (One(Right, Left), (Up, Down) | (Down, Up)) => Two((Right, Left), *conv),
                    (One(Up, Down), (Left, Right) | (Right, Left)) => Two((Up, Down), *conv),
                    (One(Down, Up), (Left, Right) | (Right, Left)) => Two((Down, Up), *conv),
                    _ => One(conv.0, conv.1),
                },
            };
            console_log!("conv {:?}: {:?}", pos1, conv);
            self.conveyor_preview.insert(*pos1, conv);
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
