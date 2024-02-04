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
pub fn say_hello() {
    console_log!("Hello, asteroid-colonies!");
}

#[wasm_bindgen]
pub fn render(context: &CanvasRenderingContext2d, img: &HtmlImageElement) -> Result<(), JsValue> {
    // let width = context.cli();
    // context.clear_rect(0., 0., 32., 32.);
    context.set_fill_style(&JsValue::from("#ff0000"));
    for iy in 0..5 {
        let y = iy as f64 * 32.;
        for ix in 0..10 {
            let x = ix as f64 * 32.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., 32., 32., x, y, 32., 32.,
            )?;
        }
    }
    // context.fill_rect(0., 0., 32., 32.);
    // if let Some(item) = self.tool_belt.get(tool_index).unwrap_or(&None) {
    //     if Some(SelectedItem::ToolBelt(tool_index)) == self.selected_item {
    //         context.set_fill_style(&js_str!("#00ffff"));
    //         context.fill_rect(0., 0., 32., 32.);
    //     }
    //     let mut tool = self.new_structure(item, &Position { x: 0, y: 0 })?;
    //     tool.set_rotation(&self.tool_rotation).ok();
    //     for depth in 0..3 {
    //         tool.draw(self, context, depth, true)?;
    //     }
    // }
    Ok(())
}
