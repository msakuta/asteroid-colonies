use std::{collections::HashMap, fmt::Display};

use rand::Rng;

use serde::Serialize;

use crate::{
    task::{
        Task, BUILD_ASSEMBLER_TIME, BUILD_CREW_CABIN_TIME, BUILD_EXCAVATOR_TIME,
        BUILD_FURNACE_TIME, BUILD_POWER_PLANT_TIME, BUILD_STORAGE_TIME, IRON_INGOT_SMELT_TIME,
    },
    ItemType,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BuildingType {
    Power,
    Excavator,
    Storage,
    CrewCabin,
    Assembler,
    Furnace,
}

impl BuildingType {
    pub fn capacity(&self) -> usize {
        match self {
            Self::Power => 3,
            Self::Excavator => 3,
            Self::Storage => 10,
            Self::CrewCabin => 10,
            Self::Assembler => 30,
            Self::Furnace => 20,
        }
    }

    pub fn size(&self) -> [usize; 2] {
        match self {
            Self::CrewCabin | Self::Assembler | Self::Furnace => [2, 2],
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
            Self::Assembler => -20,
            Self::Furnace => -10,
        }
    }

    pub fn build_time(&self) -> usize {
        match self {
            Self::Power => BUILD_POWER_PLANT_TIME,
            Self::CrewCabin => BUILD_CREW_CABIN_TIME,
            Self::Excavator => BUILD_EXCAVATOR_TIME,
            Self::Storage => BUILD_STORAGE_TIME,
            Self::Assembler => BUILD_ASSEMBLER_TIME,
            Self::Furnace => BUILD_FURNACE_TIME,
        }
    }
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
            Self::Storage => write!(f, "Storage"),
            Self::CrewCabin => write!(f, "CrewCabin"),
            Self::Assembler => write!(f, "Assembler"),
            Self::Furnace => write!(f, "Furnace"),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct Recipe {
    pub inputs: HashMap<ItemType, usize>,
    pub outputs: HashMap<ItemType, usize>,
    pub time: usize,
}

pub(crate) struct Building {
    pub pos: [i32; 2],
    pub type_: BuildingType,
    pub task: Task,
    pub inventory: HashMap<ItemType, usize>,
    /// The number of crews attending this building.
    pub crews: usize,
    pub recipe: Option<&'static Recipe>,
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

    pub fn power(&self) -> isize {
        let base = self.type_.power();
        let task_power = match self.task {
            Task::Excavate(_, _) => 200,
            _ => 0,
        };
        base - task_power
    }

    pub fn inventory_size(&self) -> usize {
        self.inventory.iter().map(|(_, v)| *v).sum()
    }

    pub fn tick(bldgs: &mut [Building], idx: usize) -> Result<(), String> {
        let (first, rest) = bldgs.split_at_mut(idx);
        let Some((this, last)) = rest.split_first_mut() else {
            return Ok(());
        };
        // let mut others = || first.iter_mut().chain(last.iter_mut());
        if matches!(this.task, Task::None) {
            if let Some(recipe) = &this.recipe {
                for (ty, count) in recipe.inputs.iter() {
                    if first
                        .iter()
                        .chain(last.iter())
                        .chain(std::iter::once(this as &_))
                        .map(|o| o.inventory.get(ty).copied().unwrap_or(0))
                        .sum::<usize>()
                        < *count
                    {
                        return Err(format!(
                            "An ingredient {ty:?} is missing for recipe {:?}",
                            recipe.outputs
                        ));
                    }
                }
                for (ty, count) in &recipe.inputs {
                    if let Some(entry) = this.inventory.get_mut(&ty) {
                        *entry -= *count;
                    } else if let Some(entry) =
                        first.iter_mut().chain(last.iter_mut()).find_map(|o| {
                            let cand = o.inventory.get_mut(ty);
                            if let Some(cand) = &cand {
                                if **cand == 0 {
                                    return None;
                                }
                            }
                            cand
                        })
                    {
                        *entry -= *count;
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
            BuildingType::Furnace => {
                let dest = first.iter_mut().chain(last.iter_mut()).find(|b| {
                    matches!(b.type_, BuildingType::Storage)
                        && b.inventory_size() < b.type_.capacity()
                });
                // Push away outputs
                if let Some(dest) = dest {
                    let product = this
                        .inventory
                        .iter_mut()
                        .find(|(t, _)| !matches!(t, ItemType::RawOre));
                    if let Some(product) = product {
                        if 0 < *product.1 {
                            *dest.inventory.entry(*product.0).or_default() += 1;
                            *product.1 -= 1;
                        }
                    }
                }
                if !matches!(this.task, Task::None) {
                    return Ok(());
                }
                let source = first
                    .iter_mut()
                    .chain(last.iter_mut())
                    .find(|b| 0 < *b.inventory.get(&ItemType::RawOre).unwrap_or(&0));
                if let Some(source) = source {
                    let Some(entry) = source.inventory.get_mut(&ItemType::RawOre) else {
                        return Ok(());
                    };
                    *entry -= 1;
                    let mut outputs = HashMap::new();
                    const TOTAL_AMOUNT: usize = 4;
                    let iron = rand::thread_rng().gen_range(0..=TOTAL_AMOUNT);
                    if 0 < iron {
                        outputs.insert(ItemType::IronIngot, iron);
                    }
                    let copper = TOTAL_AMOUNT - iron;
                    if 0 < copper {
                        outputs.insert(ItemType::CopperIngot, copper);
                    }
                    this.task = Task::Assemble {
                        t: IRON_INGOT_SMELT_TIME,
                        max_t: IRON_INGOT_SMELT_TIME,
                        outputs,
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }
}
