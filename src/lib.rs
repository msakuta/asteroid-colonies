mod assets;
mod utils;

use std::fmt::Display;

use assets::Assets;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, CanvasRenderingContext2d, HtmlImageElement};

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

#[derive(Clone, Copy)]
enum CellState {
    Solid,
    Empty,
}

#[derive(Clone, Copy)]
enum BuildingType {
    Power,
    Excavator,
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
        }
    }
}

struct Building {
    pos: [i32; 2],
    type_: BuildingType,
}

const WIDTH: usize = 20;
const HEIGHT: usize = 15;

#[wasm_bindgen]
pub struct AsteroidColonies {
    cells: Vec<CellState>,
    buildings: Vec<Building>,
    assets: Assets,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(image_assets: js_sys::Array) -> Result<AsteroidColonies, JsValue> {
        let mut cells = vec![CellState::Solid; WIDTH * HEIGHT];
        let buildings = vec![
            Building {
                pos: [3, 4],
                type_: BuildingType::Power,
            },
            Building {
                pos: [4, 4],
                type_: BuildingType::Excavator,
            },
        ];
        for building in &buildings {
            let pos = building.pos;
            cells[pos[0] as usize + pos[1] as usize * WIDTH] = CellState::Empty;
        }
        Ok(Self {
            cells,
            buildings,
            assets: Assets::new(image_assets)?,
        })
    }

    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        // let width = context.cli();
        // context.clear_rect(0., 0., 32., 32.);
        context.set_fill_style(&JsValue::from("#ff0000"));
        for (i, cell) in self.cells.iter().enumerate() {
            let iy = i / WIDTH;
            let y = iy as f64 * 32.;
            let ix = i % WIDTH;
            let x = ix as f64 * 32.;
            let (sx, sy) = match cell {
                CellState::Empty => (3. * 32., 3. * 32.),
                CellState::Solid => (0., 0.),
            };
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.assets.img_bg,
                sx,
                sy,
                32.,
                32.,
                x,
                y,
                32.,
                32.,
            )?;
        }

        for building in &self.buildings {
            let img = match building.type_ {
                BuildingType::Power => &self.assets.img_power,
                BuildingType::Excavator => &self.assets.img_excavator,
            };
            let x = building.pos[0] as f64 * 32.;
            let y = building.pos[1] as f64 * 32.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., 32., 32., x, y, 32., 32.,
            )?;
        }
        Ok(())
    }

    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if let Some(building) = self
            .buildings
            .iter()
            .find(|b| b.pos[0] == ix && b.pos[1] == iy)
        {
            Ok(JsValue::from(format!(
                "{} at {}, {}",
                building.type_, building.pos[0], building.pos[1]
            )))
        } else {
            Ok(JsValue::from(format!("Empty at {ix}, {iy}")))
        }
    }

    pub fn excavate(&mut self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        self.cells[ix as usize + iy as usize * WIDTH] = CellState::Empty;
        Ok(JsValue::from(true))
    }
}
