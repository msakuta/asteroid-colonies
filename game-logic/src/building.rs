use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use ::serde::{Deserialize, Serialize};

use crate::{
    construction::Construction,
    hash_map,
    push_pull::{pull_inputs, push_outputs},
    task::{GlobalTask, Task, RAW_ORE_SMELT_TIME},
    tile::Tiles,
    transport::find_multipath,
    AsteroidColoniesGame, Cell, CellState, Crew, ItemType, Transport, Xor128, WIDTH,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BuildingType {
    Power,
    Excavator,
    Storage,
    MediumStorage,
    CrewCabin,
    Assembler,
    Furnace,
}

impl BuildingType {
    pub fn capacity(&self) -> usize {
        match self {
            Self::Power => 5,
            Self::Excavator => 10,
            Self::Storage => 20,
            Self::MediumStorage => 100,
            Self::CrewCabin => 20,
            Self::Assembler => 40,
            Self::Furnace => 30,
        }
    }

    pub fn size(&self) -> [usize; 2] {
        match self {
            Self::MediumStorage | Self::CrewCabin | Self::Assembler | Self::Furnace => [2, 2],
            _ => [1, 1],
        }
    }

    pub fn max_crews(&self) -> usize {
        match self {
            Self::CrewCabin => 4,
            _ => 0,
        }
    }

    /// Return the amount of base generating/consuming power
    pub fn power(&self) -> isize {
        match self {
            Self::Power => 500,
            Self::CrewCabin => -100,
            Self::Excavator => -10,
            Self::Storage => 0,
            Self::MediumStorage => 0,
            Self::Assembler => -20,
            Self::Furnace => -10,
        }
    }

    pub fn is_storage(&self) -> bool {
        matches!(self, Self::Storage | Self::MediumStorage)
    }

    /// Is it a movable building?
    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::Excavator)
    }
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
            Self::Storage => write!(f, "Storage"),
            Self::MediumStorage => write!(f, "MediumStorage"),
            Self::CrewCabin => write!(f, "CrewCabin"),
            Self::Assembler => write!(f, "Assembler"),
            Self::Furnace => write!(f, "Furnace"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub inputs: HashMap<ItemType, usize>,
    pub outputs: HashMap<ItemType, usize>,
    pub time: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Building {
    pub pos: [i32; 2],
    pub type_: BuildingType,
    pub task: Task,
    pub inventory: HashMap<ItemType, usize>,
    /// The number of crews attending this building.
    pub crews: usize,
    // TODO: We want to avoid copies of recipes, but deserializing a recipe with static is
    // extremely hard with serde.
    pub recipe: Option<Recipe>,
}

impl Building {
    pub fn new(pos: [i32; 2], type_: BuildingType) -> Self {
        Self {
            pos,
            type_,
            task: Task::None,
            inventory: HashMap::new(),
            crews: type_.max_crews(),
            recipe: None,
        }
    }

    pub fn new_inventory(
        pos: [i32; 2],
        type_: BuildingType,
        inventory: HashMap<ItemType, usize>,
    ) -> Self {
        Self {
            pos,
            type_,
            task: Task::None,
            inventory,
            crews: type_.max_crews(),
            recipe: None,
        }
    }

    pub fn power(&self) -> isize {
        let base = self.type_.power();
        let task_power = match self.task {
            Task::Excavate(_, _) => 200,
            Task::Assemble { .. } => 300,
            _ => 0,
        };
        base - task_power
    }

    pub fn inventory_size(&self) -> usize {
        self.inventory.iter().map(|(_, v)| *v).sum()
    }

    pub fn tick(
        bldgs: &mut [Building],
        idx: usize,
        cells: &Tiles,
        transports: &mut Vec<Transport>,
        constructions: &mut [Construction],
        crews: &mut Vec<Crew>,
        gtasks: &[GlobalTask],
        rng: &mut Xor128,
    ) -> Result<(), String> {
        let (first, rest) = bldgs.split_at_mut(idx);
        let Some((this, last)) = rest.split_first_mut() else {
            return Ok(());
        };
        // Try pushing out products
        if let Some(ref recipe) = this.recipe {
            let outputs: HashSet<_> = recipe.outputs.keys().copied().collect();
            push_outputs(cells, transports, this, first, last, &|item| {
                outputs.contains(&item)
            });
        }
        if matches!(this.task, Task::None) {
            if let Some(recipe) = &this.recipe {
                pull_inputs(
                    &recipe.inputs,
                    cells,
                    transports,
                    this.pos,
                    this.type_.size(),
                    &mut this.inventory,
                    first,
                    last,
                );
                for (ty, recipe_count) in &recipe.inputs {
                    let actual_count = *this.inventory.get(&ty).unwrap_or(&0);
                    if actual_count < *recipe_count {
                        crate::console_log!(
                            "An ingredient {:?} is missing for recipe {:?}",
                            ty,
                            recipe.outputs
                        );
                        return Ok(());
                    }
                }
                for (ty, recipe_count) in &recipe.inputs {
                    if let Some(entry) = this.inventory.get_mut(&ty) {
                        if *recipe_count <= *entry {
                            *entry = entry.saturating_sub(*recipe_count);
                        }
                    }
                }
                this.task = Task::Assemble {
                    t: recipe.time,
                    max_t: recipe.time,
                    outputs: recipe.outputs.clone(),
                };
            }
        }
        match this.type_ {
            BuildingType::Excavator => {
                push_outputs(cells, transports, this, first, last, &|t| {
                    matches!(t, ItemType::RawOre)
                });
            }
            BuildingType::CrewCabin => {
                if this.crews == 0 {
                    return Ok(());
                }
                for gtask in gtasks {
                    let GlobalTask::Excavate(t, goal_pos) = gtask;
                    if *t <= 0. {
                        continue;
                    }
                    if crews.iter().any(|crew| crew.target() == Some(*goal_pos)) {
                        continue;
                    }
                    if let Some(crew) = Crew::new_task(this.pos, gtask, cells) {
                        crews.push(crew);
                        this.crews -= 1;
                        return Ok(());
                    }
                }
                for construction in constructions {
                    let pos = construction.pos;
                    if !matches!(cells[pos].state, CellState::Empty) {
                        // Don't bother trying to find a path in an unreachable area.
                        continue;
                    }
                    let crew = construction
                        .required_ingredients(transports, crews)
                        .find_map(|(ty, _)| {
                            if 0 < this.inventory.get(&ty).copied().unwrap_or(0) {
                                Crew::new_deliver(this.pos, construction.pos, ty, cells)
                            } else {
                                let path_to_source = find_multipath(
                                    [this.pos].into_iter(),
                                    |pos| {
                                        first.iter().chain(last.iter()).any(|o| {
                                            o.pos == pos
                                                && 0 < o.inventory.get(&ty).copied().unwrap_or(0)
                                        })
                                    },
                                    |_, pos| matches!(cells[pos].state, CellState::Empty),
                                );
                                path_to_source
                                    .and_then(|src| src.first().copied())
                                    .and_then(|src| {
                                        Crew::new_pickup(this.pos, src, construction.pos, ty, cells)
                                    })
                            }
                        })
                        .or_else(|| {
                            construction.extra_ingredients().find_map(|(ty, _)| {
                                let path_to_dest = find_multipath(
                                    [construction.pos].into_iter(),
                                    |pos| {
                                        first.iter().chain(last.iter()).any(|o| {
                                            o.pos == pos && o.inventory_size() < o.type_.capacity()
                                        })
                                    },
                                    |_, pos| matches!(cells[pos].state, CellState::Empty),
                                );

                                path_to_dest
                                    .and_then(|dst| dst.first().copied())
                                    .and_then(|dst| {
                                        Crew::new_pickup(this.pos, construction.pos, dst, ty, cells)
                                    })
                            })
                        })
                        .or_else(|| {
                            if crews
                                .iter()
                                .any(|crew| crew.target() == Some(construction.pos))
                            {
                                return None;
                            }
                            if construction.ingredients_satisfied() {
                                Crew::new_build(this.pos, construction.pos, cells)
                            } else {
                                None
                            }
                        });
                    crate::console_log!("crew: {:?}", crew);
                    if let Some(crew) = crew {
                        crews.push(crew);
                        this.crews -= 1;
                        return Ok(());
                    }
                }
            }
            BuildingType::Furnace => {
                push_outputs(cells, transports, this, first, last, &|t| {
                    !matches!(t, ItemType::RawOre)
                });
                if !matches!(this.task, Task::None) {
                    return Ok(());
                }
                // A tentative recipe. The output does not have to represent the actual products yet.
                let recipe = Recipe {
                    inputs: hash_map!(ItemType::RawOre => 1),
                    outputs: hash_map!(ItemType::IronIngot => 1),
                    time: RAW_ORE_SMELT_TIME,
                };
                pull_inputs(
                    &recipe.inputs,
                    cells,
                    transports,
                    this.pos,
                    this.type_.size(),
                    &mut this.inventory,
                    first,
                    last,
                );
                if let Some(source) = this.inventory.get_mut(&ItemType::RawOre) {
                    if *source < 1 {
                        return Ok(());
                    };
                    *source -= 1;
                    let dice = rng.nexti() % 7;
                    let outputs = hash_map!(match dice {
                        0..=3 => ItemType::Cilicate,
                        4..=5 => ItemType::IronIngot,
                        _ => ItemType::CopperIngot,
                    } => 1);
                    this.task = Task::Assemble {
                        t: RAW_ORE_SMELT_TIME,
                        max_t: RAW_ORE_SMELT_TIME,
                        outputs,
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl AsteroidColoniesGame {
    pub(super) fn process_buildings(&mut self) {
        let power_demand = self
            .buildings
            .iter()
            .map(|b| b.power().min(0).abs() as usize)
            .sum::<usize>();
        let power_supply = self
            .buildings
            .iter()
            .map(|b| b.power().max(0).abs() as usize)
            .sum::<usize>();
        // let power_load = (power_demand as f64 / power_supply as f64).min(1.);
        let power_ratio = (power_supply as f64 / power_demand as f64).min(1.);
        // A buffer to avoid borrow checker
        let mut moving_items = vec![];
        for i in 0..self.buildings.len() {
            let res = Building::tick(
                &mut self.buildings,
                i,
                &self.cells,
                &mut self.transports,
                &mut self.constructions,
                &mut self.crews,
                &self.global_tasks,
                &mut self.rng,
            );
            if let Err(e) = res {
                crate::console_log!("Building::tick error: {}", e);
            };
        }
        for building in &mut self.buildings {
            if let Some((item, dest)) = Self::process_task(
                &mut self.cells,
                building,
                power_ratio,
                self.calculate_back_image.as_mut(),
            ) {
                moving_items.push((item, dest));
            }
        }

        for (item, item_pos) in moving_items {
            let found = self.buildings.iter_mut().find(|b| b.pos == item_pos);
            if let Some(found) = found {
                *found.inventory.entry(item).or_default() += 1;
            }
        }
    }
}
