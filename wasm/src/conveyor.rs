use wasm_bindgen::prelude::*;

use crate::AsteroidColonies;

#[wasm_bindgen]
impl AsteroidColonies {
    /// Preview or stage conveyor build plan.
    pub fn preview_build_conveyor(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        preview: bool,
    ) -> Result<(), JsValue> {
        self.game
            .preview_build_conveyor(x0, y0, x1, y1, preview)
            .map_err(JsValue::from)
    }

    pub fn build_splitter(&mut self, x: i32, y: i32) {
        self.game.build_splitter(x, y);
    }

    pub fn build_merger(&mut self, x: i32, y: i32) {
        self.game.build_merger(x, y);
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
