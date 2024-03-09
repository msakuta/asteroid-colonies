use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    building::Building,
    console_log,
    construction::Construction,
    entity::{EntityEntry, EntityIterMutExt, EntitySet},
    hash_map,
    items::ItemType,
    task::{GlobalTask, EXCAVATE_ORE_AMOUNT, LABOR_EXCAVATE_TIME},
    transport::{find_path, Transport},
    AsteroidColoniesGame, Pos, TileState, Tiles,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum CrewTask {
    None,
    Return,
    Excavate(Pos),
    Build(Pos),
    /// A task to pickup an item and move to the destination.
    /// Optionally has an item filter.
    Pickup {
        src: Pos,
        dest: Pos,
        item: Option<ItemType>,
    },
    Deliver {
        dst: Pos,
        item: ItemType,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crew {
    pub pos: Pos,
    pub path: Option<Vec<Pos>>,
    pub from: Pos,
    task: CrewTask,
    inventory: HashMap<ItemType, usize>,
    to_delete: bool,
}

impl Crew {
    pub fn new_task(pos: Pos, gtask: &GlobalTask, tiles: &Tiles) -> Option<Self> {
        let (target, task) = match gtask {
            GlobalTask::Excavate(_, pos) => (*pos, CrewTask::Excavate(*pos)),
            GlobalTask::Cleanup(spos) => (
                *spos,
                CrewTask::Pickup {
                    src: *spos,
                    dest: pos,
                    item: None,
                },
            ),
        };
        let path = find_path(pos, target, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == target
        })?;
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task,
            inventory: HashMap::new(),
            to_delete: false,
        })
    }

    pub fn new_build(pos: Pos, dest: Pos, tiles: &Tiles) -> Option<Self> {
        let path = find_path(pos, dest, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == dest
        })?;
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task: CrewTask::Build(dest),
            inventory: HashMap::new(),
            to_delete: false,
        })
    }

    pub fn new_pickup(
        pos: Pos,
        src: Pos,
        dest: Pos,
        item: ItemType,
        tiles: &Tiles,
    ) -> Option<Self> {
        let path = find_path(pos, src, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == src
        })?;
        // Just to make sure if you can reach the destination from pickup
        if find_path(src, dest, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == dest
        })
        .is_none()
        {
            return None;
        }
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task: CrewTask::Pickup {
                src,
                dest,
                item: Some(item),
            },
            inventory: HashMap::new(),
            to_delete: false,
        })
    }

    pub fn new_deliver(pos: Pos, dest: Pos, item: ItemType, tiles: &Tiles) -> Option<Self> {
        let path = find_path(pos, dest, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == dest
        })?;
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task: CrewTask::Deliver { dst: dest, item },
            inventory: hash_map!(item => 1),
            to_delete: false,
        })
    }

    pub fn target(&self) -> Option<Pos> {
        match self.task {
            CrewTask::Excavate(pos) => Some(pos),
            CrewTask::Build(pos) => Some(pos),
            _ => None,
        }
    }

    fn process_excavate_task(&mut self, global_tasks: &mut [GlobalTask], ct_pos: Pos) {
        const ORE_PERIOD: f64 = LABOR_EXCAVATE_TIME as f64 / EXCAVATE_ORE_AMOUNT as f64;
        for gtask in global_tasks.iter_mut() {
            let GlobalTask::Excavate(t, gt_pos) = gtask else {
                continue;
            };
            if ct_pos == *gt_pos && 0. < *t {
                *t -= 1.;
                // crate::console_log!(
                //     "crew excavate: t: {}, t % T: {} (t - 1) % T: {}",
                //     t,
                //     t.rem_euclid(ORE_PERIOD),
                //     (*t - 1.).rem_euclid(ORE_PERIOD)
                // );
                if t.rem_euclid(ORE_PERIOD) < (*t - 1.).rem_euclid(ORE_PERIOD) {
                    let entry = self.inventory.entry(ItemType::RawOre).or_default();
                    *entry += 1;
                    if 1 <= *entry {
                        self.task = CrewTask::None;
                    }
                    // crate::console_log!("crew {:?}", crew.inventory);
                }
                return;
            }
        }
        self.task = CrewTask::None;
    }

    fn process_build_task(&mut self, constructions: &mut [Construction], ct_pos: Pos) {
        for con in constructions.iter_mut() {
            let canceling = con.canceling();
            let t = &mut con.progress;
            if canceling {
                if ct_pos == con.pos && 0. < *t {
                    *t = (*t - 1.).max(0.);
                    if *t <= 0. {
                        self.task = CrewTask::None;
                    }
                    return;
                }
            } else if ct_pos == con.pos && *t < con.recipe.time {
                *t += 1.;
                if con.recipe.time <= *t {
                    self.task = CrewTask::None;
                }
                return;
            }
        }
        self.task = CrewTask::None;
    }

    fn process_pickup_task(
        &mut self,
        item: Option<ItemType>,
        src: Pos,
        dest: Pos,
        tiles: &Tiles,
        buildings: &mut [EntityEntry<Building>],
        constructions: &mut [Construction],
        transports: &mut EntitySet<Transport>,
    ) {
        let mut process_inventory = |inventory: &mut HashMap<ItemType, usize>| {
            let Some(item) = item.or_else(|| inventory.keys().copied().next()) else {
                return None;
            };
            let entry = inventory.get_mut(&item).filter(|entry| 0 < **entry)?;
            *entry -= 1;
            if *entry == 0 {
                inventory.remove(&item);
            }
            *self.inventory.entry(item).or_default() += 1;
            let path = find_path(self.pos, dest, |pos| {
                matches!(tiles[pos].state, TileState::Empty) || pos == dest
            })?;
            self.path = Some(path);
            self.task = CrewTask::Deliver { dst: dest, item };
            Some(())
        };

        let intersects = |b: &Building| {
            let size = b.type_.size();
            b.pos[0] <= src[0]
                && src[0] < size[0] as i32 + b.pos[0]
                && b.pos[1] <= src[1]
                && src[1] < size[1] as i32 + b.pos[1]
        };

        let res =
            (|| process_inventory(&mut buildings.items_mut().find(|o| intersects(o))?.inventory))()
                .or_else(|| {
                    process_inventory(
                        &mut constructions
                            .iter_mut()
                            .find(|o| o.pos == src && (!o.canceling() || o.progress <= 0.))?
                            .ingredients,
                    )
                })
                .or_else(|| {
                    let (idx, transport) = transports
                        .iter()
                        .enumerate()
                        .find(|(_, t)| t.path.last() == Some(&src))?;
                    println!("Found transports: {idx}, {transport:?}");
                    let item = transport.item;
                    *self.inventory.entry(item).or_default() += 1;
                    let path = find_path(self.pos, dest, |pos| {
                        matches!(tiles[pos].state, TileState::Empty) || pos == dest
                    })?;
                    self.path = Some(path);
                    self.task = CrewTask::Deliver { dst: dest, item };
                    transports.remove(idx);
                    Some(())
                });
        if res.is_none() {
            self.task = CrewTask::None;
        };
    }

    fn process_deliver_task(
        &mut self,
        item: ItemType,
        dest: Pos,
        constructions: &mut [Construction],
    ) {
        let Some(construction) = constructions.iter_mut().find(|o| o.pos == dest) else {
            self.task = CrewTask::None;
            return;
        };
        let Some(amount) = self.inventory.remove(&item) else {
            self.task = CrewTask::None;
            return;
        };
        let entry = construction.ingredients.entry(item).or_default();
        *entry += amount;
        self.task = CrewTask::None;
    }
}

