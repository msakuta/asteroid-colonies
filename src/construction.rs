use std::{collections::HashMap, sync::OnceLock};

use crate::{
    building::{Building, BuildingType},
    crew::{expected_crew_deliveries, Crew},
    push_pull::{pull_inputs, push_outputs, HasInventory},
    task::{Direction, BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME},
    transport::{expected_deliveries, Transport},
    Conveyor, Inventory, ItemType, Pos, WIDTH,
};

use super::{hash_map, AsteroidColonies};

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ConstructionType {
    PowerGrid,
    Conveyor(Conveyor),
    Building(BuildingType),
}

/// A planned location for a construction. It can gather ingredients on site and start building.
pub(crate) struct Construction {
    type_: ConstructionType,
    pub pos: Pos,
    pub ingredients: HashMap<ItemType, usize>,
    pub recipe: &'static BuildMenuItem,
    canceling: bool,
    pub progress: f64,
}

impl Construction {
    fn new_ex(type_: ConstructionType, item: &'static BuildMenuItem, pos: Pos) -> Self {
        Self {
            type_,
            pos,
            ingredients: HashMap::new(),
            recipe: &item,
            canceling: false,
            progress: 0.,
        }
    }

    pub fn new(item: &'static BuildMenuItem, pos: Pos) -> Self {
        Self::new_ex(item.type_, item, pos)
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

    pub fn new_conveyor(pos: Pos, conv: Conveyor) -> Self {
        static BUILD: OnceLock<BuildMenuItem> = OnceLock::new();
        let recipe = &*BUILD.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::Conveyor(Conveyor::One(Direction::Left, Direction::Right)),
            ingredients: hash_map!(ItemType::ConveyorComponent => 1),
            time: BUILD_CONVEYOR_TIME,
        });
        Self::new_ex(ConstructionType::Conveyor(conv), recipe, pos)
    }

    pub fn new_deconstruct(
        building: BuildingType,
        pos: Pos,
        inventory: &Inventory,
    ) -> Option<Self> {
        let con_ty = ConstructionType::Building(building);
        let recipe = get_build_menu().iter().find(|bi| bi.type_ == con_ty)?;
        let mut ingredients = recipe.ingredients.clone();
        for (item, amount) in inventory {
            ingredients.insert(*item, *amount);
        }
        Some(Self {
            type_: con_ty,
            pos,
            ingredients,
            recipe,
            canceling: true,
            progress: recipe.time,
        })
    }

    pub fn get_type(&self) -> ConstructionType {
        self.type_
    }

    pub fn _building(&self) -> Option<BuildingType> {
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

    pub fn canceling(&self) -> bool {
        self.canceling
    }

    pub fn toggle_cancel(&mut self) {
        self.canceling = !self.canceling;
    }

    pub fn progress(&self) -> f64 {
        self.progress
    }

    pub fn ingredients_satisfied(&self) -> bool {
        self.recipe.ingredients.iter().all(|(ty, recipe_amount)| {
            crate::console_log!(
                "ingredients_satisfied {:?} => {:?}",
                ty,
                self.ingredients.get(ty)
            );
            *recipe_amount <= self.ingredients.get(ty).copied().unwrap_or(0)
        })
    }

    pub fn required_ingredients<'a>(
        &'a self,
        transports: &'a [Transport],
        crews: &'a [Crew],
    ) -> Box<dyn Iterator<Item = (ItemType, usize)> + 'a> {
        if self.canceling {
            return Box::new(std::iter::empty());
        }
        let expected = expected_deliveries(transports, self.pos);
        let crew_expected = expected_crew_deliveries(crews, self.pos);
        Box::new(
            self.recipe
                .ingredients
                .iter()
                .filter_map(move |(ty, recipe_count)| {
                    let required_amount = self.ingredients.get(ty).copied().unwrap_or(0)
                        + expected.get(ty).copied().unwrap_or(0)
                        + crew_expected.get(ty).copied().unwrap_or(0);
                    if *recipe_count <= required_amount {
                        None
                    } else {
                        Some((*ty, required_amount))
                    }
                }),
        )
    }

    pub fn extra_ingredients<'a>(&'a self) -> Box<dyn Iterator<Item = (ItemType, usize)> + 'a> {
        // Deconstruct first to allow retrieving ingredients
        if !self.canceling || 0. < self.progress {
            return Box::new(std::iter::empty());
        }
        Box::new(
            self.ingredients
                .iter()
                .filter_map(|(i, v)| if 0 < *v { Some((*i, *v)) } else { None }),
        )
    }
}

impl HasInventory for Construction {
    fn pos(&self) -> Pos {
        self.pos
    }

    fn size(&self) -> [usize; 2] {
        self.size()
    }

    fn inventory(&mut self) -> &mut HashMap<ItemType, usize> {
        &mut self.ingredients
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
        for (i, construction) in self.constructions.iter_mut().enumerate() {
            if construction.canceling {
                if construction.ingredients.is_empty() {
                    to_delete.push(i);
                } else if construction.progress <= 0. {
                    push_outputs(
                        &self.cells,
                        &mut self.transports,
                        construction,
                        &mut self.buildings,
                        &mut [],
                        &|_| true,
                    );
                    crate::console_log!("Pushed out after: {:?}", construction.ingredients);
                }
            } else {
                pull_inputs(
                    &construction.recipe.ingredients,
                    &&self.cells[..],
                    &mut self.transports,
                    construction.pos,
                    construction.size(),
                    &mut construction.ingredients,
                    &mut self.buildings,
                    &mut [],
                );
                // TODO: should we always use the same amount of time to deconstruct as construction?
                // Some buildings should be easier to deconstruct than construct.
                if construction.progress < construction.recipe.time {
                    continue;
                }
                let pos = construction.pos;
                match construction.type_ {
                    ConstructionType::Building(ty) => {
                        self.buildings.push(Building::new(pos, ty));
                    }
                    ConstructionType::PowerGrid => {
                        self.cells[pos[0] as usize + pos[1] as usize * WIDTH].power_grid = true;
                    }
                    ConstructionType::Conveyor(conv) => {
                        self.cells[pos[0] as usize + pos[1] as usize * WIDTH].conveyor = conv;
                    }
                }
                to_delete.push(i);
            }
        }

        for i in to_delete.iter().rev() {
            self.constructions.remove(*i);
        }
    }
}
