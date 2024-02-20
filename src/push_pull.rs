//! Push and pull items over conveyor network
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::{
    building::Building,
    conveyor::Conveyor,
    transport::{expected_deliveries, find_multipath, Transport},
    Cell, Direction, ItemType, Pos, WIDTH,
};

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
            if cell.conveyor.is_some() && start_neighbors.contains(&pos) {
                // crate::console_log!("next to start");
                return true;
            }
            if !prev_tile_connects_to(cells, from_direction, pos) {
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

pub(crate) fn push_outputs(
    cells: &impl TileSampler,
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
                if intersects(pos) {
                    return true;
                }
                let Some(cell) = cells.at(pos) else {
                    return false;
                };
                if cell.conveyor.is_some() && start_neighbors.contains(&pos) {
                    // crate::console_log!("next to start");
                    return true;
                }
                if !prev_tile_connects_to(cells, from_direction, pos) {
                    return false;
                }
                from_direction.map(|from_direction| {
                    matches!(cell.conveyor, Conveyor::One(dir, _) if dir == from_direction.reverse())
                }).unwrap_or_else(||cell.conveyor.is_some())
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

fn prev_tile_connects_to(cells: &impl TileSampler, from_dir: Option<Direction>, pos: Pos) -> bool {
    from_dir
        .map(|dir| {
            let dir_vec = dir.to_vec();
            let prev_pos = [pos[0] - dir_vec[0], pos[1] - dir_vec[1]];
            let Some(prev_cell) = cells.at(prev_pos) else {
                return true;
            };
            // If the previous cell didn't have a conveyor, it's not a failure, because we want to be
            // able to depart from a building.
            prev_cell.conveyor.to().map(|to| to == dir).unwrap_or(true)
        })
        .unwrap_or(true)
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
