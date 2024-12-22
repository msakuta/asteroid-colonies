use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use crate::{
    building::{Building, BuildingType},
    crew::{expected_crew_deliveries, Crew},
    direction::Direction,
    entity::EntitySet,
    inventory::{CountableInventory, Inventory},
    items::ItemType,
    push_pull::{pull_inputs, push_outputs, HasInventory},
    task::{BUILD_CONVEYOR_TIME, BUILD_POWER_GRID_TIME},
    transport::{expected_deliveries, Transport, TransportId},
    Conveyor, Pos,
};

use super::{hash_map, AsteroidColoniesGame};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConstructionType {
    PowerGrid,
    Conveyor(Conveyor),
    Building(BuildingType),
}

/// A planned location for a construction. It can gather ingredients on site and start building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Construction {
    type_: ConstructionType,
    pub pos: Pos,
    pub ingredients: Inventory,
    pub recipe: BuildMenuItem,
    canceling: bool,
    pub progress: f64,
    #[serde(skip)]
    /// A cache of expected transports
    expected_transports: HashSet<TransportId>,
}

impl Construction {
    fn new_ex(
        type_: ConstructionType,
        item: &'static BuildMenuItem,
        pos: Pos,
        canceling: bool,
    ) -> Self {
        Self {
            type_,
            pos,
            ingredients: if canceling {
                item.ingredients.iter().map(|(k, v)| (*k, *v)).collect()
            } else {
                Inventory::new()
            },
            recipe: (*item).clone(),
            canceling,
            progress: if canceling { item.time } else { 0. },
            expected_transports: HashSet::new(),
        }
    }

    pub fn new(item: &'static BuildMenuItem, pos: Pos) -> Self {
        Self::new_ex(item.type_, item, pos, false)
    }

