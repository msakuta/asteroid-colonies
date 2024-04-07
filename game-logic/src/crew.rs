use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    building::{Building, BuildingId},
    console_log,
    construction::Construction,
    entity::EntitySet,
    items::{Inventory, ItemType},
    task::{GlobalTask, GlobalTaskId, EXCAVATE_ORE_AMOUNT, LABOR_EXCAVATE_TIME},
    transport::{find_path, Transport},
    AsteroidColoniesGame, Pos, TileState, Tiles,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
enum CrewTask {
    None,
    Idle(usize),
    Return,
    Excavate(GlobalTaskId),
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
    pub from: BuildingId,
    task: CrewTask,
    inventory: Inventory,
}

impl Crew {
    pub fn new_task(
        from_id: BuildingId,
        from_building: &mut Building,
        gt_id: GlobalTaskId,
        gtask: &GlobalTask,
        tiles: &Tiles,
    ) -> Option<Self> {
        let (target, task) = match gtask {
            GlobalTask::Excavate(_, pos) => (*pos, CrewTask::Excavate(gt_id)),
            GlobalTask::Cleanup(spos) => (
                *spos,
                CrewTask::Pickup {
                    src: *spos,
                    dest: from_building.pos,
                    item: None,
                },
            ),
        };
        let path = find_path(from_building.pos, target, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == target
        })?;
        Some(Self {
            pos: from_building.pos,
            path: Some(path),
            from: from_id,
            task,
            inventory: Inventory::new(),
        })
    }

    pub fn new_build(from_id: BuildingId, from_pos: Pos, dest: Pos, tiles: &Tiles) -> Option<Self> {
        let path = find_path(from_pos, dest, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == dest
        })?;
        Some(Self {
            pos: from_pos,
            path: Some(path),
            from: from_id,
            task: CrewTask::Build(dest),
            inventory: Inventory::new(),
        })
    }

    pub fn new_pickup(
        from_id: BuildingId,
        from_pos: Pos,
        src: Pos,
        dest: Pos,
        item: ItemType,
        tiles: &Tiles,
    ) -> Option<Self> {
        let path = find_path(from_pos, src, |pos| {
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
            pos: from_pos,
            path: Some(path),
            from: from_id,
            task: CrewTask::Pickup {
                src,
                dest,
                item: Some(item),
            },
            inventory: Inventory::new(),
        })
    }

    pub fn new_deliver(
        from_id: BuildingId,
        from_pos: Pos,
        dest: Pos,
        item: ItemType,
        tiles: &Tiles,
    ) -> Option<Self> {
        let path = find_path(from_pos, dest, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || pos == dest
        })?;
        Some(Self {
            pos: from_pos,
            path: Some(path),
            from: from_id,
            task: CrewTask::Deliver { dst: dest, item },
            inventory: Inventory::from([(item, 1)]),
        })
    }

    /// Returns the id of the global task
    pub fn gt_id(&self) -> Option<GlobalTaskId> {
        match self.task {
            CrewTask::Excavate(id) => Some(id),
            _ => None,
        }
    }

    pub fn target(&self) -> Option<Pos> {
        match self.task {
            CrewTask::Build(pos) => Some(pos),
            _ => None,
        }
    }

    fn process_excavate_task(
        &mut self,
        global_tasks: &mut EntitySet<GlobalTask>,
        gt_id: GlobalTaskId,
    ) {
        if let Some(GlobalTask::Excavate(t, _)) = global_tasks.get_mut(gt_id) {
            if proceed_excavate(t, 1., &mut self.inventory) && self.inventory.is_empty() {
                return;
            }
        }
        self.task = CrewTask::None;
    }

    fn process_build_task(&mut self, constructions: &mut EntitySet<Construction>, ct_pos: Pos) {
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
        buildings: &mut EntitySet<Building>,
        constructions: &mut EntitySet<Construction>,
        transports: &mut EntitySet<Transport>,
    ) {
        let mut process_inventory = |inventory: &mut Inventory| {
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
            (|| process_inventory(&mut buildings.iter_mut().find(|o| intersects(o))?.inventory))()
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
                        .items_mut()
                        .find(|(_, t)| t.path.last() == Some(&src))?;
                    println!("Found transports: {idx}, {transport:?}");
                    let item = transport.item;
                    let move_amount = transport.amount.min(1);
                    if 0 < move_amount {
                        *self.inventory.entry(item).or_default() += move_amount;
                        transport.amount -= move_amount;
                    }
                    if transport.amount == 0 {
                        transports.remove(idx);
                    }
                    let path = find_path(self.pos, dest, |pos| {
                        matches!(tiles[pos].state, TileState::Empty) || pos == dest
                    })?;
                    self.path = Some(path);
                    self.task = CrewTask::Deliver { dst: dest, item };
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
        constructions: &mut EntitySet<Construction>,
        buildings: &EntitySet<Building>,
    ) {
        let Some(crew_amount) = self.inventory.get_mut(&item) else {
            self.task = CrewTask::None;
            return;
        };
        if *crew_amount <= 0 {
            self.task = CrewTask::None;
            return;
        }
        if let Some(construction) = constructions.iter_mut().find(|o| o.pos == dest) {
            let entry = construction.ingredients.entry(item).or_default();
            *entry += *crew_amount;
            *crew_amount = 0;
        }
        if let Some(mut building) = buildings.iter_borrow_mut().find(|b| b.intersects(dest)) {
            let entry = building.inventory.entry(item).or_default();
            *entry += *crew_amount;
            *crew_amount = 0;
        }
        self.task = CrewTask::None;
    }

    fn process_idle(
        &mut self,
        crews: &EntitySet<Crew>,
        tiles: &Tiles,
        constructions: &mut EntitySet<Construction>,
        buildings: &mut EntitySet<Building>,
    ) -> bool {
        let construction = constructions.iter().find(|construction| {
            if crews
                .iter()
                .any(|crew| crew.target() == Some(construction.pos))
            {
                return false;
            }
            construction.ingredients_satisfied()
        });

        if let Some((construction, path)) = construction.and_then(|construction| {
            let dest = construction.pos;
            let path = find_path(self.pos, dest, |pos| {
                matches!(tiles[pos].state, TileState::Empty) || pos == dest
            })?;
            Some((construction, path))
        }) {
            self.task = CrewTask::Build(construction.pos);
            self.path = Some(path);
            return true;
        }
        let Some(from_building) = buildings.get(self.from) else {
            return true;
        };
        if from_building.intersects(self.pos) {
            drop(from_building);
            return self.try_return(buildings);
        }

        if let Some(path) = find_path(self.pos, from_building.pos, |pos| {
            matches!(tiles[pos].state, TileState::Empty) || from_building.intersects(pos)
        }) {
            self.task = CrewTask::Return;
            self.path = Some(path);
            return true;
        }

        // Nothing useful to do. Check some time later.
        self.task = CrewTask::Idle(10);

        true
    }

    fn try_return(&mut self, buildings: &mut EntitySet<Building>) -> bool {
        if let Some(building) = buildings.get_mut(self.from) {
            building.crews += 1;
            for (item, amount) in &self.inventory {
                *building.inventory.entry(*item).or_default() += *amount;
            }
            false
        } else {
            true
        }
    }
}

