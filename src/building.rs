use std::{collections::HashMap, fmt::Display};

use rand::Rng;

use serde::Serialize;

use crate::{
    task::{
        Task, BUILD_ASSEMBLER_TIME, BUILD_CREW_CABIN_TIME, BUILD_EXCAVATOR_TIME,
        BUILD_FURNACE_TIME, BUILD_MEDIUM_STORAGE_TIME, BUILD_POWER_PLANT_TIME, BUILD_STORAGE_TIME,
        IRON_INGOT_SMELT_TIME,
    },
    transport::{expected_deliveries, find_path},
    Cell, ItemType, Transport,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub(crate) enum BuildingType {
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
            Self::Power => 3,
            Self::Excavator => 3,
            Self::Storage => 10,
            Self::MediumStorage => 50,
            Self::CrewCabin => 10,
            Self::Assembler => 30,
            Self::Furnace => 20,
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

    pub fn build_time(&self) -> usize {
        match self {
            Self::Power => BUILD_POWER_PLANT_TIME,
            Self::CrewCabin => BUILD_CREW_CABIN_TIME,
            Self::Excavator => BUILD_EXCAVATOR_TIME,
            Self::Storage => BUILD_STORAGE_TIME,
            Self::MediumStorage => BUILD_MEDIUM_STORAGE_TIME,
            Self::Assembler => BUILD_ASSEMBLER_TIME,
            Self::Furnace => BUILD_FURNACE_TIME,
        }
    }

    pub fn is_storage(&self) -> bool {
        matches!(self, Self::Storage | Self::MediumStorage)
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

#[derive(Clone, Serialize)]
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
    /// A path to output resources for visualization
    pub output_path: Option<Vec<[i32; 2]>>,
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
            output_path: None,
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
            output_path: None,
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

    pub fn tick(
        bldgs: &mut [Building],
        idx: usize,
        cells: &[Cell],
        transports: &[Transport],
    ) -> Result<Vec<Transport>, String> {
        let (first, rest) = bldgs.split_at_mut(idx);
        let Some((this, last)) = rest.split_first_mut() else {
            return Ok(vec![]);
        };
        let mut ret = vec![];
        if matches!(this.task, Task::None) {
            if let Some(recipe) = &this.recipe {
                let expected = expected_deliveries(transports, this.pos);
                for (ty, count) in &recipe.inputs {
                    let this_count = this.inventory.get(ty).copied().unwrap_or(0)
                        + expected.get(ty).copied().unwrap_or(0);
                    if this_count < *count {
                        let src = find_from_other_inventory_mut(*ty, first, last);
                        if let Some((src, path)) = src.and_then(|src| {
                            if src.1 == 0 {
                                return None;
                            }
                            let path = find_path(cells, src.0.pos, this.pos)?;
                            Some((src.0, path))
                        }) {
                            ret.push(Transport {
                                src: src.pos,
                                dest: this.pos,
                                path,
                                item: *ty,
                                amount: *count - this_count,
                            });
                            let src_count = src.inventory.entry(*ty).or_default();
                            *src_count = src_count.saturating_sub(*count - this_count);
                        }
                    }
                }
                for (ty, recipe_count) in &recipe.inputs {
                    let actual_count = *this.inventory.get(&ty).unwrap_or(&0);
                    if actual_count < *recipe_count {
                        crate::console_log!(
                            "An ingredient {:?} is missing for recipe {:?}",
                            ty,
                            recipe.outputs
                        );
                        return Ok(ret);
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
            BuildingType::Furnace => {
                let dest = first.iter_mut().chain(last.iter_mut()).find_map(|b| {
                    if !b.type_.is_storage()
                        || b.type_.capacity()
                            <= b.inventory_size()
                                + expected_deliveries(&transports, b.pos)
                                    .values()
                                    .sum::<usize>()
                    {
                        return None;
                    }
                    let path = find_path(cells, this.pos, b.pos)?;
                    Some((b, path))
                });
                // Push away outputs
                if let Some((dest, path)) = dest {
                    let product = this
                        .inventory
                        .iter_mut()
                        .find(|(t, count)| !matches!(t, ItemType::RawOre) && 0 < **count);
                    if let Some(product) = product {
                        ret.push(Transport {
                            src: this.pos,
                            dest: dest.pos,
                            path,
                            item: *product.0,
                            amount: 1,
                        });
                        // *dest.inventory.entry(*product.0).or_default() += 1;
                        *product.1 -= 1;
                        // this.output_path = Some(path);
                    }
                }
                if !matches!(this.task, Task::None) {
                    return Ok(ret);
                }
                let source = first
                    .iter_mut()
                    .chain(last.iter_mut())
                    .find(|b| 0 < *b.inventory.get(&ItemType::RawOre).unwrap_or(&0));
                if let Some(source) = source {
                    let Some(entry) = source.inventory.get_mut(&ItemType::RawOre) else {
                        return Ok(ret);
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
        Ok(ret)
    }
}

fn _find_from_all_inventories(
    item: ItemType,
    this: &Building,
    first: &[Building],
    last: &[Building],
) -> usize {
    first
        .iter()
        .chain(last.iter())
        .chain(std::iter::once(this as &_))
        .map(|o| o.inventory.get(&item).copied().unwrap_or(0))
        .sum::<usize>()
}

fn _find_from_other_inventory<'a>(
    item: ItemType,
    first: &'a [Building],
    last: &'a [Building],
) -> Option<(&'a Building, usize)> {
    first
        .iter()
        .chain(last.iter())
        .find_map(|o| Some((o, *o.inventory.get(&item)?)))
}

fn find_from_other_inventory_mut<'a>(
    item: ItemType,
    first: &'a mut [Building],
    last: &'a mut [Building],
) -> Option<(&'a mut Building, usize)> {
    first.iter_mut().chain(last.iter_mut()).find_map(|o| {
        let count = *o.inventory.get(&item)?;
        Some((o, count))
    })
}
