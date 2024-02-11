use crate::{building::Building, AsteroidColonies, ItemType, Pos};

/// Transporting item
#[derive(Clone, Debug)]
pub(crate) struct Transport {
    pub src: Pos,
    pub dest: Pos,
    pub item: ItemType,
    pub amount: usize,
    pub path: Vec<Pos>,
}

impl AsteroidColonies {
    pub(super) fn process_transports(&mut self) {
        let intersects = |b: &Building, [ix, iy]: Pos| {
            let size = b.type_.size();
            b.pos[0] <= ix
                && ix < size[0] as i32 + b.pos[0]
                && b.pos[1] <= iy
                && iy < size[1] as i32 + b.pos[1]
        };

        for t in &mut self.transports {
            if t.path.len() <= 1 {
                if let Some(building) = self.buildings.iter_mut().find(|b| intersects(b, t.dest)) {
                    if building.inventory_size() + t.amount <= building.type_.capacity() {
                        *building.inventory.entry(t.item).or_default() += t.amount;
                        t.path.clear();
                    }
                }
            } else {
                t.path.pop();
            }
        }

        self.transports.retain(|t| !t.path.is_empty());
    }
}