impl AsteroidColoniesGame {
    pub(super) fn process_crews(&mut self) {
        self.crews.retain_borrow_mut(|crew, id| {
            // console_log!("crew has path: {:?}", crew.path.as_ref().map(|p| p.len()));
            if let Some(path) = &mut crew.path {
                if path.len() <= 1 {
                    crew.path = None;
                    if matches!(crew.task, CrewTask::Return) {
                        return crew.try_return(&mut self.buildings);
                    }
                } else if let Some(pos) = path.pop() {
                    crew.pos = pos;
                }
                return true;
            }
            match crew.task {
                CrewTask::Excavate(gt_id) => {
                    crew.process_excavate_task(&mut self.global_tasks, gt_id);
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
                    crew.process_deliver_task(item, dst, &mut self.constructions, &self.buildings);
                    if matches!(crew.task, CrewTask::None) {
                        return crew.process_idle(
                            &self.crews,
                            &self.tiles,
                            &mut self.constructions,
                            &mut self.buildings,
                        );
                    }
                }
                CrewTask::None => {
                    return crew.process_idle(
                        &self.crews,
                        &self.tiles,
                        &mut self.constructions,
                        &mut self.buildings,
                    );
                }
                CrewTask::Idle(ref mut t) => {
                    console_log!("Crew {id} CrewTask::Idle {}", t);
                    if *t == 0 {
                        crew.task = CrewTask::None;
                        console_log!("Crew {id} Reset to None");
                    } else {
                        *t -= 1;
                    }
                }
                CrewTask::Return => {}
            }
            true
        });
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
pub(crate) fn expected_crew_pickup_any(crews: &EntitySet<Crew>, src: Pos) -> usize {
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

pub(crate) fn expected_crew_deliveries(
    crews: &EntitySet<Crew>,
    dest: Pos,
) -> HashMap<ItemType, usize> {
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

pub(crate) fn proceed_excavate(t: &mut f64, speed: f64, inventory: &mut Inventory) -> bool {
    if 0. < *t {
        let before_amount = (*t / LABOR_EXCAVATE_TIME * EXCAVATE_ORE_AMOUNT as f64).ceil() as usize;
        *t = (*t - speed).max(0.);
        let after_amount = (*t / LABOR_EXCAVATE_TIME * EXCAVATE_ORE_AMOUNT as f64).ceil() as usize;
        for _ in after_amount..before_amount {
            let entry = inventory.entry(ItemType::RawOre).or_default();
            *entry += 1;
        }
        true
    } else {
        false
    }
}