    pub fn new_power_grid(pos: Pos, canceling: bool) -> Self {
        static BUILD: OnceLock<BuildMenuItem> = OnceLock::new();
        let recipe = &*BUILD.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::PowerGrid,
            ingredients: hash_map!(ItemType::PowerGridComponent => 1),
            time: BUILD_POWER_GRID_TIME,
        });
        Self::new_ex(ConstructionType::PowerGrid, recipe, pos, canceling)
    }

    fn build_recipe() -> &'static BuildMenuItem {
        static BUILD: OnceLock<BuildMenuItem> = OnceLock::new();
        &*BUILD.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::Conveyor(Conveyor::One(Direction::Left, Direction::Right)),
            ingredients: hash_map!(ItemType::ConveyorComponent => 1),
            time: BUILD_CONVEYOR_TIME,
        })
    }

    fn splitter_recipe() -> &'static BuildMenuItem {
        static BUILD_SPLITTER: OnceLock<BuildMenuItem> = OnceLock::new();
        &*BUILD_SPLITTER.get_or_init(|| BuildMenuItem {
            type_: ConstructionType::Conveyor(Conveyor::One(Direction::Left, Direction::Right)),
            ingredients: hash_map!(ItemType::ConveyorComponent => 1, ItemType::Circuit => 1, ItemType::Gear => 1),
            time: BUILD_CONVEYOR_TIME,
        })
    }

    pub fn new_conveyor(pos: Pos, conv: Conveyor, canceling: bool) -> Self {
        if matches!(conv, Conveyor::Splitter(_) | Conveyor::Merger(_)) {
            Self::new_ex(
                ConstructionType::Conveyor(conv),
                Self::splitter_recipe(),
                pos,
                canceling,
            )
        } else {
            Self::new_ex(
                ConstructionType::Conveyor(conv),
                Self::build_recipe(),
                pos,
                canceling,
            )
        }
    }

    pub fn new_deconstruct(
        building: BuildingType,
        pos: Pos,
        inventory: &Inventory,
    ) -> Option<Self> {
        let con_ty = ConstructionType::Building(building);
        let recipe = get_build_menu().iter().find(|bi| bi.type_ == con_ty)?;
        let mut ingredients: CountableInventory =
            recipe.ingredients.iter().map(|(k, v)| (*k, *v)).collect();
        for (item, amount) in inventory {
            ingredients.insert(*item, *amount);
        }
        Some(Self {
            type_: con_ty,
            pos,
            ingredients: ingredients.into(),
            recipe: recipe.clone(),
            canceling: true,
            progress: recipe.time,
            expected_transports: HashSet::new(),
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

    pub fn intersects(&self, pos: Pos) -> bool {
        let size = self.size();
        self.pos[0] <= pos[0]
            && pos[0] <= self.pos[0] + size[0] as i32
            && self.pos[1] <= pos[1]
            && pos[1] <= self.pos[1] + size[1] as i32
    }

    pub fn intersects_rect(&self, pos: Pos, other_size: [usize; 2]) -> bool {
        let size = self.size();
        self.pos[0] < pos[0] + other_size[0] as i32
            && pos[0] < self.pos[0] + size[0] as i32
            && self.pos[1] < pos[1] + other_size[1] as i32
            && pos[1] < self.pos[1] + size[1] as i32
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
        self.recipe
            .ingredients
            .iter()
            .all(|(ty, recipe_amount)| *recipe_amount <= self.ingredients.get(ty))
    }

    pub fn required_ingredients<'a>(
        &'a self,
        transports: &'a EntitySet<Transport>,
        crews: &'a EntitySet<Crew>,
    ) -> Box<dyn Iterator<Item = (ItemType, usize)> + 'a> {
        if self.canceling {
            return Box::new(std::iter::empty());
        }
        let expected = expected_deliveries(transports, &self.expected_transports);
        let crew_expected = expected_crew_deliveries(crews, self.pos);
        Box::new(
            self.recipe
                .ingredients
                .iter()
                .filter_map(move |(ty, recipe_count)| {
                    let required_amount = self.ingredients.get(ty)
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

    pub fn insert_expected_transports(&mut self, id: TransportId) {
        self.expected_transports.insert(id);
    }

    pub fn clear_expected(&mut self, id: TransportId) {
        self.expected_transports.remove(&id);
    }

    pub fn clear_expected_all(&mut self) {
        self.expected_transports.clear();
    }
}

impl HasInventory for Construction {
    fn pos(&self) -> Pos {
        self.pos
    }

    fn size(&self) -> [usize; 2] {
        self.size()
    }

    fn inventory(&mut self) -> &mut Inventory {
        &mut self.ingredients
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BuildMenuItem {
    pub type_: ConstructionType,
    pub ingredients: HashMap<ItemType, usize>,
    pub time: f64,
}

pub fn get_build_menu() -> &'static [BuildMenuItem] {
    static BUILD_MENU: OnceLock<Vec<BuildMenuItem>> = OnceLock::new();
    &*BUILD_MENU.get_or_init(|| {
        vec![
            BuildMenuItem {
                type_: ConstructionType::Building(BuildingType::Battery),
                ingredients: hash_map!(ItemType::Battery => 2, ItemType::IronIngot => 1),
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

impl AsteroidColoniesGame {
    pub(super) fn process_constructions(&mut self) {
        self.constructions.retain(|construction| {
            if construction.canceling {
                if construction.ingredients.is_empty() {
                    return false;
                } else if construction.progress <= 0. {
                    push_outputs(
                        &self.tiles,
                        &mut self.transports,
                        construction,
                        &self.buildings,
                        &|_| true,
                    );
                    crate::console_log!("Pushed out after: {:?}", construction.ingredients);
                }
            } else {
                let size = construction.size();
                pull_inputs(
                    &construction.recipe.ingredients,
                    &self.tiles,
                    &mut self.transports,
                    &mut construction.expected_transports,
                    construction.pos,
                    size,
                    &mut construction.ingredients,
                    &self.buildings,
                );
                // TODO: should we always use the same amount of time to deconstruct as construction?
                // Some buildings should be easier to deconstruct than construct.
                if construction.progress < construction.recipe.time {
                    return true;
                }
                let pos = construction.pos;
                match construction.type_ {
                    ConstructionType::Building(ty) => {
                        self.buildings.insert(Building::new(pos, ty));
                    }
                    ConstructionType::PowerGrid => {
                        if let Some(tile) = self.tiles.try_get_mut(pos) {
                            tile.power_grid = true;
                        }
                    }
                    ConstructionType::Conveyor(conv) => {
                        if let Some(tile) = self.tiles.try_get_mut(pos) {
                            tile.conveyor = conv;
                        }
                    }
                }
                return false;
            }
            true
        });
    }
}
