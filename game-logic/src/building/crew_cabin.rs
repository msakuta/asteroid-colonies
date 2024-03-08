use crate::{
    construction::Construction, entity::EntityEntry, transport::find_multipath, Crew, TileState,
    Tiles, Transport,
};

use super::Building;

pub(super) struct Envs<'a> {
    pub first: &'a [EntityEntry<Building>],
    pub last: &'a [EntityEntry<Building>],
    pub transports: &'a [Transport],
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
                self.inventory
                    .get_mut(&ty)
                    .and_then(|n| {
                        if 0 < *n {
                            println!("new_deliver, sending a crew {:?}", construction.pos);
                            *n -= 1;
                            Crew::new_deliver(self.pos, construction.pos, ty, &envs.tiles)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        let path_to_source = find_multipath(
                            [self.pos].into_iter(),
                            |pos| {
                                envs.first.iter().chain(envs.last.iter()).any(|o| {
                                    o.payload
                                        .as_ref()
                                        .map(|o| {
                                            o.pos == pos
                                                && 0 < o.inventory.get(&ty).copied().unwrap_or(0)
                                        })
                                        .unwrap_or(false)
                                })
                            },
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
}
