use serde::{Deserialize, Serialize};
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    hash::Hash,
};

use crate::{
    direction::Direction, entity::EntityIterMutExt, items::ItemType, AsteroidColoniesGame,
    Conveyor, Pos,
};

/// Transporting item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transport {
    pub src: Pos,
    pub dest: Pos,
    pub item: ItemType,
    pub amount: usize,
    pub path: Vec<Pos>,
}

impl AsteroidColoniesGame {
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
            let building = self
                .buildings
                .items_mut()
                .find(|b| intersects(b.pos, b.type_.size(), t.dest));
            if let Some(building) = building {
                if building.inventory_size() + t.amount <= building.type_.capacity() {
                    *building.inventory.entry(t.item).or_default() += t.amount;
                    t.path.clear();
                    return true;
                }
            }
            false
        };

        let occupied: HashSet<_> = self
            .transports
            .iter()
            .filter_map(|t| t.path.last().copied())
            .collect();

        for t in &mut self.transports {
            if t.path.len() <= 1 {
                let delivered = check_construction(t) || check_building(t);
                if !delivered {
                    let tiles = &self.tiles;
                    let return_path = find_multipath(
                        std::iter::once(t.dest),
                        |pos| pos == t.src,
                        |from_direction, pos| {
                            let tile = &tiles[pos];
                            if let Some(from_direction) = from_direction {
                                matches!(tile.conveyor, Conveyor::One(_, dir) if dir == from_direction)
                                    && tile.conveyor.is_some()
                            } else {
                                tile.conveyor.is_some()
                            }
                        },
                    );
                    if let Some(return_path) = return_path {
                        std::mem::swap(&mut t.src, &mut t.dest);
                        t.path = return_path;
                    }
                }
            } else if t.path.len() <= 2
                || t.path
                    .get(t.path.len() - 2)
                    .map(|pos| !occupied.contains(pos))
                    .unwrap_or(true)
            {
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

/// Conveyor position, or composite position.
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) struct CPos {
    /// 2D coordinates
    pub pos: Pos,
    /// layer flag (true = second layer)
    pub level: bool,
}

impl CPos {
    pub fn new(pos: Pos, level: bool) -> Self {
        Self { pos, level }
    }
}

impl std::ops::Add for CPos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        CPos {
            pos: [self.pos[0] + rhs.pos[0], self.pos[1] + rhs.pos[1]],
            level: self.level,
        }
    }
}

pub(crate) fn find_path(
    start: [i32; 2],
    goal: [i32; 2],
    is_passable: impl Fn([i32; 2]) -> bool,
) -> Option<Vec<[i32; 2]>> {
    find_multipath(
        [start].into_iter(),
        |pos| pos == goal,
        |_, pos| is_passable(pos),
    )
}

pub(crate) fn find_multipath(
    start: impl Iterator<Item = [i32; 2]>,
    goal: impl Fn([i32; 2]) -> bool,
    is_passable: impl Fn(Option<Direction>, Pos) -> bool,
) -> Option<Vec<[i32; 2]>> {
    find_multipath_should_expand(start, goal, is_passable, |_, _, _| LevelTarget::One)
}

pub(crate) enum LevelTarget {
    None,
    One,
    Two,
}

/// A generic path finding logic with potentially multiple starts and multiple goals.
///
/// * `start` is an iterator over `Pos`, which can yield one or more items. Typically it is
///   `iter()` on a hash set.
/// * `goal` can be arbitrary set of positions, so it is given as a callback.
/// * `is_passable` takes 2 arguments, first is the direction that the search came from, second is
/// the position.
/// * `should_expand` takes 3 arguments, direction from, position and direction to, and returns
/// if we should expand to that tile. It is used to implement self-intersecting conveyor tiles.
pub(crate) fn find_multipath_should_expand(
    start: impl Iterator<Item = Pos>,
    goal: impl Fn(Pos) -> bool,
    is_passable: impl Fn(Option<Direction>, Pos) -> bool,
    should_expand: impl Fn(Direction, CPos, Option<Direction>) -> LevelTarget,
) -> Option<Vec<Pos>> {
    #[derive(Clone, Copy)]
    struct Entry {
        pos: CPos,
        dist: usize,
        from: Option<(Direction, CPos)>,
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

    type VisitedMap = HashMap<CPos, Entry>;
    let mut visited = VisitedMap::new();
    let mut next_set = BinaryHeap::new();
    let insert_neighbors = |next_set: &mut BinaryHeap<Entry>,
                            visited: &VisitedMap,
                            pos: CPos,
                            dist: usize,
                            from: Option<Direction>| {
        for dir in Direction::all() {
            let level = match should_expand(dir, pos, from) {
                LevelTarget::One => false,
                LevelTarget::Two => true,
                _ => continue,
            };
            let dir_vec = CPos::new(dir.to_vec(), false);
            let mut next_pos = pos + dir_vec;
            next_pos.level = level;
            if visited.get(&next_pos).is_some_and(|e| e.dist <= dist) {
                continue;
            }
            next_set.push(Entry {
                pos: next_pos,
                dist: dist + 1,
                from: Some((dir, pos)),
            });
        }
    };
    for s_pos in start {
        let s_cpos = CPos::new(s_pos, false);
        visited.insert(
            s_cpos,
            Entry {
                pos: s_cpos,
                dist: 0,
                from: None,
            },
        );
        insert_neighbors(&mut next_set, &visited, s_cpos, 0, None);
    }
    while let Some(next) = next_set.pop() {
        let from_dir = next.from.map(|(dir, _)| dir);
        if !is_passable(from_dir, next.pos.pos) {
            continue;
        }
        if goal(next.pos.pos) {
            let mut cursor = Some(next);
            let mut nodes = vec![];
            while let Some(cursor_entry) = cursor {
                nodes.push(cursor_entry.pos.pos);
                cursor = cursor_entry
                    .from
                    .and_then(|(_, pos)| visited.get(&pos))
                    .copied();
            }
            return Some(nodes);
        }
        visited.insert(next.pos, next);
        insert_neighbors(&mut next_set, &visited, next.pos, next.dist, from_dir);
    }
    None
}
