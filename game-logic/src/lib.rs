pub use crate::{
    building::Recipe,
    construction::get_build_menu,
    conveyor::Conveyor,
    crew::Crew,
    direction::Direction,
    game::{AsteroidColoniesGame, SerializeGame},
    inventory::{CountableInventory, Inventory},
    items::ItemType,
    tile::{new_hasher, Chunk, ImageIdx, Position, Tile, TileState, Tiles, CHUNK_SIZE},
    transport::{Transport, TransportPayload},
    xor128::Xor128,
};

pub mod building;
pub mod construction;
pub mod conveyor;
mod crew;
mod direction;
mod entity;
mod game;
mod inventory;
mod items;
mod push_pull;
pub mod task;
mod tile;
mod transport;
mod xor128;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[macro_export]
macro_rules! hash_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
    { } => {
        ::std::collections::HashMap::new()
    }
}

#[macro_export]
macro_rules! btree_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::BTreeMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
    { } => {
        ::std::collections::BTreeMap::new()
    }
}

#[macro_export]
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[cfg(not(target_family = "wasm"))]
pub fn log(s: &str) {
    println!("{}", s);
}

pub const TILE_SIZE: f64 = 32.;
pub const WIDTH: usize = 100;
pub const HEIGHT: usize = 100;

pub type Pos = [i32; 2];

#[cfg(not(target_arch = "wasm32"))]
fn measure_time<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = std::time::Instant::now();
    let ret = f();
    (ret, start.elapsed().as_secs_f64())
}

#[cfg(target_arch = "wasm32")]
fn measure_time<T>(f: impl FnOnce() -> T) -> (T, f64) {
    (f(), 0.)
}
