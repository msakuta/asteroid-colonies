use std::collections::HashMap;

use crate::{building::Recipe, conveyor::Conveyor, crew::Crew, transport::Transport};
pub use crate::{
    construction::get_build_menu,
    game::{AsteroidColoniesGame, SerializeGame},
    tile::{Chunk, Position, Tile, TileState, Tiles},
    xor128::Xor128,
};
use serde::{Deserialize, Serialize};

pub mod building;
pub mod construction;
pub mod conveyor;
mod crew;
mod game;
mod push_pull;
pub mod task;
mod tile;
mod transport;
mod xor128;

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
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

#[cfg(target = "wasm32-unknown-unknown")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

#[cfg(not(target = "wasm32-unknown-unknown"))]
pub(crate) fn log(s: &str) {
    println!("{}", s);
}

pub const TILE_SIZE: f64 = 32.;
pub const WIDTH: usize = 100;
pub const HEIGHT: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ItemType {
    /// Freshly dug soil from asteroid body. Hardly useful unless refined
    RawOre,
    IronIngot,
    CopperIngot,
    Cilicate,
    Gear,
    Wire,
    Circuit,
    PowerGridComponent,
    ConveyorComponent,
    AssemblerComponent,
}

static RECIPES: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
fn recipes() -> &'static [Recipe] {
    RECIPES.get_or_init(|| {
        vec![
            Recipe {
                inputs: hash_map!(ItemType::Wire => 1, ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::PowerGridComponent => 1),
                time: 100.,
            },
            Recipe {
                inputs: hash_map!(ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::ConveyorComponent => 1),
                time: 120.,
            },
            Recipe {
                inputs: hash_map!(ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::Gear => 2),
                time: 70.,
            },
            Recipe {
                inputs: hash_map!(ItemType::CopperIngot => 1),
                outputs: hash_map!(ItemType::Wire => 2),
                time: 50.,
            },
            Recipe {
                inputs: hash_map!(ItemType::Wire => 1, ItemType::IronIngot => 1),
                outputs: hash_map!(ItemType::Circuit => 1),
                time: 120.,
            },
            Recipe {
                inputs: hash_map!(ItemType::Gear => 2, ItemType::Circuit => 2),
                outputs: hash_map!(ItemType::AssemblerComponent => 1),
                time: 200.,
            },
        ]
    })
}

pub type Inventory = HashMap<ItemType, usize>;

pub type Pos = [i32; 2];
