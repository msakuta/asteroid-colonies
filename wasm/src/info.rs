use std::collections::HashMap;

use crate::{render::TILE_SIZE, AsteroidColonies};
use asteroid_colonies_logic::{
    building::{BuildingType, Recipe},
    construction::{BuildMenuItem, ConstructionType},
    Inventory, ItemType, Pos,
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
    power_demand: isize,
    power_supply: isize,
    power_capacity: isize,
    transports: usize,
}

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn get_info(&self, x: f64, y: f64) -> Result<JsValue, JsValue> {
        let ix = (x - self.viewport.offset[0]).div_euclid(TILE_SIZE) as i32;
        let iy = (y - self.viewport.offset[1]).div_euclid(TILE_SIZE) as i32;
        let intersects = |pos: Pos, size: [usize; 2]| {
            pos[0] <= ix
                && ix < size[0] as i32 + pos[0]
                && pos[1] <= iy
                && iy < size[1] as i32 + pos[1]
        };
        let bldg_result = self
            .game
            .iter_building()
            .find(|b| intersects(b.pos, b.type_.size()))
            .map(|building| {
                let recipe = building.recipe.clone();
                GetBuildingInfoResult {
                    type_: building.type_,
                    recipe,
                    task: format!("{:?}", building.task),
                    inventory: building.inventory.clone(),
                    crews: building.crews,
                    max_crews: building.type_.max_crews(),
                }
            });
        let construction = self.game.iter_construction().find_map(|c| {
            if !intersects(c.pos, c.size()) {
                return None;
            }
            Some(GetConstructionInfoResult {
                type_: c.get_type(),
                recipe: c.recipe.clone(),
                ingredients: c.ingredients.clone(),
            })
        });
        // We want to count power generation and consumption separately
        let (power_capacity, power_supply, power_demand) = self
            .game
            .iter_building()
            .map(|b| (b.power_discharge(), b.power_gen()))
            .fold((0, 0, 0), |acc, (cap, gen)| {
                (
                    acc.0 + cap,
                    acc.1 + gen.max(0).abs(),
                    acc.2 + gen.min(0).abs(),
                )
            });

        let result = GetInfoResult {
            building: bldg_result,
            construction,
            power_demand,
            power_supply,
            power_capacity,
            transports: self.game.num_transports(),
        };

        serde_wasm_bindgen::to_value(&result).map_err(JsValue::from)
    }
}
