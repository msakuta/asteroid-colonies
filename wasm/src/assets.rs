use wasm_bindgen::{JsCast, JsValue};
use web_sys::{js_sys, HtmlImageElement};

use asteroid_colonies_logic::building::BuildingType;

pub(crate) struct Assets {
    pub img_bg: HtmlImageElement,
    pub img_cursor: HtmlImageElement,
    pub img_move_cursor: HtmlImageElement,
    pub img_crew: HtmlImageElement,
    pub img_power_grid: HtmlImageElement,
    pub img_conveyor: HtmlImageElement,
    pub img_atomic_battery: HtmlImageElement,
    pub img_battery: HtmlImageElement,
    pub img_excavator: HtmlImageElement,
    pub img_storage: HtmlImageElement,
    pub img_medium_storage: HtmlImageElement,
    pub img_crew_cabin: HtmlImageElement,
    pub img_assembler: HtmlImageElement,
    pub img_furnace: HtmlImageElement,
    pub img_raw_ore: HtmlImageElement,
    pub img_iron_ingot: HtmlImageElement,
    pub img_copper_ingot: HtmlImageElement,
    pub img_cilicate: HtmlImageElement,
    pub img_gear: HtmlImageElement,
    pub img_wire: HtmlImageElement,
    pub img_circuit: HtmlImageElement,
    pub img_construction: HtmlImageElement,
    pub img_deconstruction: HtmlImageElement,
    pub img_cleanup: HtmlImageElement,
}

impl Assets {
    /// Constructs the assets from resource maps (name => element)
    pub fn new(image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture = |name| -> Result<HtmlImageElement, JsValue> {
            let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(name))
            }) else {
                return Err(JsValue::from_str(&format!("Couldn't find texture: {name}")));
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
            img_cursor: load_texture("cursor")?,
            img_move_cursor: load_texture("move_cursor")?,
            img_crew: load_texture("crew")?,
            img_power_grid: load_texture("power_grid")?,
            img_conveyor: load_texture("conveyor")?,
            img_atomic_battery: load_texture("atomic_battery")?,
            img_battery: load_texture("battery")?,
            img_excavator: load_texture("excavator")?,
            img_storage: load_texture("storage")?,
            img_medium_storage: load_texture("medium_storage")?,
            img_crew_cabin: load_texture("crew_cabin")?,
            img_assembler: load_texture("assembler")?,
            img_furnace: load_texture("furnace")?,
            img_raw_ore: load_texture("raw_ore")?,
            img_iron_ingot: load_texture("iron_ingot")?,
            img_copper_ingot: load_texture("copper_ingot")?,
            img_cilicate: load_texture("cilicate")?,
            img_gear: load_texture("gear")?,
            img_wire: load_texture("wire")?,
            img_circuit: load_texture("circuit")?,
            img_construction: load_texture("construction")?,
            img_deconstruction: load_texture("deconstruction")?,
            img_cleanup: load_texture("cleanup")?,
        })
    }

    pub fn building_to_img(&self, ty: BuildingType) -> &HtmlImageElement {
        match ty {
            BuildingType::Power => &self.img_atomic_battery,
            BuildingType::Excavator => &self.img_excavator,
            BuildingType::Storage => &self.img_storage,
            BuildingType::MediumStorage => &self.img_medium_storage,
            BuildingType::CrewCabin => &self.img_crew_cabin,
            BuildingType::Assembler => &self.img_assembler,
            BuildingType::Furnace => &self.img_furnace,
            _ => panic!("Uncovered building type!"),
        }
    }
}
