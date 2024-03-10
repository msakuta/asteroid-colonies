//! Push and pull items over conveyor network
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::{
    building::Building,
    conveyor::Conveyor,
    direction::Direction,
    entity::{EntityEntry, EntityId, EntitySet, RefMutOption},
    items::ItemType,
    transport::{expected_deliveries, find_multipath_should_expand, CPos, LevelTarget, Transport},
    Pos, Tile, Tiles, WIDTH,
};

/// An abstraction of tile map where you can pick a tile from a position.
/// Used to mock in unit tests.
pub(crate) trait TileSampler {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile>;
}

impl TileSampler for &[Tile] {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile> {
        Some(&self[pos[0] as usize + pos[1] as usize * WIDTH])
    }
}

impl TileSampler for Vec<Tile> {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile> {
        Some(&self[pos[0] as usize + pos[1] as usize * WIDTH])
    }
}

impl TileSampler for Tiles {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile> {
        Some(&self[pos])
    }
}

/// Pull inputs over transportation network
pub(crate) fn pull_inputs(
    inputs: &HashMap<ItemType, usize>,
    tiles: &impl TileSampler,
    transports: &mut EntitySet<Transport>,
    expected_transports: &mut HashSet<EntityId>,
    this_pos: Pos,
    this_size: [usize; 2],
    this_inventory: &mut HashMap<ItemType, usize>,
    buildings: &EntitySet<Building>,
) {
    let intersects_goal = |[ix, iy]: [i32; 2]| {
        this_pos[0] <= ix
            && ix < this_size[0] as i32 + this_pos[0]
            && this_pos[1] <= iy
            && iy < this_size[1] as i32 + this_pos[1]
    };
    // let start = std::time::Instant::now();
    // crate::console_log!("pulling to at {:?} size {:?}", this_pos, this_size);
    let expected = expected_deliveries(transports, expected_transports);

    for (ty, count) in inputs {
        let this_count =
            this_inventory.get(ty).copied().unwrap_or(0) + expected.get(ty).copied().unwrap_or(0);
        if *count <= this_count {
            continue;
        }
        let Some((mut src, amount)) = find_from_inventory_mut(*ty, buildings) else {
            continue;
        };
        if amount == 0 {
            continue;
        }
        let size = src.type_.size();
        let start_pos = rect_iter(src.pos, size);
        let start_neighbors = neighbors_set(rect_iter(src.pos, size));
        let path = find_multipath_should_expand(
            start_pos,
            intersects_goal,
            |from_direction, pos| {
                if intersects_goal(pos) {
                    return true;
                }
                push_pull_passable(tiles, from_direction, &start_neighbors, pos)
            },
            |to, pos, from| push_pull_should_expand(tiles, to, pos, from),
        );
        let Some(path) = path else {
            continue;
        };
        let src_pos = src.pos;
        let src_count = src.inventory.entry(*ty).or_default();
        let amount = (*src_count).min(*count - this_count);
        let id = transports.insert(Transport {
            src: src_pos,
            dest: this_pos,
            path,
            item: *ty,
            amount,
        });
        expected_transports.insert(id);
        if *src_count <= amount {
            src.inventory.remove(ty);
        } else {
            *src_count -= amount;
        }
    }
    // let time = start.elapsed().as_secs_f64();
    // println!("pull_inputs took {} sec", time);
}

fn _find_from_other_inventory_mut<'a>(
    item: ItemType,
    first: &'a mut [EntityEntry<Building>],
    last: &'a mut [EntityEntry<Building>],
) -> Option<(&'a mut Building, usize)> {
    first.iter_mut().chain(last.iter_mut()).find_map(|o| {
        let Some(ref mut o) = o.payload.get_mut() else {
            return None;
        };
        let count = *o.inventory.get(&item)?;
        if count == 0 {
            return None;
        }
        Some((o, count))
    })
}

fn find_from_inventory_mut<'a>(
    item: ItemType,
    buildings: &'a EntitySet<Building>,
) -> Option<(RefMutOption<'a, Building>, usize)> {
    buildings.iter_borrow_mut().find_map(|o| {
        let count = *o.inventory.get(&item)?;
        if count == 0 {
            return None;
        }
        Some((o, count))
    })
}

fn _find_from_inventory<'a>(
    item: ItemType,
    mut iter: impl Iterator<Item = &'a Building>,
) -> Option<(&'a Building, usize)> {
    iter.find_map(|o| {
        let count = *o.inventory.get(&item)?;
        if count == 0 {
            return None;
        }
        Some((o, count))
    })
}

