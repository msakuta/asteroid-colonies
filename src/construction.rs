use std::{collections::HashMap, sync::OnceLock};

use crate::{
    building::{pull_inputs, BuildingType},
    task::{GlobalTask, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME},
    ItemType, Pos,
};

use super::{hash_map, AsteroidColonies};

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ConstructionType {
    PowerGrid,
    Conveyor,
    Building(BuildingType),
}

pub(crate) struct Construction {
    type_: ConstructionType,
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

    pub fn new_power_grid(pos: Pos) -> Self {
        static BUILD: OnceLock<BuildMenuItem> = OnceLock::new();
        let recipe = &*BUILD.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::PowerGrid,
            ingredients: hash_map!(ItemType::PowerGridComponent => 1),
            time: BUILD_POWER_GRID_TIME,
        });
        Self::new(recipe, pos)
    }

    pub fn new_conveyor(pos: Pos) -> Self {
        static BUILD: OnceLock<BuildMenuItem> = OnceLock::new();
        let recipe = &*BUILD.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::Conveyor,
            ingredients: hash_map!(ItemType::ConveyorComponent => 1),
            time: BUILD_CONVEYOR_TIME,
        });
        Self::new(recipe, pos)
    }

    pub fn building(&self) -> Option<BuildingType> {
        match self.type_ {
            ConstructionType::Building(b) => Some(b),
            _ => None,
        }
    }

    pub fn size(&self) -> [usize; 2] {
        match self.type_ {
            ConstructionType::Building(b) => b.size(),
            _ => [1; 2],
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct BuildMenuItem {
    pub type_: ConstructionType,
    pub ingredients: HashMap<ItemType, usize>,
    pub time: f64,
}

pub(crate) fn get_build_menu() -> &'static [BuildMenuItem] {
    static BUILD_MENU: OnceLock<Vec<BuildMenuItem>> = OnceLock::new();
    &*BUILD_MENU.get_or_init(|| {
        vec![
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Power),
                ingredients: hash_map!(ItemType::PowerGridComponent => 3),
                time: 120.,
            },
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Storage),
                ingredients: hash_map!(ItemType::IronIngot => 1, ItemType::Cilicate => 5),
                time: 100.,
            },
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Excavator),
                ingredients: hash_map!(ItemType::IronIngot => 3, ItemType::Gear => 2, ItemType::Circuit => 2),
                time: 200.,
            },
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::MediumStorage),
                ingredients: hash_map!(ItemType::IronIngot => 2,  ItemType::Gear => 2, ItemType::Cilicate => 10),
                time: 200.,
            },
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Furnace),
                ingredients: hash_map!(ItemType::IronIngot => 2, ItemType::Wire => 1, ItemType::Cilicate => 6),
                time: 300.,
            },
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Assembler),
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
            self.global_tasks.push(GlobalTask::Build(
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
