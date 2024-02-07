use std::{collections::HashMap, fmt::Display};

use crate::{
    task::{
        Task, BUILD_ASSEMBLER_TIME, BUILD_CREW_CABIN_TIME, BUILD_EXCAVATOR_TIME,
        BUILD_FURNACE_TIME, BUILD_POWER_PLANT_TIME, BUILD_STORAGE_TIME, SLUG_SMELT_TIME,
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
            Self::Assembler => 10,
            Self::Furnace => 10,
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

pub(crate) struct Building {
    pub pos: [i32; 2],
    pub type_: BuildingType,
    pub task: Task,
    pub inventory: HashMap<ItemType, usize>,
    /// The number of crews attending this building.
    pub crews: usize,
}

impl Building {
    pub fn new(pos: [i32; 2], type_: BuildingType) -> Self {
        Self {
            pos,
            type_,
            task: Task::None,
            inventory: HashMap::new(),
            crews: type_.max_crews(),
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

    pub fn tick(bldgs: &mut [Building], idx: usize) {
        let (first, rest) = bldgs.split_at_mut(idx);
        let Some((this, last)) = rest.split_first_mut() else {
            return;
        };
        let mut others = first.iter_mut().chain(last.iter_mut());
        match this.type_ {
            BuildingType::Furnace => {
                if !matches!(this.task, Task::None) {
                    return;
                }
                let source = others.find(|b| 0 < *b.inventory.get(&ItemType::RawOre).unwrap_or(&0));
                if let Some(source) = source {
                    let Some(entry) = source.inventory.get_mut(&ItemType::RawOre) else {
                        return;
                    };
                    *entry -= 1;
                    this.task = Task::Assemble(SLUG_SMELT_TIME, ItemType::IronIngot);
                }
            }
            _ => {}
        }
    }
}
