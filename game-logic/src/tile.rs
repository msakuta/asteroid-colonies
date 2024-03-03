use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

use fnv::FnvHasher;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::{conveyor::Conveyor, Pos};

pub const CHUNK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TileState {
    Solid,
    Empty,
    Space,
}

impl Hash for TileState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((*self) as u8).hash(state)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq)]
pub struct Tile {
    pub state: TileState,
    pub power_grid: bool,
    pub conveyor: Conveyor,
    /// The index into the background image for quick rendering
    #[serde(skip)]
    pub image_lt: u8,
    #[serde(skip)]
    pub image_lb: u8,
    #[serde(skip)]
    pub image_rb: u8,
    #[serde(skip)]
    pub image_rt: u8,
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.power_grid == other.power_grid
            && self.conveyor == other.conveyor
    }
}

impl Hash for Tile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state);
        self.power_grid.hash(state);
        self.conveyor.hash(state);
    }
}

impl Tile {
    pub const fn new() -> Self {
        Self {
            state: TileState::Space,
            power_grid: false,
            conveyor: Conveyor::None,
            image_lt: 0,
            image_lb: 0,
            image_rb: 0,
            image_rt: 0,
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn new_with_conveyor(conveyor: Conveyor) -> Self {
        Self {
            state: TileState::Empty,
            power_grid: false,
            conveyor,
            image_lt: 0,
            image_lb: 0,
            image_rb: 0,
            image_rt: 0,
        }
    }

    pub(crate) const fn building() -> Self {
        Self {
            state: TileState::Empty,
            power_grid: true,
            conveyor: Conveyor::None,
            image_lt: 8,
            image_lb: 8,
            image_rb: 8,
            image_rt: 8,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Chunk {
    Tiles(Vec<Tile>, u64),
    Uniform(Tile, u64),
}

impl std::hash::Hash for Chunk {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Tiles(tiles, _) => tiles.hash(state),
            Self::Uniform(tile, _) => tile.hash(state),
        }
    }
}

fn new_hasher() -> FnvHasher {
    FnvHasher::default()
}

impl Chunk {
    pub fn new() -> Self {
        let mut hasher = new_hasher();
        let cell = Tile::new();
        cell.hash(&mut hasher);
        let hash = hasher.finish();
        Self::Uniform(Tile::new(), hash)
    }

    /// Attempt to compress uniform chunks and save space.
    /// Returns false if it is uniform space and can be removed without losing information.
    pub fn uniformify(&mut self) -> bool {
        match self {
            Chunk::Tiles(tiles, hash) => {
                let first = tiles[0];
                let is_uniform = tiles.iter().fold(true, |acc, cur| acc && first == *cur);
                if !is_uniform {
                    let mut hasher = new_hasher();
                    for tile in tiles {
                        tile.hash(&mut hasher);
                    }
                    *hash = hasher.finish();
                    return true;
                }
                if first == Tile::new() {
                    return false;
                }
                let mut hasher = new_hasher();
                first.hash(&mut hasher);
                let hash = hasher.finish();
                *self = Chunk::Uniform(first, hash);
                true
            }
            Chunk::Uniform(tile, _) => *tile != Tile::new(),
        }
    }

    /// Get the cached hash
    pub fn get_hash(&self) -> u64 {
        match self {
            Chunk::Tiles(_, hash) | Chunk::Uniform(_, hash) => *hash,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{},{}", self.x, self.y))
    }
}

struct PositionVisitor;

impl<'de> Visitor<'de> for PositionVisitor {
    type Value = Position;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string in pair of integers \"x,y\"")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let Some(comma) = value.find(',') else {
            return Err(serde::de::Error::custom("Needs comma"));
        };
        let first = &value[..comma];
        let last = &value[comma + 1..];
        let x = first.parse().map_err(serde::de::Error::custom)?;
        let y = last.parse().map_err(serde::de::Error::custom)?;
        Ok(Position { x, y })
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Position, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(PositionVisitor)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Tiles {
    pub(crate) chunks: HashMap<Position, Chunk>,
}

impl Tiles {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn filter_with_diffs(
        &self,
        chunks_digest: &HashMap<Position, u64>,
    ) -> Result<Self, String> {
        let chunks: HashMap<Position, Chunk> = self
            .chunks()
            .iter()
            .filter_map(|(pos, c)| {
                if chunks_digest
                    .get(pos)
                    .map(|d| c.get_hash() != *d)
                    .unwrap_or(true)
                {
                    Some((*pos, c.clone()))
                } else {
                    None
                }
            })
            .collect();
        Ok(Self { chunks })
    }

    // pub fn iter(&self) -> TilesIter {
    //     TilesIter::new(self)
    // }

    pub fn chunks(&self) -> &HashMap<Position, Chunk> {
        &self.chunks
    }

    pub fn try_get_mut(&mut self, index: [i32; 2]) -> Option<&mut Tile> {
        let chunk_pos = Position {
            x: index[0].div_euclid(CHUNK_SIZE as i32),
            y: index[1].div_euclid(CHUNK_SIZE as i32),
        };
        self.chunks
            .get_mut(&chunk_pos)
            .and_then(|chunk| match chunk {
                Chunk::Tiles(tiles, _) => {
                    let tile_pos = [
                        index[0].rem_euclid(CHUNK_SIZE as i32),
                        index[1].rem_euclid(CHUNK_SIZE as i32),
                    ];
                    Some(&mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE])
                }
                Chunk::Uniform(_, _) => None,
            })
    }

    pub fn uniformify(&mut self) {
        self.chunks.retain(|_k, v| v.uniformify());
    }
}

impl Index<[i32; 2]> for Tiles {
    type Output = Tile;
    fn index(&self, index: [i32; 2]) -> &Self::Output {
        static SPACE: Tile = Tile::new();
        let chunk_pos = Position {
            x: index[0].div_euclid(CHUNK_SIZE as i32),
            y: index[1].div_euclid(CHUNK_SIZE as i32),
        };
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| match chunk {
                Chunk::Tiles(tiles, _) => {
                    let tile_pos = [
                        index[0].rem_euclid(CHUNK_SIZE as i32),
                        index[1].rem_euclid(CHUNK_SIZE as i32),
                    ];
                    &tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
                }
                Chunk::Uniform(tile, _) => tile,
            })
            .unwrap_or(&SPACE)
    }
}

impl IndexMut<[i32; 2]> for Tiles {
    /// Allocate a chunk if the given position doesn't have one.
    fn index_mut(&mut self, index: [i32; 2]) -> &mut Self::Output {
        let chunk_pos = Position {
            x: index[0].div_euclid(CHUNK_SIZE as i32),
            y: index[1].div_euclid(CHUNK_SIZE as i32),
        };
        let chunk = self.chunks.entry(chunk_pos).or_insert_with(Chunk::new);
        let tile_pos = [
            index[0].rem_euclid(CHUNK_SIZE as i32),
            index[1].rem_euclid(CHUNK_SIZE as i32),
        ];
        match chunk {
            Chunk::Tiles(tiles, _) => {
                &mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
            }
            Chunk::Uniform(tile, _) => {
                let mut tiles = vec![Tile::new(); CHUNK_SIZE * CHUNK_SIZE];
                tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE] = *tile;
                let mut hasher = new_hasher();
                for tile in &tiles {
                    tile.hash(&mut hasher);
                }
                let hash = hasher.finish();
                *chunk = Chunk::Tiles(tiles, hash);
                match chunk {
                    Chunk::Tiles(tiles, _) => {
                        &mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[allow(dead_code)]
pub struct TilesIter<'a> {
    // tiles: &'a Tiles,
    iter_chunks: Option<Box<dyn Iterator<Item = (&'a Position, &'a Chunk)> + 'a>>,
    chunk_pos: Option<Position>,
    iter: Option<Box<dyn Iterator<Item = (usize, &'a Tile)> + 'a>>,
}

#[allow(dead_code)]
impl<'a> TilesIter<'a> {
    pub fn new(tiles: &'a Tiles) -> Self {
        let mut iter_chunks = tiles.chunks.iter();
        let first = iter_chunks.next();
        let chunk_pos = first.map(|(k, _)| *k);
        let iter = first.map(|(_, v)| match v {
            Chunk::Tiles(tiles, _) => Box::new(tiles.iter().enumerate()) as _,
            Chunk::Uniform(tile, _) => Box::new(std::iter::once(tile).enumerate()) as _,
        });
        Self {
            // tiles,
            iter_chunks: Some(Box::new(iter_chunks) as _),
            chunk_pos,
            iter,
        }
    }
}

impl<'a> Iterator for TilesIter<'a> {
    type Item = (Pos, &'a Tile);
    fn next(&mut self) -> Option<Self::Item> {
        let Some(iter) = self.iter.as_mut() else {
            return None;
        };
        let Some(chunk_pos) = self.chunk_pos else {
            return None;
        };
        match iter.next() {
            Some((i, item)) => {
                let pos = [(i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32];
                Some((
                    [
                        chunk_pos.x * CHUNK_SIZE as i32 + pos[0],
                        chunk_pos.y * CHUNK_SIZE as i32 + pos[1],
                    ],
                    item,
                ))
            }
            None => {
                let Some((chunk_pos, chunk)) = self.iter_chunks.as_mut().and_then(|i| i.next())
                else {
                    return None;
                };
                let mut iter: Box<dyn Iterator<Item = (usize, &'a Tile)>> = match chunk {
                    Chunk::Tiles(tiles, _) => Box::new(tiles.iter().enumerate()) as _,
                    Chunk::Uniform(tile, _) => Box::new(std::iter::once(tile).enumerate()) as _,
                };
                let ret = iter.next();
                self.iter = Some(iter);
                ret.map(|(i, c)| {
                    (
                        [
                            chunk_pos.x * CHUNK_SIZE as i32 + i.rem_euclid(CHUNK_SIZE) as i32,
                            chunk_pos.y * CHUNK_SIZE as i32 + i.div_euclid(CHUNK_SIZE) as i32,
                        ],
                        c,
                    )
                })
            }
        }
    }
}
