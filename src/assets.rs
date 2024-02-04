use wasm_bindgen::{JsCast, JsValue};
use web_sys::{js_sys, HtmlImageElement};

pub(crate) struct Assets {
    pub img_bg: HtmlImageElement,
    pub img_power: HtmlImageElement,
    pub img_excavator: HtmlImageElement,
}

impl Assets {
    /// Constructs the assets from resource maps (name => element)
    pub fn new(image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture = |name| -> Result<HtmlImageElement, JsValue> {
            let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(name))
            }) else {
                return Err(JsValue::from_str("Couldn't find texture"));
            };
            js_sys::Array::from(&value)
                .to_vec()
                .get(2)
                .cloned()
                .ok_or_else(|| {
                    JsValue::from_str(&format!(
                        "Couldn't convert value to HtmlImageElement: {:?}",
                        name
                    ))
                })?
                .dyn_into::<HtmlImageElement>()
        };

        Ok(Self {
            img_bg: load_texture("bg32")?,
            img_power: load_texture("power")?,
            img_excavator: load_texture("excavator")?,
        })
    }
}
