use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use rand::Rng;

use serde::{Deserialize, Serialize};

use crate::{
    construction::Construction,
    hash_map,
    task::{Direction, GlobalTask, Task, RAW_ORE_SMELT_TIME},
    transport::{expected_deliveries, find_multipath},
    AsteroidColonies, Cell, CellState, Conveyor, Crew, ItemType, Pos, Transport, WIDTH,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Serialize)]
pub(crate) struct Recipe {
    pub inputs: HashMap<ItemType, usize>,
    pub outputs: HashMap<ItemType, usize>,
    pub time: f64,
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
        cells: &[Cell],
        transports: &mut Vec<Transport>,
        constructions: &mut [Construction],
        crews: &mut Vec<Crew>,
        gtasks: &[GlobalTask],
    ) -> Result<(), String> {
        let (first, rest) = bldgs.split_at_mut(idx);
        let Some((this, last)) = rest.split_first_mut() else {
            return Ok(());
        };
        // Try pushing out products
        if let Some(recipe) = this.recipe {
            push_outputs(cells, transports, this, first, last, &|item| {
                recipe.outputs.contains_key(&item)
            });
        }
        if matches!(this.task, Task::None) {
            if let Some(recipe) = &this.recipe {
                pull_inputs(
                    &recipe.inputs,
                    &cells,
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
                        break;
                    }
                }
                for construction in constructions {
                    let pos = construction.pos;
                    if !matches!(
                        cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                        CellState::Empty
                    ) {
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
                                    |_, pos| {
                                        matches!(
                                            cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                                            CellState::Empty
                                        )
                                    },
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
                                    |_, pos| {
                                        matches!(
                                            cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                                            CellState::Empty
                                        )
                                    },
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
                    &cells,
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
                    let outputs = hash_map!(match rand::thread_rng().gen_range(0..7) {
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

impl AsteroidColonies {
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
            );
            if let Err(e) = res {
                crate::console_log!("Building::tick error: {}", e);
            };
        }
        for building in &mut self.buildings {
            if let Some((item, dest)) = Self::process_task(&mut self.cells, building, power_ratio) {
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

pub(crate) trait TileSampler {
    fn at(&self, pos: [i32; 2]) -> Option<&Cell>;
}

impl TileSampler for &[Cell] {
    fn at(&self, pos: [i32; 2]) -> Option<&Cell> {
        Some(&self[pos[0] as usize + pos[1] as usize * WIDTH])
    }
}

/// Pull inputs over transportation network
pub(crate) fn pull_inputs(
    inputs: &HashMap<ItemType, usize>,
    cells: &impl TileSampler,
    transports: &mut Vec<Transport>,
    this_pos: Pos,
    this_size: [usize; 2],
    this_inventory: &mut HashMap<ItemType, usize>,
    first: &mut [Building],
    last: &mut [Building],
) {
    let intersects_goal = |[ix, iy]: [i32; 2]| {
        this_pos[0] <= ix
            && ix < this_size[0] as i32 + this_pos[0]
            && this_pos[1] <= iy
            && iy < this_size[1] as i32 + this_pos[1]
    };
    // crate::console_log!("pulling to at {:?} size {:?}", this_pos, this_size);
    let expected = expected_deliveries(transports, this_pos);
    for (ty, count) in inputs {
        let this_count =
            this_inventory.get(ty).copied().unwrap_or(0) + expected.get(ty).copied().unwrap_or(0);
        if *count <= this_count {
            continue;
        }
        let Some((src, amount)) = find_from_other_inventory_mut(*ty, first, last) else {
            continue;
        };
        if amount == 0 {
            continue;
        }
        let size = src.type_.size();
        let start_pos = rect_iter(src.pos, size);
        let start_neighbors = neighbors_set(rect_iter(src.pos, size));
        let path = find_multipath(start_pos, intersects_goal, |from_direction, pos| {
            if intersects_goal(pos) {
                return true;
            }
            let Some(cell) = cells.at(pos) else {
                return false;
            };
            let prev_cell = from_direction
                .map(|dir| {
                    let dir_vec = dir.to_vec();
                    let prev_pos = [pos[0] - dir_vec[0], pos[1] - dir_vec[1]];
                    println!("dir: {dir:?}, dir_vec: {dir_vec:?}, prev_pos: {prev_pos:?}");
                    let Some(prev_cell) = cells.at(prev_pos) else {
                        return true;
                    };
                    println!("prev_cell: {prev_cell:?}");
                    // If the previous cell didn't have a conveyor, it's not a failure, because we want to be
                    // able to depart from a building.
                    prev_cell.conveyor.to().map(|to| to == dir).unwrap_or(true)
                })
                .unwrap_or(true);
            // crate::console_log!(
            //     "pulling {:?} from {:?}: dir: {:?} cell {:?}, {:?}",
            //     ty,
            //     src.pos,
            //     from_direction.map(|d| d.reverse()),
            //     pos,
            //     cell.conveyor
            // );
            if cell.conveyor.is_some() && start_neighbors.contains(&pos) {
                // crate::console_log!("next to start");
                return true;
            }
            if !prev_cell {
                return false;
            }
            from_direction.map(|from_direction| {
                matches!(cell.conveyor, Conveyor::One(dir, _) if dir == from_direction.reverse())
            }).unwrap_or_else(|| cell.conveyor.is_some())
            // cell.conveyor.is_some() || intersects(pos)
        });
        let Some(path) = path else {
            continue;
        };
        let src_count = src.inventory.entry(*ty).or_default();
        let amount = (*src_count).min(*count - this_count);
        transports.push(Transport {
            src: src.pos,
            dest: this_pos,
            path,
            item: *ty,
            amount,
        });
        if *src_count <= amount {
            src.inventory.remove(ty);
        } else {
            *src_count -= amount;
        }
    }
}

#[test]
fn test_pull_inputs() {
    struct MockTiles;

    impl TileSampler for MockTiles {
        fn at(&self, pos: [i32; 2]) -> Option<&Cell> {
            use {Conveyor::*, Direction::*};
            static SOLID: Cell = Cell::new();
            static RD: Cell = Cell::new_with_conveyor(One(Right, Down));
            static UD: Cell = Cell::new_with_conveyor(One(Up, Down));
            static UR: Cell = Cell::new_with_conveyor(One(Up, Right));
            static LR: Cell = Cell::new_with_conveyor(One(Left, Right));
            static LU: Cell = Cell::new_with_conveyor(One(Left, Up));
            static DU: Cell = Cell::new_with_conveyor(One(Down, Up));
            static DL: Cell = Cell::new_with_conveyor(One(Down, Left));
            static RL: Cell = Cell::new_with_conveyor(One(Right, Left));
            let ret = match pos {
                [0, 0] => Some(&RD),
                [0, 1] => Some(&UD),
                [0, 2] => Some(&UR),
                [1, 2] => Some(&LR),
                [2, 2] => Some(&LU),
                [2, 1] => Some(&DU),
                [2, 0] => Some(&DL),
                [1, 0] => Some(&RL),
                _ => Some(&SOLID),
            };
            println!("at({pos:?}): {ret:?}");
            ret
        }
    }

    let mut inputs = HashMap::new();
    inputs.insert(ItemType::RawOre, 1);

    let mut storage = [Building::new_inventory(
        [1, -1],
        BuildingType::Storage,
        inputs.clone(),
    )];

    let mut transports = vec![];

    pull_inputs(
        &inputs,
        &MockTiles,
        &mut transports,
        [1, 3],
        [1, 1],
        &mut HashMap::new(),
        &mut storage,
        &mut [],
    );

    assert_eq!(
        transports,
        vec![Transport {
            src: [1, -1],
            dest: [1, 3],
            item: ItemType::RawOre,
            amount: 1,
            path: vec![[1, 3], [1, 2], [0, 2], [0, 1], [0, 0], [1, 0], [1, -1]],
        }]
    )
}

/// A trait for objects that has inventory and position.
pub(crate) trait HasInventory {
    fn pos(&self) -> Pos;
    fn size(&self) -> [usize; 2];
    fn inventory(&mut self) -> &mut HashMap<ItemType, usize>;
}

impl HasInventory for Building {
    fn pos(&self) -> Pos {
        self.pos
    }

    fn size(&self) -> [usize; 2] {
        self.type_.size()
    }

    fn inventory(&mut self) -> &mut HashMap<ItemType, usize> {
        &mut self.inventory
    }
}

pub(crate) fn push_outputs(
    cells: &[Cell],
    transports: &mut Vec<Transport>,
    this: &mut impl HasInventory,
    first: &mut [Building],
    last: &mut [Building],
    is_output: &impl Fn(ItemType) -> bool,
) {
    let pos = this.pos();
    let size = this.size();
    let start_pos = || rect_iter(pos, size);
    let start_neighbors = neighbors_set(start_pos());
    // crate::console_log!(
    //     "pusheing from {:?} size {:?}, neighbors: {:?}",
    //     pos,
    //     size,
    //     start_neighbors
    // );
    let dest = first.iter_mut().chain(last.iter_mut()).find_map(|b| {
        if !b.type_.is_storage()
            || b.type_.capacity()
                <= b.inventory_size()
                    + expected_deliveries(transports, b.pos)
                        .values()
                        .sum::<usize>()
        {
            return None;
        }
        let b_size = b.type_.size();
        let intersects = |[ix, iy]: [i32; 2]| {
            b.pos[0] <= ix
                && ix < b_size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < b_size[1] as i32 + b.pos[1]
        };
        let path = find_multipath(
            start_pos(),
            |pos| pos == b.pos,
            |from_direction, pos| {
                let cell = &cells[pos[0] as usize + pos[1] as usize * WIDTH];
                // crate::console_log!(
                //     "pushing to {:?}: from: {:?}, cell {:?}, {:?}",
                //     b.pos,
                //     from_direction.map(|d| d.reverse()),
                //     pos,
                //     cell.conveyor
                // );
                if cell.conveyor.is_some() && start_neighbors.contains(&pos) {
                    // crate::console_log!("next to start");
                    return true;
                }
                from_direction.map(|from_direction| {
                    matches!(cell.conveyor, Conveyor::One(dir, _) if dir == from_direction.reverse())
                }).unwrap_or_else(||cell.conveyor.is_some()) || intersects(pos)
            },
        )?;
        Some((b, path))
    });
    // Push away outputs
    if let Some((dest, path)) = dest {
        let product = this
            .inventory()
            .iter_mut()
            .find(|(t, count)| is_output(**t) && 0 < **count);
        if let Some((&item, amount)) = product {
            transports.push(Transport {
                src: pos,
                dest: dest.pos,
                path,
                item,
                amount: 1,
            });
            // *dest.inventory.entry(*product.0).or_default() += 1;
            if *amount <= 1 {
                this.inventory().remove(&item);
            } else {
                *amount -= 1;
            }
            // this.output_path = Some(path);
        }
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
        if count == 0 {
            return None;
        }
        Some((o, count))
    })
}

/// Return an iterator over cells covering a rectangle specified by left top corner position and a size.
fn rect_iter(pos: Pos, size: [usize; 2]) -> impl Iterator<Item = Pos> {
    (0..size[0])
        .map(move |ix| (0..size[1]).map(move |iy| [pos[0] + ix as i32, pos[1] + iy as i32]))
        .flatten()
}

fn neighbors_set(it: impl Iterator<Item = Pos>) -> HashSet<Pos> {
    let mut set = HashSet::new();
    for sp in it {
        for dir in Direction::all() {
            let dv = dir.to_vec();
            set.insert([sp[0] + dv[0], sp[1] + dv[1]]);
        }
    }
    set
}

fn _is_neighbor(a: Pos, b: Pos) -> bool {
    a[0].abs_diff(b[0]) < 1 && a[1] == b[1] || a[1].abs_diff(b[1]) < 1 && a[0] == b[0]
}
