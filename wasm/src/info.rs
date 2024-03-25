use crate::AsteroidColonies;
use asteroid_colonies_logic::{
    building::{BuildingType, Recipe},
    construction::{BuildMenuItem, ConstructionType},
    Inventory, Pos,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct GetBuildingInfoResult {
    type_: BuildingType,
    recipe: Option<Recipe>,
    task: String,
    inventory: Inventory,
    crews: usize,
    max_crews: usize,
}

#[derive(Serialize)]
struct GetConstructionInfoResult {
    type_: ConstructionType,
    recipe: BuildMenuItem,
    ingredients: Inventory,
}

#[derive(Serialize)]
struct GetInfoResult {
    building: Option<GetBuildingInfoResult>,
    construction: Option<GetConstructionInfoResult>,
    energy: usize,
    power_demand: isize,
    power_supply: isize,
    power_capacity: isize,
    transports: usize,
}

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn get_info(&self) -> Result<JsValue, JsValue> {
        // let [ix, iy] = self.transform_pos(x, y);
        let mut building = None;
        let mut construction = None;

        if let Some([ix, iy]) = self.cursor {
            let intersects = |pos: Pos, size: [usize; 2]| {
                pos[0] <= ix
                    && ix < size[0] as i32 + pos[0]
                    && pos[1] <= iy
                    && iy < size[1] as i32 + pos[1]
            };
            building = self
                .game
                .iter_building()
                .find(|b| intersects(b.pos, b.type_.size()))
                .map(|building| {
                    let recipe = building.recipe.clone();
                    GetBuildingInfoResult {
                        type_: building.type_,
                        recipe,
                        task: format!("{}", building.task),
                        inventory: building.inventory.clone(),
                        crews: building.crews,
                        max_crews: building.type_.max_crews(),
                    }
                });
            construction = self.game.iter_construction().find_map(|c| {
                if !intersects(c.pos, c.size()) {
                    return None;
                }
                Some(GetConstructionInfoResult {
                    type_: c.get_type(),
                    recipe: c.recipe.clone(),
                    ingredients: c.ingredients.clone(),
                })
            });
        }

        // We want to count power generation and consumption separately
        let (energy, dischargeable, power_supply, power_demand) = self
            .game
            .iter_building()
            .map(|b| (b.energy.unwrap_or(0), b.power_discharge(), b.power_gen()))
            .fold((0, 0, 0, 0), |acc, (energy, discharge, gen)| {
                (
                    acc.0 + energy,
                    acc.1 + discharge,
                    acc.2 + gen.max(0).abs(),
                    acc.3 + gen.min(0).abs(),
                )
            });

        let result = GetInfoResult {
            building,
            construction,
            energy,
            power_demand,
            power_supply,
            power_capacity: dischargeable + power_supply,
            transports: self.game.num_transports(),
        };

        serde_wasm_bindgen::to_value(&result).map_err(JsValue::from)
    }
}
