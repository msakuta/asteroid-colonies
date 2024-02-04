mod utils;

use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

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

#[wasm_bindgen]
pub struct AsteroidColonies {
    buildings: Vec<[i32; 2]>,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            buildings: vec![[3, 4]],
        }
    }

    pub fn render(
        &self,
        context: &CanvasRenderingContext2d,
        img: &HtmlImageElement,
        img2: &HtmlImageElement,
    ) -> Result<(), JsValue> {
        // let width = context.cli();
        // context.clear_rect(0., 0., 32., 32.);
        context.set_fill_style(&JsValue::from("#ff0000"));
        for iy in 0..15 {
            let y = iy as f64 * 32.;
            for ix in 0..20 {
                let x = ix as f64 * 32.;
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, 0., 0., 32., 32., x, y, 32., 32.,
                    )?;
            }
        }

        for building in &self.buildings {
            let x = building[0] as f64 * 32.;
            let y = building[1] as f64 * 32.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img2, 0., 0., 32., 32., x, y, 32., 32.,
            )?;
        }
        Ok(())
    }

    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if let Some(building) = self.buildings.iter().find(|b| b[0] == ix && b[1] == iy) {
            Ok(JsValue::from(format!(
                "Power plant at {}, {}",
                building[0], building[1]
            )))
        } else {
            Ok(JsValue::from(format!("Empty at {ix}, {iy}")))
        }
    }
}