impl AsteroidColoniesGame {
    pub(super) fn process_crews(&mut self) {
        let try_return = |crew: &mut Crew, buildings: &mut [EntityEntry<Building>]| {
            if let Some(building) = buildings.items_mut().find(|b| b.pos == crew.from) {
                building.crews += 1;
                for (item, amount) in &crew.inventory {
                    *building.inventory.entry(*item).or_default() += *amount;
                }
                crew.to_delete = true;
            }
        };

        for crew in &mut self.crews {
            // console_log!("crew has path: {:?}", crew.path.as_ref().map(|p| p.len()));
            if let Some(path) = &mut crew.path {
                if path.len() <= 1 {
                    crew.path = None;
                    if matches!(crew.task, CrewTask::Return) {
                        try_return(crew, &mut self.buildings);
                    }
                } else if let Some(pos) = path.pop() {
                    crew.pos = pos;
                }
                continue;
            }
            match crew.task {
                CrewTask::Excavate(ct_pos) => {
                    crew.process_excavate_task(&mut self.global_tasks, ct_pos);
                }
                CrewTask::Build(ct_pos) => {
                    crew.process_build_task(&mut self.constructions, ct_pos);
                }
                CrewTask::Pickup { src, dest, item } => {
                    crew.process_pickup_task(
                        item,
                        src,
                        dest,
                        &self.tiles,
                        &mut self.buildings,
                        &mut self.constructions,
                        &mut self.transports,
                    );
                }
                CrewTask::Deliver { dst, item } => {
                    crew.process_deliver_task(item, dst, &mut self.constructions);
                }
                _ => {
                    console_log!("Returning home at {:?}", crew.from);
                    if crew.from == crew.pos {
                        try_return(crew, &mut self.buildings);
                    } else if let Some(path) = find_path(crew.pos, crew.from, |pos| {
                        matches!(self.tiles[pos].state, TileState::Empty) || pos == crew.from
                    }) {
                        crew.task = CrewTask::Return;
                        crew.path = Some(path);
                    }
                }
            }
        }

        self.crews.retain(|c| !c.to_delete);
    }
}

pub(crate) fn _expected_crew_pickups(crews: &[Crew], src: Pos) -> HashMap<ItemType, usize> {
    crews
        .iter()
        .filter_map(|t| match t.task {
            CrewTask::Pickup {
                src: pkup_src,
                item,
                ..
            } => {
                if src == pkup_src {
                    item
                } else {
                    None
                }
            }
            _ => None,
        })
        .fold(HashMap::new(), |mut acc, cur| {
            *acc.entry(cur).or_default() += 1;
            acc
        })
}

/// Count Pickup tasks without specific item type.
pub(crate) fn expected_crew_pickup_any(crews: &[Crew], src: Pos) -> usize {
    crews
        .iter()
        .filter(|t| match t.task {
            CrewTask::Pickup {
                src: pkup_src,
                item: None,
                ..
            } => src == pkup_src,
            _ => false,
        })
        .count()
}

pub(crate) fn expected_crew_deliveries(crews: &[Crew], dest: Pos) -> HashMap<ItemType, usize> {
    crews
        .iter()
        .filter_map(|t| match t.task {
            CrewTask::Deliver { dst, item } => {
                if dest == dst {
                    Some(item)
                } else {
                    None
                }
            }
            CrewTask::Pickup {
                dest: pkup_dest,
                item,
                ..
            } => {
                if dest == pkup_dest {
                    item
                } else {
                    None
                }
            }
            _ => None,
        })
        .fold(HashMap::new(), |mut acc, cur| {
            *acc.entry(cur).or_default() += 1;
            acc
        })
}
