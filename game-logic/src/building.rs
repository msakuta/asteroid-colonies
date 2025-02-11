mod crew_cabin;

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use ::serde::{Deserialize, Serialize};

use self::crew_cabin::Envs;

use crate::{
    construction::Construction,
    crew::expected_crew_pickup_any,
    entity::{EntityId, EntitySet},
    inventory::Inventory,
    items::ItemType,
    measure_time,
    push_pull::{pull_inputs, pull_ores, push_outputs},
    task::{BuildingTask, GlobalTask, RAW_ORE_SMELT_TIME},
    tile::Tiles,
    transport::TransportId,
    AsteroidColoniesGame, Crew, Direction, Pos, TileState, Transport, Xor128,
};

pub type BuildingId = EntityId<Building>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BuildingType {
    Power,
    Battery,
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
            Self::Battery => 0,
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
    pub fn power_gen(&self) -> isize {
        match self {
            Self::Power => 250,
            Self::Battery => 0,
            Self::CrewCabin => -100,
            Self::Excavator => -10,
            Self::Storage => 0,
            Self::MediumStorage => 0,
            Self::Assembler => -20,
            Self::Furnace => -10,
        }
    }

    pub fn energy_capacity(&self) -> Option<usize> {
        match self {
            Self::Battery => Some(10000),
            _ => None,
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
            Self::Battery => write!(f, "Battery"),
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
    pub task: BuildingTask,
    pub inventory: Inventory,
    /// The number of crews attending this building.
    pub crews: usize,
    // TODO: We want to avoid copies of recipes, but deserializing a recipe with static is
    // extremely hard with serde.
    pub recipe: Option<Recipe>,
    /// Some buildings have direction.
    pub direction: Option<Direction>,
    /// Some buildings can store energy, like capacitors and batteries.
    pub energy: Option<usize>,
    pub ore_accum: OreAccum,
    #[serde(skip)]
    /// A cache of expected transports
    pub expected_transports: HashSet<TransportId>,
}

impl Building {
    pub fn new(pos: [i32; 2], type_: BuildingType) -> Self {
        Self {
            pos,
            type_,
            task: BuildingTask::None,
            inventory: Inventory::new(),
            crews: type_.max_crews(),
            recipe: None,
            direction: None,
            energy: type_.energy_capacity(),
            ore_accum: OreAccum::default(),
            expected_transports: HashSet::new(),
        }
    }

    pub fn new_inventory(pos: [i32; 2], type_: BuildingType, inventory: Inventory) -> Self {
        Self {
            pos,
            type_,
            task: BuildingTask::None,
            inventory,
            crews: type_.max_crews(),
            recipe: None,
            direction: None,
            energy: type_.energy_capacity(),
            ore_accum: OreAccum::default(),
            expected_transports: HashSet::new(),
        }
    }

    /// Generating power. Prioritized to be used.
    pub fn power_gen(&self) -> isize {
        let base = self.type_.power_gen();
        let task_power = match self.task {
            BuildingTask::Excavate(_, _) => 200,
            BuildingTask::Assemble { .. } => 300,
            _ => 0,
        };
        base - task_power
    }

    /// Power demanded to charge
    pub fn power_charge(&self) -> isize {
        if matches!(self.type_, BuildingType::Battery) {
            let max = self.type_.energy_capacity();
            return self
                .energy
                .zip(max)
                .map(|(e, max)| (max - e).min(500) as isize)
                .unwrap_or(0);
        }
        0
    }

    /// Power provided by a battery or a capacitor. Used only if power_gen is not enough
    pub fn power_discharge(&self) -> isize {
        if matches!(self.type_, BuildingType::Battery) {
            return self.energy.map(|e| e.min(500) as isize).unwrap_or(0);
        }
        0
    }

    pub fn inventory_size(&self) -> usize {
        self.inventory.countable_size()
    }

    pub fn intersects(&self, pos: Pos) -> bool {
        let size = self.type_.size();
        self.pos[0] <= pos[0]
            && pos[0] < self.pos[0] + size[0] as i32
            && self.pos[1] <= pos[1]
            && pos[1] < self.pos[1] + size[1] as i32
    }

    pub fn intersects_rect(&self, pos: Pos, other_size: [usize; 2]) -> bool {
        let size = self.type_.size();
        self.pos[0] < pos[0] + other_size[0] as i32
            && pos[0] < self.pos[0] + size[0] as i32
            && self.pos[1] < pos[1] + other_size[1] as i32
            && pos[1] < self.pos[1] + size[1] as i32
    }

    pub(super) fn set_recipe(&mut self, recipe: Option<&Recipe>) -> Result<(), String> {
        if !matches!(self.type_, BuildingType::Assembler) {
            return Err(String::from("The building is not an assembler"));
        }
        self.recipe = recipe.cloned();
        Ok(())
    }

    pub fn tick(
        &mut self,
        id: BuildingId,
        bldgs: &EntitySet<Building>,
        tiles: &Tiles,
        transports: &mut EntitySet<Transport>,
        constructions: &mut EntitySet<Construction>,
        crews: &mut EntitySet<Crew>,
        gtasks: &EntitySet<GlobalTask>,
        _rng: &mut Xor128,
    ) -> Result<(), String> {
        // Try pushing out products
        if let Some(ref recipe) = self.recipe {
            let outputs: HashSet<_> = recipe.outputs.keys().copied().collect();
            push_outputs(
                tiles,
                transports,
                self,
                bldgs,
                &|item| outputs.contains(&item),
                false,
            );
        }
        let this = self;
        if matches!(this.task, BuildingTask::None) {
            if let Some(recipe) = &this.recipe {
                pull_inputs(
                    &recipe.inputs,
                    tiles,
                    transports,
                    &mut this.expected_transports,
                    this.pos,
                    this.type_.size(),
                    &mut this.inventory,
                    bldgs,
                );
                for (ty, recipe_count) in &recipe.inputs {
                    let actual_count = this.inventory.get(&ty);
                    if actual_count < *recipe_count {
                        // crate::console_log!(
                        //     "An ingredient {:?} is missing for recipe {:?}",
                        //     ty,
                        //     recipe.outputs
                        // );
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
                this.task = BuildingTask::Assemble {
                    t: recipe.time,
                    max_t: recipe.time,
                    outputs: recipe.outputs.clone(),
                };
            }
        }
        match this.type_ {
            BuildingType::Excavator => {
                push_outputs(
                    tiles,
                    transports,
                    &mut *this,
                    bldgs,
                    &|t| matches!(t, ItemType::RawOre),
                    true,
                );
            }
            BuildingType::CrewCabin => {
                if this.crews == 0 {
                    return Ok(());
                }
                for (gt_id, gtask) in gtasks.items() {
                    let goal_pos = match &*gtask {
                        GlobalTask::Excavate(t, goal_pos) => {
                            if *t <= 0. {
                                continue;
                            }
                            if bldgs.iter().any(|b| matches!(b.task, BuildingTask::Excavate(_, other_id) if other_id == gt_id)) {
                                continue;
                            }
                            goal_pos
                        }
                        GlobalTask::Cleanup(pos) => {
                            let pickups = expected_crew_pickup_any(crews, *pos);
                            if pickups != 0 {
                                continue;
                            }
                            pos
                        }
                    };
                    if crews
                        .iter()
                        .any(|crew| crew.gt_id() == Some(gt_id) || crew.target() == Some(*goal_pos))
                    {
                        continue;
                    }
                    if let Some(crew) = Crew::new_task(id, this, gt_id, &*gtask, tiles) {
                        crews.insert(crew);
                        this.crews -= 1;
                        return Ok(());
                    }
                }
                fn print_time<R>(name: &str, f: impl FnOnce() -> R) -> R {
                    let (r, t) = measure_time(f);
                    if 0.001 < t {
                        println!("{name} time: {}", t);
                    }
                    r
                }
                for construction in constructions.iter() {
                    let pos = construction.pos;
                    if !matches!(tiles[pos].state, TileState::Empty) {
                        // Don't bother trying to find a path in an unreachable area.
                        continue;
                    }
                    let envs = Envs {
                        buildings: bldgs,
                        transports,
                        crews,
                        tiles,
                    };
                    let crew = print_time("try_find_deliver", || {
                        this.try_find_deliver(id, &*construction, &envs)
                    })
                    .or_else(|| {
                        print_time("try_find_pickup_and_deliver", || {
                            this.try_find_pickup_and_deliver(id, &*construction, &envs)
                        })
                    })
                    .or_else(|| {
                        print_time("try_send_to_build", || {
                            this.try_send_to_build(id, &*construction, &envs)
                        })
                    });
                    if let Some(crew) = crew {
                        crews.insert(crew);
                        this.crews -= 1;
                        return Ok(());
                    }
                }
            }
            BuildingType::Furnace => {
                push_outputs(
                    tiles,
                    transports,
                    &mut *this,
                    bldgs,
                    &|t| !matches!(t, ItemType::RawOre),
                    false,
                );
                if !matches!(this.task, BuildingTask::None) {
                    return Ok(());
                }
                pull_ores(
                    tiles,
                    transports,
                    &mut this.expected_transports,
                    this.pos,
                    this.type_.size(),
                    &mut this.inventory,
                    bldgs,
                );
                let source = this.inventory.ores_mut();
                if let Some(normalized) = source.normalize() {
                    let outputs = normalized.min(source);
                    for (src, out) in source.iter_mut().zip(outputs.iter()) {
                        *src -= *out;
                    }
                    this.task = BuildingTask::Smelt {
                        t: RAW_ORE_SMELT_TIME,
                        max_t: RAW_ORE_SMELT_TIME,
                        output_ores: outputs,
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
        let (chargeable, dischargeable, power_gen, power_demand) = self
            .buildings
            .iter()
            .map(|b| (b.power_charge(), b.power_discharge(), b.power_gen()))
            .fold((0, 0, 0, 0), |acc, (charge, discharge, gen)| {
                (
                    acc.0 + charge,
                    acc.1 + discharge,
                    acc.2 + gen.max(0).abs(),
                    acc.3 + gen.min(0).abs(),
                )
            });
        // let power_load = (power_demand as f64 / power_gen as f64).min(1.);
        let power_ratio = ((dischargeable as f64 + power_gen as f64) / power_demand as f64).min(1.);
        // A buffer to avoid borrow checker
        let mut moving_items = vec![];
        for (id, mut b) in self.buildings.items_borrow_mut() {
            let res = b.tick(
                id,
                &self.buildings,
                &self.tiles,
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
        for mut building in self.buildings.iter_borrow_mut() {
            if let Some((item, dest)) = Self::process_task(
                &mut self.tiles,
                &mut *building,
                &self.buildings,
                &mut self.global_tasks,
                power_ratio,
                &mut self.rng,
                self.calculate_back_image.as_mut(),
            ) {
                moving_items.push((item, dest));
            }
        }

        let charging_total = power_gen - power_demand;

        if charging_total < 0 {
            if 0 < dischargeable {
                let drain_total = -charging_total;
                // Drain energy from capacitors proportional to the capacity
                for building in self.buildings.iter_mut() {
                    let cap = building.power_discharge();
                    let Some(ref mut energy) = building.energy else {
                        continue;
                    };
                    let drain = drain_total * cap / dischargeable;
                    *energy = (*energy as isize - drain).max(0) as usize;
                }
            }
        } else if 0 < chargeable {
            for building in self.buildings.iter_mut() {
                let max_charge = building.power_charge();
                let Some(max_energy) = building.type_.energy_capacity() else {
                    continue;
                };
                let Some(ref mut energy) = building.energy else {
                    continue;
                };
                let charge = charging_total * max_charge / chargeable;
                *energy = (*energy as isize + charge).clamp(0, max_energy as isize) as usize;
            }
        }

        self.power_ratio = power_ratio;
        self.used_power = power_ratio * power_demand as f64;
        // println!("charge: {chargeable}, discharge: {dischargeable}, power_gen: {power_gen}, power_demand: {power_demand}, ratio = {power_ratio}, used: {}", self.used_power);

        for (item, item_pos) in moving_items {
            let found = self.buildings.iter_mut().find(|b| b.pos == item_pos);
            if let Some(found) = found {
                *found.inventory.entry(item).or_default() += 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct OreAccum {
    pub cilicate: f64,
    pub iron: f64,
    pub copper: f64,
    pub lithium: f64,
}

impl OreAccum {
    pub const fn new() -> Self {
        Self {
            cilicate: 0.,
            iron: 0.,
            copper: 0.,
            lithium: 0.,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cilicate == 0. && self.iron == 0. && self.copper == 0. && self.lithium == 0.
    }

    pub fn total(&self) -> f64 {
        self.cilicate + self.iron + self.copper + self.lithium
    }

    /// Normalize the ore fraction so that it sums up to 1. In case of zero vector, returns zero.
    pub fn normalize(&self) -> Option<Self> {
        let total = self.total();
        if total == 0. {
            return None;
        }
        Some(Self {
            cilicate: self.cilicate / total,
            iron: self.iron / total,
            copper: self.copper / total,
            lithium: self.lithium / total,
        })
    }

    pub fn min(&self, other: &Self) -> Self {
        self.each(other, |lhs, rhs| lhs.min(rhs))
    }

    pub fn each(&self, other: &Self, f: impl Fn(f64, f64) -> f64) -> Self {
        Self {
            cilicate: f(self.cilicate, other.cilicate),
            iron: f(self.iron, other.iron),
            copper: f(self.copper, other.copper),
            lithium: f(self.lithium, other.lithium),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &f64> {
        [&self.cilicate, &self.iron, &self.copper, &self.lithium].into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut f64> {
        [
            &mut self.cilicate,
            &mut self.iron,
            &mut self.copper,
            &mut self.lithium,
        ]
        .into_iter()
    }
}
