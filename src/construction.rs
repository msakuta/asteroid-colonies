use std::{collections::HashMap, sync::OnceLock};

use crate::{
    building::{pull_inputs, BuildingType},
    task::GlobalTask,
    ItemType, Pos,
};

use super::{hash_map, AsteroidColonies};

use serde::Serialize;
use wasm_bindgen::prelude::*;

pub(crate) struct Construction {
    pub type_: BuildingType,
    pub pos: Pos,
    pub ingredients: HashMap<ItemType, usize>,
    pub recipe: &'static BuildMenuItem,
}

impl Construction {
    pub fn new(item: &'static BuildMenuItem, pos: Pos) -> Self {
        Self {
            type_: item.type_,
            pos,
            ingredients: HashMap::new(),
            recipe: &item,
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct BuildMenuItem {
    pub type_: BuildingType,
    pub ingredients: HashMap<ItemType, usize>,
    pub time: f64,
}

pub(crate) fn get_build_menu() -> &'static [BuildMenuItem] {
    static BUILD_MENU: OnceLock<Vec<BuildMenuItem>> = OnceLock::new();
    &*BUILD_MENU.get_or_init(|| {
        vec![
            BuildMenuItem {
                type_: BuildingType::Power,
                ingredients: hash_map!(ItemType::PowerGridComponent => 3),
                time: 120.,
            },
            BuildMenuItem {
                type_: BuildingType::Storage,
                ingredients: hash_map!(ItemType::IronIngot => 1, ItemType::Cilicate => 5),
                time: 100.,
            },
            BuildMenuItem {
                type_: BuildingType::Excavator,
                ingredients: hash_map!(ItemType::IronIngot => 3, ItemType::Gear => 2, ItemType::Circuit => 2),
                time: 200.,
            },
            BuildMenuItem {
                type_: BuildingType::MediumStorage,
                ingredients: hash_map!(ItemType::IronIngot => 2,  ItemType::Gear => 2, ItemType::Cilicate => 10),
                time: 200.,
            },
            BuildMenuItem {
                type_: BuildingType::Furnace,
                ingredients: hash_map!(ItemType::IronIngot => 2, ItemType::Wire => 1, ItemType::Cilicate => 6),
                time: 300.,
            },
            BuildMenuItem {
                type_: BuildingType::Assembler,
                ingredients: hash_map!(ItemType::AssemblerComponent => 4),
                time: 300.,
            },
        ]
    })
}

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn get_build_menu(&self) -> Result<Vec<JsValue>, JsValue> {
        get_build_menu()
            .iter()
            .map(|s| serde_wasm_bindgen::to_value(&s).map_err(JsValue::from))
            .collect()
    }
}

impl AsteroidColonies {
    pub(super) fn process_constructions(&mut self) {
        let mut to_delete = vec![];
        'outer: for (i, construction) in self.constructions.iter_mut().enumerate() {
            pull_inputs(
                &construction.recipe.ingredients,
                &self.cells,
                &mut self.transports,
                construction.pos,
                &mut construction.ingredients,
                &mut self.buildings,
                &mut [],
            );
            for (ty, &required) in &construction.recipe.ingredients {
                let arrived = construction.ingredients.get(ty).copied().unwrap_or(0);
                if arrived < required {
                    continue 'outer;
                }
            }
            self.global_tasks.push(GlobalTask::BuildBuilding(
                construction.recipe.time,
                construction.pos,
                construction.recipe,
            ));
            to_delete.push(i);
        }

        for i in to_delete.iter().rev() {
            self.constructions.remove(*i);
        }
    }
}
