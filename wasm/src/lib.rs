mod assets;
mod conveyor;
mod info;
mod render;
mod utils;

use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use asteroid_colonies_logic::{
    building::{Building, BuildingType},
    get_build_menu, AsteroidColoniesGame, Pos, HEIGHT, TILE_SIZE, WIDTH,
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
    move_cursor: Option<Pos>,
    assets: Assets,
    viewport: Viewport,
    debug_draw_chunks: bool,
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
            move_cursor: None,
            assets: Assets::new(image_assets)?,
            viewport: Viewport {
                offset: [
                    -(WIDTH as f64 / 8. - 4.) * TILE_SIZE,
                    -(HEIGHT as f64 / 2. - 8.) * TILE_SIZE,
                ],
                size: [vp_width, vp_height],
            },
            debug_draw_chunks: false,
        })
    }

    pub fn set_size(&mut self, sx: f64, sy: f64) {
        self.viewport.size = [sx, sy];
    }

    pub fn set_cursor(&mut self, x: f64, y: f64) {
        self.cursor = Some(self.transform_pos(x, y));
    }

    pub fn transform_coords(&self, x: f64, y: f64) -> Vec<i32> {
        self.transform_pos(x, y).to_vec()
    }

    pub fn command(&mut self, com: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let [ix, iy] = self.transform_pos(x, y);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside tile"));
        }
        let res = match com {
            "excavate" => self.game.excavate(ix, iy),
            "power" => self.game.build_power_grid(ix, iy),
            "moveItem" => self.game.move_item(ix, iy),
            _ => Err(format!("Unknown command: {}", com)),
        };
        res.map(|r| JsValue::from(r)).map_err(|e| JsValue::from(e))
    }

    pub fn start_move_building(&mut self, x: f64, y: f64) -> bool {
        let pos = self.transform_pos(x, y);
        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= pos[0]
                && pos[0] < size[0] as i32 + b.pos[0]
                && b.pos[1] <= pos[1]
                && pos[1] < size[1] as i32 + b.pos[1]
        };
        if self
            .game
            .iter_building()
            .find(|b| intersects(b))
            .is_some_and(|b| b.type_.is_mobile())
        {
            self.move_cursor = Some(pos);
            true
        } else {
            false
        }
    }

    pub fn move_building(&mut self, dst_x: f64, dst_y: f64) -> Result<JsValue, JsValue> {
        let dpos = self.transform_pos(dst_x, dst_y);
        if let Some(src) = self.move_cursor {
            self.move_cursor = None;
            self.game
                .move_building(src[0], src[1], dpos[0], dpos[1])
                .map_err(JsValue::from)?;
            Ok(serde_wasm_bindgen::to_value(&src)?)
        } else {
            Err(JsValue::from("Select a building to move first"))
        }
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

    pub fn build_plan(&mut self, constructions: Vec<JsValue>) -> Result<(), JsValue> {
        let constructions = constructions
            .into_iter()
            .map(serde_wasm_bindgen::from_value)
            .collect::<Result<Vec<_>, _>>()?;
        self.game.build_plan(&constructions);
        Ok(())
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
        let [ix, iy] = self.transform_pos(x, y);
        self.game
            .set_recipe(ix, iy, Some(name))
            .map_err(JsValue::from)
    }

    pub fn clear_recipe(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
        let [ix, iy] = self.transform_pos(x, y);
        self.game.set_recipe(ix, iy, None).map_err(JsValue::from)
    }

    pub fn cleanup_item(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.cleanup_item([ix, iy]).map_err(JsValue::from)
    }

    pub fn pan(&mut self, x: f64, y: f64) {
        self.viewport.offset[0] += x;
        self.viewport.offset[1] += y;
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        self.game.tick().map_err(JsValue::from)
    }

    pub fn set_debug_draw_chunks(&mut self, v: bool) {
        self.debug_draw_chunks = v;
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

    pub fn deserialize_bin(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.game
            .deserialize_bin(data)
            .map_err(|e| JsValue::from(format!("{e}")))
    }

    pub fn uniformify_tiles(&mut self) {
        self.game.uniformify_tiles();
    }

    pub fn serialize_chunks_digest(&self) -> Result<Vec<u8>, JsValue> {
        self.game
            .serialize_chunks_digest()
            .map_err(|e| JsValue::from(e.to_string()))
    }
}

impl AsteroidColonies {
    fn transform_pos(&self, x: f64, y: f64) -> Pos {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        [ix, iy]
    }
}
