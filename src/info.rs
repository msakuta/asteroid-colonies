use std::collections::HashMap;

use crate::{
    building::{BuildingType, Recipe},
    ItemType,
};

use super::{AsteroidColonies, Building};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct GetBuildingInfoResult {
    type_: BuildingType,
    recipe: Option<Recipe>,
    task: String,
    inventory: HashMap<ItemType, usize>,
    crews: usize,
    max_crews: usize,
}

#[derive(Serialize)]
struct GetInfoResult {
    building: Option<GetBuildingInfoResult>,
    power_consumed: usize,
    power_capacity: usize,
    transports: usize,
}

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        let intersects = |b: &&Building| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };
        let bldg_result = self.buildings.iter().find(intersects).map(|building| {
            let recipe = building.recipe.cloned();
            GetBuildingInfoResult {
                type_: building.type_,
                recipe,
                task: format!("{:?}", building.task),
                inventory: building.inventory.clone(),
                crews: building.crews,
                max_crews: building.type_.max_crews(),
            }
        });
        // We want to count power generation and consumption separately
        let power_capacity = self
            .buildings
            .iter()
            .map(|b| b.power().max(0))
            .sum::<isize>() as usize;
        let power_consumed = self
            .buildings
            .iter()
            .map(|b| b.power().min(0))
            .sum::<isize>()
            .abs() as usize;

        let result = GetInfoResult {
            building: bldg_result,
            power_consumed,
            power_capacity,
            transports: self.transports.len(),
        };

        serde_wasm_bindgen::to_value(&result).map_err(JsValue::from)
    }
}
