use wasm_bindgen::prelude::*;

use crate::{render::TILE_SIZE, AsteroidColonies};

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
        let ix0 = (x0 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy0 = (y0 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let ix1 = (x1 - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy1 = (y1 - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game
            .preview_build_conveyor(ix0, iy0, ix1, iy1, preview)
            .map_err(JsValue::from)
    }

    pub fn build_splitter(&mut self, x: f64, y: f64) {
        let ix0 = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy0 = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.build_splitter(ix0, iy0);
    }

    pub fn build_merger(&mut self, x: f64, y: f64) {
        let ix0 = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy0 = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.build_merger(ix0, iy0);
    }

    pub fn cancel_build_conveyor(&mut self, preview: bool) {
        self.game.cancel_build_conveyor(preview);
    }

    pub fn commit_build_conveyor(&mut self) -> Result<Vec<JsValue>, JsValue> {
        self.game
            .commit_build_conveyor()
            .iter()
            .map(serde_wasm_bindgen::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(JsValue::from)
    }
}