/// Return an iterator over tiles covering a rectangle specified by left top corner position and a size.
pub(crate) fn rect_iter(pos: Pos, size: [usize; 2]) -> impl Iterator<Item = Pos> {
    (0..size[0])
        .map(move |ix| (0..size[1]).map(move |iy| [pos[0] + ix as i32, pos[1] + iy as i32]))
        .flatten()
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

pub(crate) fn push_outputs<'a, 'b>(
    tiles: &impl TileSampler,
    transports: &mut EntitySet<Transport>,
    this: &mut impl HasInventory,
    buildings: &EntitySet<Building>,
    is_output: &impl Fn(ItemType) -> bool,
) where
    'b: 'a,
{
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
    // let start = std::time::Instant::now();
    let dest = buildings.iter_borrow_mut().find_map(|b| {
        if !b.type_.is_storage() {
            return None;
        }
        if b.type_.capacity()
            <= b.inventory_size()
                + expected_deliveries(transports, &b.expected_transports)
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
        let path = find_multipath_should_expand(
            start_pos(),
            |pos| pos == b.pos,
            |from_direction, pos| {
                if intersects(pos) {
                    return true;
                }
                push_pull_passable(tiles, from_direction, &start_neighbors, pos)
            },
            |to, pos, from| push_pull_should_expand(tiles, to, pos, from),
        )?;
        Some((b, path))
    });
    // let time = start.elapsed().as_secs_f64();
    // println!("searching {:?} nodes path took {} sec", dest.as_ref().map(|(_, path)| path.len()), time);

    // Push away outputs
    if let Some((mut dest, path)) = dest {
        let product = this
            .inventory()
            .iter_mut()
            .find(|(t, count)| is_output(**t) && 0 < **count);
        if let Some((&item, amount)) = product {
            let id = transports.insert(Transport {
                src: pos,
                dest: dest.pos,
                path,
                item,
                amount: 1,
            });
            dest.expected_transports.insert(id);
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

pub(crate) fn send_item<'a, 'b>(
    tiles: &impl TileSampler,
    transports: &mut EntitySet<Transport>,
    src: &mut impl HasInventory,
    dest_pos: Pos,
    buildings: &EntitySet<Building>,
    is_output: &impl Fn(ItemType) -> bool,
) -> Result<(), String>
where
    'b: 'a,
{
    let pos = src.pos();
    let size = src.size();
    let start_pos = || rect_iter(pos, size);
    let start_neighbors = neighbors_set(start_pos());
    let mut dest = buildings
        .iter_borrow_mut()
        .find(|b| b.intersects(dest_pos))
        .ok_or_else(|| "Destination did not have a building")?;
    let expected_inventory_size = dest.inventory_size()
        + expected_deliveries(transports, &dest.expected_transports)
            .values()
            .sum::<usize>();
    if dest.type_.capacity() <= expected_inventory_size {
        return Err("Destination capacity is full".to_string());
    }
    let path = find_multipath_should_expand(
        start_pos(),
        |pos| dest.intersects(pos),
        |from_direction, pos| {
            if dest.intersects(pos) {
                return true;
            }
            push_pull_passable(tiles, from_direction, &start_neighbors, pos)
        },
        |to, pos, from| push_pull_should_expand(tiles, to, pos, from),
    )
    .ok_or_else(|| "Could not find a path from source to dest")?;

    let (&item, amount) = src
        .inventory()
        .iter_mut()
        .find(|(t, count)| is_output(**t) && 0 < **count)
        .ok_or_else(|| "The designated item was not found")?;

    let id = transports.insert(Transport {
        src: pos,
        dest: dest.pos,
        path,
        item,
        amount: 1,
    });
    dest.expected_transports.insert(id);
    if *amount <= 1 {
        src.inventory().remove(&item);
    } else {
        *amount -= 1;
    }
    Ok(())
}

fn push_pull_passable(
    tiles: &impl TileSampler,
    from_direction: Option<Direction>,
    start_neighbors: &HashSet<Pos>,
    pos: Pos,
) -> bool {
    let Some(tile) = tiles.at(pos) else {
        return false;
    };
    if tile.conveyor.is_some() && start_neighbors.contains(&pos) {
        // crate::console_log!("next to start");
        return true;
    }
    if !prev_tile_connects_to(tiles, from_direction, pos) {
        return false;
    }
    from_direction
        .map(|from| tile.conveyor.has_from(from.reverse()))
        .unwrap_or_else(|| tile.conveyor.is_some())
}

fn prev_tile_connects_to(tiles: &impl TileSampler, from_dir: Option<Direction>, pos: Pos) -> bool {
    let Some(dir) = from_dir else {
        return true;
    };
    let dir_vec = dir.to_vec();
    let prev_pos = [pos[0] - dir_vec[0], pos[1] - dir_vec[1]];
    let Some(prev_tile) = tiles.at(prev_pos) else {
        return true;
    };
    // If the previous tile didn't have a conveyor, it's not a failure, because we want to be
    // able to depart from a building.
    prev_tile.conveyor.has_to(dir)
}

fn push_pull_should_expand(
    tiles: &impl TileSampler,
    to: Direction,
    cpos: CPos,
    from: Option<Direction>,
) -> LevelTarget {
    use Direction::*;
    let Some(tile) = tiles.at(cpos.pos) else {
        return LevelTarget::One;
    };
    let conv = &tile.conveyor;
    let dir_vec = to.to_vec();
    let next_pos = [cpos.pos[0] + dir_vec[0], cpos.pos[1] + dir_vec[1]];
    let Some(next_tile) = tiles.at(next_pos) else {
        return LevelTarget::One;
    };
    let next_conv = &next_tile.conveyor;
    if next_conv.has_two()
        && (conv.has_to(Up) || conv.has_to(Down))
        && (next_conv.has(Up) || next_conv.has(Down))
    {
        return LevelTarget::Two;
    }
    match tile.conveyor {
        Conveyor::One(_, _) => LevelTarget::One,
        Conveyor::Two(_, _) => {
            if from.is_some_and(|from| to == from) {
                LevelTarget::One
            } else {
                LevelTarget::None
            }
        }
        _ => LevelTarget::One,
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
