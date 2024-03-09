use crate::{
    construction::Construction,
    entity::{EntityEntry, EntityIterExt, EntitySet},
    push_pull::HasInventory,
    transport::find_multipath,
    Crew, TileState, Tiles, Transport,
};

use super::Building;

pub(super) struct Envs<'a> {
    pub first: &'a [EntityEntry<Building>],
    pub last: &'a [EntityEntry<Building>],
    pub transports: &'a EntitySet<Transport>,
    pub crews: &'a [Crew],
    pub tiles: &'a Tiles,
}

impl Building {
    pub(super) fn try_find_deliver(
        &mut self,
        construction: &Construction,
        envs: &Envs,
    ) -> Option<Crew> {
        construction
            .required_ingredients(envs.transports, envs.crews)
            .find_map(|(ty, _)| {
                let mut targets = std::collections::HashMap::new();
                for o in envs.first.items().chain(envs.last.items()) {
                    if 0 < o.inventory.get(&ty).copied().unwrap_or(0) {
                        let size = o.size();
                        for iy in 0..size[1] {
                            for ix in 0..size[0] {
                                targets.insert([o.pos[0] + ix as i32, o.pos[1] + iy as i32], o);
                            }
                        }
                    }
                }
                self.inventory
                    .get_mut(&ty)
                    .and_then(|n| {
                        if 0 < *n {
                            *n -= 1;
                            Crew::new_deliver(self.pos, construction.pos, ty, &envs.tiles)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        let path_to_source = find_multipath(
                            [self.pos].into_iter(),
                            |pos| targets.contains_key(&pos),
                            |_, pos| matches!(envs.tiles[pos].state, TileState::Empty),
                        );
                        path_to_source
                            .and_then(|src| src.first().copied())
                            .and_then(|src| {
                                Crew::new_pickup(self.pos, src, construction.pos, ty, envs.tiles)
                            })
                    })
            })
    }

    pub(super) fn try_find_pickup_and_deliver(
        &mut self,
        construction: &Construction,
        envs: &Envs,
    ) -> Option<Crew> {
        construction.extra_ingredients().find_map(|(ty, _)| {
            let path_to_dest = find_multipath(
                [construction.pos].into_iter(),
                |pos| {
                    envs.first.iter().chain(envs.last.iter()).any(|o| {
                        o.payload
                            .as_ref()
                            .map(|o| o.pos == pos && o.inventory_size() < o.type_.capacity())
                            .unwrap_or(false)
                    })
                },
                |_, pos| matches!(envs.tiles[pos].state, TileState::Empty),
            );

            path_to_dest
                .and_then(|dst| dst.first().copied())
                .and_then(|dst| Crew::new_pickup(self.pos, construction.pos, dst, ty, envs.tiles))
        })
    }

    pub(super) fn try_send_to_build(
        &mut self,
        construction: &Construction,
        envs: &Envs,
    ) -> Option<Crew> {
        if envs
            .crews
            .iter()
            .any(|crew| crew.target() == Some(construction.pos))
        {
            return None;
        }
        if construction.ingredients_satisfied() {
            Crew::new_build(self.pos, construction.pos, envs.tiles)
        } else {
            None
        }
    }
}
