mod assets;
mod conveyor;
mod info;
mod render;
mod utils;

use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use asteroid_colonies_logic::{
    building::BuildingType, get_build_menu, AsteroidColoniesGame, Pos, HEIGHT, TILE_SIZE, WIDTH,
};

use crate::{assets::Assets, render::calculate_back_image};

#[macro_export]
macro_rules! hash_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
    { } => {
        ::std::collections::HashMap::new()
    }
}

#[macro_export]
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

struct Viewport {
    /// View offset in pixels
    offset: [f64; 2],
    /// Viewport size in pixels
    size: [f64; 2],
}

#[wasm_bindgen]
pub struct AsteroidColonies {
    game: AsteroidColoniesGame,
    cursor: Option<Pos>,
    assets: Assets,
    viewport: Viewport,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(
        image_assets: js_sys::Array,
        vp_width: f64,
        vp_height: f64,
    ) -> Result<AsteroidColonies, JsValue> {
        Ok(Self {
            game: AsteroidColoniesGame::new(Some(Box::new(calculate_back_image)))?,
            cursor: None,
            assets: Assets::new(image_assets)?,
            viewport: Viewport {
                offset: [0.; 2],
                size: [vp_width, vp_height],
            },
        })
    }

    pub fn set_size(&mut self, sx: f64, sy: f64) {
        self.viewport.size = [sx, sy];
    }

    pub fn set_cursor(&mut self, x: f64, y: f64) {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.cursor = Some([ix, iy]);
    }

    pub fn command(&mut self, com: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        let res = match com {
            "excavate" => self.game.excavate(ix, iy),
            "power" => self.game.build_power_grid(ix, iy),
            "moveItem" => self.game.move_item(ix, iy),
            _ => Err(format!("Unknown command: {}", com)),
        };
        res.map(|r| JsValue::from(r)).map_err(|e| JsValue::from(e))
    }

    pub fn move_building(
        &mut self,
        src_x: f64,
        src_y: f64,
        dst_x: f64,
        dst_y: f64,
    ) -> Result<(), JsValue> {
        let ix = (src_x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (src_y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let dx = (dst_x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let dy = (dst_y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game
            .move_building(ix, iy, dx, dy)
            .map_err(|e| JsValue::from(e))
    }

    pub fn build(&mut self, x: f64, y: f64, type_: JsValue) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let type_: BuildingType = serde_wasm_bindgen::from_value(type_)?;
        self.game.build(ix, iy, type_).map_err(|e| JsValue::from(e))
    }

    pub fn cancel_build(&mut self, x: f64, y: f64) {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.cancel_build(ix, iy)
    }

    /// Puts a task to deconstruct a building. It is different from `cancel_build` in that it destroys already built ones.
    pub fn deconstruct(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.deconstruct(ix, iy).map_err(|e| JsValue::from(e))
    }

    pub fn get_recipes(&self, x: f64, y: f64) -> Result<Vec<JsValue>, JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let recipes = self.game.get_recipes(ix, iy).map_err(JsValue::from)?;

        recipes
            .into_iter()
            .map(|recipe| serde_wasm_bindgen::to_value(recipe))
            .collect::<Result<_, _>>()
            .map_err(JsValue::from)
    }

    pub fn set_recipe(&mut self, x: f64, y: f64, name: &str) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.set_recipe(ix, iy, name).map_err(JsValue::from)
    }

    pub fn pan(&mut self, x: f64, y: f64) {
        self.viewport.offset[0] += x;
        self.viewport.offset[1] += y;
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        self.game.tick().map_err(JsValue::from)
    }

    pub fn get_build_menu(&self) -> Result<Vec<JsValue>, JsValue> {
        get_build_menu()
            .iter()
            .map(|s| serde_wasm_bindgen::to_value(&s).map_err(JsValue::from))
            .collect()
    }

    pub fn deserialize(&mut self, data: &str) -> Result<(), JsValue> {
        self.game
            .deserialize(data.as_bytes())
            .map_err(|e| JsValue::from(format!("{e}")))
    }
}
