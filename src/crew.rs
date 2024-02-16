use crate::task::GlobalTask;
use crate::{console_log, Cell, Pos};

use crate::{transport::find_path, AsteroidColonies, CellState, WIDTH};

enum CrewTask {
    Return,
    Excavate(Pos),
}

pub(crate) struct Crew {
    pub pos: Pos,
    pub path: Option<Vec<Pos>>,
    pub from: Pos,
    task: Option<CrewTask>,
    to_delete: bool,
}

impl Crew {
    pub fn new(pos: Pos, target: Pos, cells: &[Cell]) -> Option<Self> {
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
            task: Some(CrewTask::Excavate(target)),
            to_delete: false,
        })
    }

    pub fn target(&self) -> Option<Pos> {
        match self.task {
            Some(CrewTask::Excavate(pos)) => Some(pos),
            _ => None,
        }
    }
}

impl AsteroidColonies {
    pub(super) fn process_crews(&mut self) {
        let mut try_return = |crew: &mut Crew| {
            if let Some(building) = self.buildings.iter_mut().find(|b| b.pos == crew.from) {
                building.crews += 1;
                crew.to_delete = true;
            }
        };

        for crew in &mut self.crews {
            // console_log!("crew has path: {:?}", crew.path.as_ref().map(|p| p.len()));
            if let Some(path) = &mut crew.path {
                if path.len() <= 1 {
                    crew.path = None;
                    if matches!(crew.task, Some(CrewTask::Return)) {
                        try_return(crew);
                    }
                } else if let Some(pos) = path.pop() {
                    crew.pos = pos;
                }
            } else if let Some(CrewTask::Excavate(ct_pos)) = crew.task {
                'outer: {
                    for gtask in self.global_tasks.iter_mut() {
                        if let GlobalTask::Excavate(t, gt_pos) = gtask {
                            if ct_pos == *gt_pos {
                                *t -= 1.;
                                break 'outer;
                            }
                        }
                    }
                    crew.task = None;
                }
            } else {
                console_log!("Returning home at {:?}", crew.from);
                if crew.from == crew.pos {
                    try_return(crew);
                } else if let Some(path) = find_path(crew.pos, crew.from, |pos| {
                    matches!(
                        self.cells[pos[0] as usize + pos[1] as usize * WIDTH].state,
                        CellState::Empty
                    ) || pos == crew.from
                }) {
                    crew.task = Some(CrewTask::Return);
                    crew.path = Some(path);
                }
            }
        }

        self.crews.retain(|c| !c.to_delete);
    }
}