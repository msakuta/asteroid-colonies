use std::collections::HashMap;

use crate::{
    building::Building,
    console_log,
    construction::Construction,
    task::{GlobalTask, EXCAVATE_ORE_AMOUNT, LABOR_EXCAVATE_TIME},
    transport::find_path,
    AsteroidColonies, Cell, CellState, ItemType, Pos, WIDTH,
};

#[derive(Clone, Debug)]
enum CrewTask {
    None,
    Return,
    Excavate(Pos),
    Build(Pos),
    Pickup { src: Pos, dest: Pos, item: ItemType },
    Deliver { dst: Pos, item: ItemType },
}

#[derive(Debug)]
pub(crate) struct Crew {
    pub pos: Pos,
    pub path: Option<Vec<Pos>>,
    pub from: Pos,
    task: CrewTask,
    inventory: HashMap<ItemType, usize>,
    to_delete: bool,
}

impl Crew {
    pub fn new_task(pos: Pos, gtask: &GlobalTask, cells: &[Cell]) -> Option<Self> {
        let (target, task) = match gtask {
            GlobalTask::Excavate(_, pos) => (*pos, CrewTask::Excavate(*pos)),
            GlobalTask::Build(_, pos, _) => (*pos, CrewTask::Build(*pos)),
        };
        let path = find_path(pos, target, |pos| {
            matches!(
                cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                CellState::Empty
            ) || pos == target
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

    pub fn new_pickup(
        pos: Pos,
        src: Pos,
        dest: Pos,
        item: ItemType,
        cells: &[Cell],
    ) -> Option<Self> {
        let path = find_path(pos, src, |pos| {
            matches!(
                cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                CellState::Empty
            ) || pos == src
        })?;
        crate::console_log!("new_deliver");
        // Just to make sure if you can reach the destination from pickup
        if find_path(src, dest, |pos| {
            matches!(
                cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                CellState::Empty
            ) || pos == dest
        })
        .is_none()
        {
            return None;
        }
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task: CrewTask::Pickup { src, dest, item },
            inventory: HashMap::new(),
            to_delete: false,
        })
    }

    pub fn new_deliver(pos: Pos, dest: Pos, item: ItemType, cells: &[Cell]) -> Option<Self> {
        let path = find_path(pos, dest, |pos| {
            matches!(
                cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                CellState::Empty
            ) || pos == dest
        })?;
        Some(Self {
            pos,
            path: Some(path),
            from: pos,
            task: CrewTask::Deliver { dst: dest, item },
            inventory: HashMap::new(),
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
            if let GlobalTask::Excavate(t, gt_pos) = gtask {
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
        }
        self.task = CrewTask::None;
    }

    fn process_build_task(&mut self, global_tasks: &mut [GlobalTask], ct_pos: Pos) {
        for gtask in global_tasks.iter_mut() {
            if let GlobalTask::Build(t, gt_pos, _) = gtask {
                if ct_pos == *gt_pos && 0. < *t {
                    *t -= 1.;
                    if *t <= 0. {
                        self.task = CrewTask::None;
                    }
                    return;
                }
            }
        }
        self.task = CrewTask::None;
    }

    fn process_pickup_task(
        &mut self,
        item: ItemType,
        src: Pos,
        dest: Pos,
        cells: &[Cell],
        buildings: &mut [Building],
    ) {
        let Some(building) = buildings.iter_mut().find(|o| o.pos == src) else {
            self.task = CrewTask::None;
            return;
        };
        let Some(entry) = building.inventory.get_mut(&item) else {
            self.task = CrewTask::None;
            return;
        };
        if *entry == 0 {
            self.task = CrewTask::None;
            return;
        }
        *entry -= 1;
        *self.inventory.entry(item).or_default() += 1;
        let Some(path) = find_path(self.pos, dest, |pos| {
            matches!(
                cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                CellState::Empty
            ) || pos == dest
        }) else {
            return;
        };
        self.path = Some(path);
        self.task = CrewTask::Deliver { dst: dest, item };
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

impl AsteroidColonies {
    pub(super) fn process_crews(&mut self) {
        let try_return = |crew: &mut Crew, buildings: &mut [Building]| {
            if let Some(building) = buildings.iter_mut().find(|b| b.pos == crew.from) {
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
                    crew.process_build_task(&mut self.global_tasks, ct_pos);
                }
                CrewTask::Pickup { src, dest, item } => {
                    crew.process_pickup_task(item, src, dest, &self.cells, &mut self.buildings);
                }
                CrewTask::Deliver { dst, item } => {
                    crew.process_deliver_task(item, dst, &mut self.constructions);
                }
                _ => {
                    console_log!("Returning home at {:?}", crew.from);
                    if crew.from == crew.pos {
                        try_return(crew, &mut self.buildings);
                    } else if let Some(path) = find_path(crew.pos, crew.from, |pos| {
                        matches!(
                            self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                            CellState::Empty
                        ) || pos == crew.from
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
                    Some(item)
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
