use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use serde::{de::Visitor, Deserialize, Serialize};

use crate::{conveyor::Conveyor, Pos};

pub const CHUNK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CellState {
    Solid,
    Empty,
    Space,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq)]
pub struct Cell {
    pub state: CellState,
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

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.power_grid == other.power_grid
            && self.conveyor == other.conveyor
    }
}

impl Cell {
    pub const fn new() -> Self {
        Self {
            state: CellState::Space,
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
            state: CellState::Empty,
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
            state: CellState::Empty,
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
    Tiles(Vec<Cell>),
    Uniform(Cell),
}

impl Chunk {
    pub fn new() -> Self {
        Self::Uniform(Cell::new())
    }

    pub fn uniformify(&mut self) -> bool {
        match self {
            Chunk::Tiles(tiles) => {
                let first = tiles[0];
                let is_uniform = tiles.iter().fold(true, |acc, cur| acc && first == *cur);
                if is_uniform {
                    if first == Cell::new() {
                        return false;
                    }
                    *self = Chunk::Uniform(first);
                }
                true
            }
            Chunk::Uniform(tile) => *tile != Cell::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
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
    chunks: HashMap<Position, Chunk>,
}

impl Tiles {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn iter(&self) -> TilesIter {
        TilesIter::new(self)
    }

    pub fn try_get_mut(&mut self, index: [i32; 2]) -> Option<&mut Cell> {
        let chunk_pos = Position {
            x: index[0].div_euclid(CHUNK_SIZE as i32),
            y: index[1].div_euclid(CHUNK_SIZE as i32),
        };
        self.chunks
            .get_mut(&chunk_pos)
            .and_then(|chunk| match chunk {
                Chunk::Tiles(tiles) => {
                    let tile_pos = [
                        index[0].rem_euclid(CHUNK_SIZE as i32),
                        index[1].rem_euclid(CHUNK_SIZE as i32),
                    ];
                    Some(&mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE])
                }
                Chunk::Uniform(_) => None,
            })
    }

    pub fn uniformify(&mut self) {
        self.chunks.retain(|_k, v| v.uniformify())
    }
}

impl Index<[i32; 2]> for Tiles {
    type Output = Cell;
    fn index(&self, index: [i32; 2]) -> &Self::Output {
        static SPACE: Cell = Cell::new();
        let chunk_pos = Position {
            x: index[0].div_euclid(CHUNK_SIZE as i32),
            y: index[1].div_euclid(CHUNK_SIZE as i32),
        };
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| match chunk {
                Chunk::Tiles(tiles) => {
                    let tile_pos = [
                        index[0].rem_euclid(CHUNK_SIZE as i32),
                        index[1].rem_euclid(CHUNK_SIZE as i32),
                    ];
                    &tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
                }
                Chunk::Uniform(tile) => tile,
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
            Chunk::Tiles(tiles) => {
                &mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
            }
            Chunk::Uniform(tile) => {
                let mut tiles = vec![Cell::new(); CHUNK_SIZE * CHUNK_SIZE];
                tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE] = *tile;
                *chunk = Chunk::Tiles(tiles);
                match chunk {
                    Chunk::Tiles(tiles) => {
                        &mut tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

pub struct TilesIter<'a> {
    // tiles: &'a Tiles,
    iter_chunks: Option<Box<dyn Iterator<Item = (&'a Position, &'a Chunk)> + 'a>>,
    chunk_pos: Option<Position>,
    iter: Option<Box<dyn Iterator<Item = (usize, &'a Cell)> + 'a>>,
}

impl<'a> TilesIter<'a> {
    pub fn new(tiles: &'a Tiles) -> Self {
        let mut iter_chunks = tiles.chunks.iter();
        let first = iter_chunks.next();
        let chunk_pos = first.map(|(k, _)| *k);
        let iter = first.map(|(_, v)| match v {
            Chunk::Tiles(tiles) => Box::new(tiles.iter().enumerate()) as _,
            Chunk::Uniform(tile) => Box::new(std::iter::once(tile).enumerate()) as _,
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
    type Item = (Pos, &'a Cell);
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
                let mut iter: Box<dyn Iterator<Item = (usize, &'a Cell)>> = match chunk {
                    Chunk::Tiles(tiles) => Box::new(tiles.iter().enumerate()) as _,
                    Chunk::Uniform(tile) => Box::new(std::iter::once(tile).enumerate()) as _,
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
