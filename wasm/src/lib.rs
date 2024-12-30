mod assets;
mod conveyor;
mod info;
mod render;
mod utils;
mod gl {
    pub mod assets;
    mod render_gl;
    pub mod shader_bundle;
    mod utils;
}

use wasm_bindgen::prelude::*;
use web_sys::{js_sys, WebGlRenderingContext};

use asteroid_colonies_logic::{
    building::BuildingType, get_build_menu, AsteroidColoniesGame, Conveyor, ItemType, Pos,
    TileState, HEIGHT, TILE_SIZE, WIDTH,
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

/// format-like macro that returns js_sys::String
#[macro_export]
macro_rules! js_str {
    ($fmt:expr, $($arg1:expr),*) => {
        JsValue::from_str(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        JsValue::from_str($fmt)
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

#[allow(dead_code)]
fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

#[allow(dead_code)]
fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

const MAX_SCALE: f64 = 4.;
const MIN_SCALE: f64 = 0.5;

struct Viewport {
    /// View offset in pixels
    offset: [f64; 2],
    /// Viewport size in pixels
    size: [f64; 2],
    /// Zoom level
    scale: f64,
}

#[wasm_bindgen]
pub struct AsteroidColonies {
    game: AsteroidColoniesGame,
    cursor: Option<Pos>,
    move_cursor: Option<Pos>,
    move_item_cursor: Option<Pos>,
    assets: Assets,
    gl_assets: Option<gl::assets::Assets>,
    viewport: Viewport,
    debug_draw_chunks: bool,
    draw_ore_overlay: bool,
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
            move_item_cursor: None,
            assets: Assets::new(image_assets)?,
            gl_assets: None,
            viewport: Viewport {
                offset: [
                    -(WIDTH as f64 / 8. - 4.) * TILE_SIZE,
                    -(HEIGHT as f64 / 2. - 8.) * TILE_SIZE,
                ],
                size: [vp_width, vp_height],
                scale: 1.,
            },
            debug_draw_chunks: false,
            draw_ore_overlay: false,
        })
    }

    /// Load WebGL assets. Delayed from construction of AsteroidColonies instance, because
    /// the assets must be associated with the canvas.
    pub fn load_gl_assets(
        &mut self,
        gl: &WebGlRenderingContext,
        image_assets: js_sys::Array,
    ) -> Result<(), JsValue> {
        let assets = gl::assets::Assets::new(gl, image_assets)?;
        self.gl_assets = Some(assets);
        Ok(())
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

    pub fn is_excavatable_at(&self, x: i32, y: i32) -> Result<bool, JsValue> {
        Ok(matches!(self.game.tiles()[[x, y]].state, TileState::Solid))
    }

    pub fn command(&mut self, com: &str, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let [ix, iy] = self.transform_pos(x, y);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside tile"));
        }
        let res = match com {
            "excavate" => self.game.excavate(ix, iy),
            "power" => self.game.build_power_grid(ix, iy),
            _ => Err(format!("Unknown command: {}", com)),
        };
        res.map(|r| JsValue::from(r)).map_err(|e| JsValue::from(e))
    }

    pub fn excavate(&mut self, ix: i32, iy: i32) -> Result<bool, JsValue> {
        self.game.excavate(ix, iy).map_err(JsValue::from)
    }

    pub fn build_power_grid(&mut self, ix: i32, iy: i32) -> Result<bool, JsValue> {
        self.game.build_power_grid(ix, iy).map_err(JsValue::from)
    }

    pub fn start_move_item(&mut self, x: i32, y: i32) -> bool {
        let pos = [x, y];
        if self.game.iter_building().any(|b| b.intersects(pos)) {
            self.move_item_cursor = Some(pos);
            true
        } else {
            false
        }
    }

    pub fn move_item(&mut self, dst_x: f64, dst_y: f64, item: JsValue) -> Result<JsValue, JsValue> {
        let item: ItemType = serde_wasm_bindgen::from_value(item)?;
        let dpos = self.transform_pos(dst_x, dst_y);
        let src = self
            .move_item_cursor
            .ok_or_else(|| JsValue::from("Select a building to move items from first"))?;
        self.move_item_cursor = None;
        self.game
            .move_item(src, dpos, item)
            .map_err(JsValue::from)?;
        Ok(serde_wasm_bindgen::to_value(&src)?)
    }

    pub fn start_move_building(&mut self, ix: i32, iy: i32) -> Result<(), JsValue> {
        let pos = [ix, iy];
        let bldg = self
            .game
            .iter_building()
            .find(|b| b.intersects(pos))
            .ok_or_else(|| JsValue::from("Building to move does not exist"))?;
        if !bldg.type_.is_mobile() {
            return Err(JsValue::from("The building is not mobile"));
        }
        self.move_cursor = Some(pos);
        Ok(())
    }

    pub fn move_building(&mut self, dst_x: f64, dst_y: f64) -> Result<JsValue, JsValue> {
        let dpos = self.transform_pos(dst_x, dst_y);
        if let Some(src) = self.move_cursor {
            self.move_cursor = None;
            self.game.move_building(src, dpos).map_err(JsValue::from)?;
            Ok(serde_wasm_bindgen::to_value(&src)?)
        } else {
            Err(JsValue::from("Select a building to move first"))
        }
    }

    pub fn build(&mut self, ix: i32, iy: i32, type_: JsValue) -> Result<(), JsValue> {
        let type_: BuildingType = serde_wasm_bindgen::from_value(type_)?;
        self.game.build(ix, iy, type_).map_err(|e| JsValue::from(e))
    }

    pub fn cancel_build(&mut self) -> Result<(), JsValue> {
        let [ix, iy] = self.cursor.ok_or("Cursor was not selected")?;
        self.game.cancel_build(ix, iy);
        Ok(())
    }

    pub fn find_building(&self, x: i32, y: i32) -> Result<bool, JsValue> {
        Ok(self.game.iter_building().any(|c| c.intersects([x, y])))
    }

    pub fn find_construction(&self, x: i32, y: i32) -> Result<bool, JsValue> {
        Ok(self.game.iter_construction().any(|c| c.intersects([x, y])))
    }

    pub fn has_conveyor(&self) -> Result<bool, JsValue> {
        let pos = self.cursor.ok_or("Cursor was not selected")?;
        Ok(!matches!(self.game.tiles()[pos].conveyor, Conveyor::None))
    }

    pub fn has_power_grid(&self) -> Result<bool, JsValue> {
        let pos = self.cursor.ok_or("Cursor was not selected")?;
        Ok(self.game.tiles()[pos].power_grid)
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
    pub fn deconstruct(&mut self) -> Result<(), JsValue> {
        let [ix, iy] = self.cursor.ok_or("Cursor was not selected")?;
        Ok(self.game.deconstruct(ix, iy)?)
    }

    /// Puts a task to deconstruct a conveyor.
    pub fn deconstruct_conveyor(&mut self) -> Result<(), JsValue> {
        let [ix, iy] = self.cursor.ok_or("Cursor was not selected")?;
        Ok(self.game.deconstruct_conveyor(ix, iy)?)
    }

    /// Puts a task to deconstruct a power grid.
    pub fn deconstruct_power_grid(&mut self) -> Result<(), JsValue> {
        let [ix, iy] = self.cursor.ok_or("Cursor was not selected")?;
        Ok(self.game.deconstruct_power_grid(ix, iy)?)
    }

    pub fn get_recipes(&self, ix: i32, iy: i32) -> Result<Vec<JsValue>, JsValue> {
        let recipes = self.game.get_recipes(ix, iy).map_err(JsValue::from)?;

        recipes
            .into_iter()
            .map(|recipe| serde_wasm_bindgen::to_value(recipe))
            .collect::<Result<_, _>>()
            .map_err(JsValue::from)
    }

    pub fn set_recipe(&mut self, ix: i32, iy: i32, name: &str) -> Result<(), JsValue> {
        self.game
            .set_recipe(ix, iy, Some(name))
            .map_err(JsValue::from)
    }

    pub fn clear_recipe(&mut self, ix: i32, iy: i32) -> Result<(), JsValue> {
        self.game.set_recipe(ix, iy, None).map_err(JsValue::from)
    }

    pub fn cleanup_item(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        self.game.cleanup_item([ix, iy]).map_err(JsValue::from)
    }

    pub fn get_inventory(&self) -> Result<JsValue, JsValue> {
        let inventory = self.cursor.and_then(|cursor| {
            self.game
                .iter_building()
                .find(|b| b.intersects(cursor))
                .map(|building| building.inventory.clone())
        });
        serde_wasm_bindgen::to_value(&inventory).map_err(JsValue::from)
    }

    pub fn pan(&mut self, x: f64, y: f64) {
        self.viewport.offset[0] += x / self.viewport.scale;
        self.viewport.offset[1] += y / self.viewport.scale;
    }

    pub fn get_cursor(&self) -> Option<Vec<i32>> {
        self.cursor.map(|c| c.to_vec())
    }

    pub fn get_pos(&self) -> Vec<f64> {
        self.viewport.offset.to_vec()
    }

    pub fn get_zoom(&self) -> f64 {
        self.viewport.scale
    }

    pub fn set_zoom(&mut self, x: f64, y: f64, scale: f64) {
        let new_scale = (self.viewport.scale * scale).clamp(MIN_SCALE, MAX_SCALE);
        self.viewport.offset[0] +=
            (x as f64 / self.viewport.scale) * (1. - new_scale / self.viewport.scale);
        self.viewport.offset[1] +=
            (y as f64 / self.viewport.scale) * (1. - new_scale / self.viewport.scale);

        self.viewport.scale = new_scale;
    }

    pub fn change_zoom(&mut self, x: f64, y: f64, v: f64) {
        let new_scale = if v < 0. {
            (self.viewport.scale * 1.2).min(MAX_SCALE)
        } else {
            (self.viewport.scale / 1.2).max(MIN_SCALE)
        };
        self.viewport.offset[0] +=
            (x as f64 / self.viewport.scale) * (1. - new_scale / self.viewport.scale);
        self.viewport.offset[1] +=
            (y as f64 / self.viewport.scale) * (1. - new_scale / self.viewport.scale);

        self.viewport.scale = new_scale;
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        self.game.tick().map_err(JsValue::from)
    }

    pub fn set_debug_draw_chunks(&mut self, v: bool) {
        self.debug_draw_chunks = v;
    }

    pub fn set_draw_ore_overlay(&mut self, v: bool) {
        self.draw_ore_overlay = v;
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
        let vp = &self.viewport;
        let ix = (x / vp.scale - vp.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y / vp.scale - vp.offset[1]).div_euclid(TILE_SIZE) as i32;
        [ix, iy]
    }
}
