use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use serde::{de::Visitor, Deserialize, Serialize};

use crate::{conveyor::Conveyor, Pos};

pub const CHUNK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CellState {
    Solid,
    Empty,
    Space,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

impl Cell {
    pub const fn new() -> Self {
        Self {
            state: CellState::Solid,
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
pub struct Chunk {
    tiles: Vec<Cell>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            tiles: vec![Cell::new(); CHUNK_SIZE * CHUNK_SIZE],
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

struct I32Visitor;

impl<'de> Visitor<'de> for I32Visitor {
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
        deserializer.deserialize_str(I32Visitor)
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
        self.chunks.get_mut(&chunk_pos).map(|chunk| {
            let tile_pos = [
                index[0].rem_euclid(CHUNK_SIZE as i32),
                index[1].rem_euclid(CHUNK_SIZE as i32),
            ];
            &mut chunk.tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
        })
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
            .map(|chunk| {
                let tile_pos = [
                    index[0].rem_euclid(CHUNK_SIZE as i32),
                    index[1].rem_euclid(CHUNK_SIZE as i32),
                ];
                &chunk.tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
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
        &mut chunk.tiles[tile_pos[0] as usize + tile_pos[1] as usize * CHUNK_SIZE]
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
        let iter = first.map(|(_, v)| Box::new(v.tiles.iter().enumerate()) as _);
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
                let mut iter = chunk.tiles.iter().enumerate();
                let ret = iter.next();
                self.iter = Some(Box::new(iter) as _);
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
