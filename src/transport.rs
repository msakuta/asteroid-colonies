use std::collections::{BinaryHeap, HashMap};

use crate::{task::Direction, AsteroidColonies, ItemType, Pos, WIDTH};

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
        let intersects = |pos: Pos, size: [usize; 2], [ix, iy]: Pos| {
            pos[0] <= ix
                && ix < size[0] as i32 + pos[0]
                && pos[1] <= iy
                && iy < size[1] as i32 + pos[1]
        };

        let mut check_construction = |t: &mut Transport| {
            if let Some(construction) = self
                .constructions
                .iter_mut()
                .find(|c| intersects(c.pos, c.size(), t.dest))
            {
                let arrived = construction.ingredients.get(&t.item).copied().unwrap_or(0);
                let demand = construction
                    .recipe
                    .ingredients
                    .get(&t.item)
                    .copied()
                    .unwrap_or(0);
                if arrived + t.amount <= demand {
                    *construction.ingredients.entry(t.item).or_default() += t.amount;
                    t.path.clear();
                    return true;
                }
            }
            false
        };

        let mut check_building = |t: &mut Transport| {
            if let Some(building) = self
                .buildings
                .iter_mut()
                .find(|b| intersects(b.pos, b.type_.size(), t.dest))
            {
                if building.inventory_size() + t.amount <= building.type_.capacity() {
                    *building.inventory.entry(t.item).or_default() += t.amount;
                    t.path.clear();
                    return true;
                }
            }
            false
        };

        for t in &mut self.transports {
            if t.path.len() <= 1 {
                let delivered = check_construction(t) || check_building(t);
                if !delivered {
                    let cells = &self.cells;
                    let return_path = find_path(t.dest, t.src, |pos| {
                        let cell = &cells[pos[0] as usize + pos[1] as usize * WIDTH];
                        cell.conveyor
                    });
                    if let Some(return_path) = return_path {
                        std::mem::swap(&mut t.src, &mut t.dest);
                        t.path = return_path;
                    }
                }
            } else {
                t.path.pop();
            }
        }

        self.transports.retain(|t| !t.path.is_empty());
    }
}

/// Count all items in delivery flight and sum up in a single HashMap.
pub(crate) fn expected_deliveries(transports: &[Transport], dest: Pos) -> HashMap<ItemType, usize> {
    transports
        .iter()
        .filter(|t| t.dest == dest)
        .fold(HashMap::new(), |mut acc, cur| {
            *acc.entry(cur.item).or_default() += cur.amount;
            acc
        })
}

pub(crate) fn find_path(
    start: [i32; 2],
    goal: [i32; 2],
    is_passable: impl Fn([i32; 2]) -> bool,
) -> Option<Vec<[i32; 2]>> {
    #[derive(Clone, Copy)]
    struct Entry {
        pos: [i32; 2],
        dist: usize,
        from: Option<[i32; 2]>,
    }

    impl std::cmp::PartialEq for Entry {
        fn eq(&self, other: &Self) -> bool {
            self.dist.eq(&other.dist)
        }
    }

    impl std::cmp::Eq for Entry {}

    impl std::cmp::PartialOrd for Entry {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.dist.cmp(&other.dist).reverse())
        }
    }

    impl std::cmp::Ord for Entry {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.dist.cmp(&other.dist)
        }
    }

    type VisitedMap = HashMap<[i32; 2], Entry>;
    let mut visited = VisitedMap::new();
    visited.insert(
        start,
        Entry {
            pos: start,
            dist: 0,
            from: None,
        },
    );
    let mut next_set = BinaryHeap::new();
    let insert_neighbors =
        |next_set: &mut BinaryHeap<Entry>, visited: &VisitedMap, pos: [i32; 2], dist: usize| {
            for dir in [
                Direction::Left,
                Direction::Up,
                Direction::Right,
                Direction::Down,
            ] {
                let dir_vec = dir.to_vec();
                let next_pos = [pos[0] + dir_vec[0], pos[1] + dir_vec[1]];
                if visited.get(&next_pos).is_some_and(|e| e.dist <= dist) {
                    continue;
                }
                next_set.push(Entry {
                    pos: [pos[0] + dir_vec[0], pos[1] + dir_vec[1]],
                    dist: dist + 1,
                    from: Some(pos),
                });
            }
        };
    insert_neighbors(&mut next_set, &visited, start, 0);
    while let Some(next) = next_set.pop() {
        if !is_passable(next.pos) {
            continue;
        }
        if next.pos == goal {
            let mut cursor = Some(next);
            let mut nodes = vec![];
            while let Some(cursor_entry) = cursor {
                nodes.push(cursor_entry.pos);
                cursor = cursor_entry.from.and_then(|pos| visited.get(&pos)).copied();
            }
            return Some(nodes);
        }
        visited.insert(next.pos, next);
        insert_neighbors(&mut next_set, &visited, next.pos, next.dist);
    }
    None
}
