use wasm_bindgen::{JsCast, JsValue};
use web_sys::{js_sys, HtmlImageElement};

pub(crate) struct Assets {
    pub img_bg: HtmlImageElement,
    pub img_power_grid: HtmlImageElement,
    pub img_conveyor: HtmlImageElement,
    pub img_power: HtmlImageElement,
    pub img_excavator: HtmlImageElement,
    pub img_storage: HtmlImageElement,
    pub img_crew_cabin: HtmlImageElement,
    pub img_assembler: HtmlImageElement,
    pub img_furnace: HtmlImageElement,
    pub img_iron_ingot: HtmlImageElement,
    pub img_copper_ingot: HtmlImageElement,
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
            img_power_grid: load_texture("power_grid")?,
            img_conveyor: load_texture("conveyor")?,
            img_power: load_texture("power")?,
            img_excavator: load_texture("excavator")?,
            img_storage: load_texture("storage")?,
            img_crew_cabin: load_texture("crew_cabin")?,
            img_assembler: load_texture("assembler")?,
            img_furnace: load_texture("furnace")?,
            img_iron_ingot: load_texture("iron_ingot")?,
            img_copper_ingot: load_texture("copper_ingot")?,
        })
    }
}
