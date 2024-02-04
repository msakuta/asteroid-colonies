mod assets;
mod utils;

use std::fmt::Display;

use assets::Assets;
use wasm_bindgen::prelude::*;
use web_sys::{js_sys, CanvasRenderingContext2d};

#[macro_export]
macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[derive(Clone, Copy)]
enum CellState {
    Solid,
    Empty,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuildingType {
    Power,
    Excavator,
}

impl Display for BuildingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Power => write!(f, "Power"),
            Self::Excavator => write!(f, "Excavator"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Task {
    None,
    Excavate(usize, Direction),
    Move(usize, Direction),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Excavate(_, _) => write!(f, "Excavate"),
            Self::Move(_, _) => write!(f, "Move"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn to_vec(&self) -> [i32; 2] {
        match self {
            Self::Left => [-1, 0],
            Self::Up => [0, -1],
            Self::Right => [1, 0],
            Self::Down => [0, 1],
        }
    }
}

struct Building {
    pos: [i32; 2],
    type_: BuildingType,
    task: Task,
}

const WIDTH: usize = 20;
const HEIGHT: usize = 15;

const EXCAVATE_TIME: usize = 10;
const MOVE_TIME: usize = 2;

#[wasm_bindgen]
pub struct AsteroidColonies {
    cells: Vec<CellState>,
    buildings: Vec<Building>,
    assets: Assets,
}

#[wasm_bindgen]
impl AsteroidColonies {
    #[wasm_bindgen(constructor)]
    pub fn new(image_assets: js_sys::Array) -> Result<AsteroidColonies, JsValue> {
        let mut cells = vec![CellState::Solid; WIDTH * HEIGHT];
        let buildings = vec![
            Building {
                pos: [3, 4],
                type_: BuildingType::Power,
                task: Task::None,
            },
            Building {
                pos: [4, 4],
                type_: BuildingType::Excavator,
                task: Task::None,
            },
        ];
        for building in &buildings {
            let pos = building.pos;
            cells[pos[0] as usize + pos[1] as usize * WIDTH] = CellState::Empty;
        }
        Ok(Self {
            cells,
            buildings,
            assets: Assets::new(image_assets)?,
        })
    }

    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        const TILE_SIZE: f64 = 32.;
        const BAR_MARGIN: f64 = 4.;
        const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
        const BAR_HEIGHT: f64 = 6.;

        context.set_fill_style(&JsValue::from("#ff0000"));
        for (i, cell) in self.cells.iter().enumerate() {
            let iy = i / WIDTH;
            let y = iy as f64 * TILE_SIZE;
            let ix = i % WIDTH;
            let x = ix as f64 * TILE_SIZE;
            let (sx, sy) = match cell {
                CellState::Empty => (3. * TILE_SIZE, 3. * TILE_SIZE),
                CellState::Solid => (0., 0.),
            };
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.assets.img_bg,
                sx,
                sy,
                TILE_SIZE,
                TILE_SIZE,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
            )?;
        }

        for building in &self.buildings {
            let img = match building.type_ {
                BuildingType::Power => &self.assets.img_power,
                BuildingType::Excavator => &self.assets.img_excavator,
            };
            let x = building.pos[0] as f64 * TILE_SIZE;
            let y = building.pos[1] as f64 * TILE_SIZE;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., TILE_SIZE, TILE_SIZE, x, y, TILE_SIZE, TILE_SIZE,
            )?;
            match building.task {
                Task::Excavate(t, _) | Task::Move(t, _) => {
                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#7f0000"));
                    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
                    context.set_stroke_style(&JsValue::from("#000"));
                    context.set_fill_style(&JsValue::from("#007f00"));
                    context.fill_rect(
                        x + BAR_MARGIN,
                        y + BAR_MARGIN,
                        t as f64 * BAR_WIDTH / EXCAVATE_TIME as f64,
                        BAR_HEIGHT,
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn get_info(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if let Some(building) = self
            .buildings
            .iter()
            .find(|b| b.pos[0] == ix && b.pos[1] == iy)
        {
            Ok(JsValue::from(format!(
                "{} at {}, {}\nTask: {:?}",
                building.type_, building.pos[0], building.pos[1], building.task
            )))
        } else {
            Ok(JsValue::from(format!("Empty at {ix}, {iy}")))
        }
    }

    pub fn command(&mut self, com: &str, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let ix = x.div_euclid(32);
        let iy = y.div_euclid(32);
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(JsValue::from("Point outside cell"));
        }
        match com {
            "excavate" => self.excavate(ix, iy),
            "move" => self.move_(ix, iy),
            _ => Err(JsValue::from(format!("Unknown command: {}", com))),
        }
    }

    fn excavate(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if matches!(
            self.cells[ix as usize + iy as usize * WIDTH],
            CellState::Empty
        ) {
            return Err(JsValue::from("Already excavated"));
        }
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if iy == building.pos[1] {
                if ix - building.pos[0] == 1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Right);
                } else if ix - building.pos[0] == -1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Left);
                }
            }
            if ix == building.pos[0] {
                if iy - building.pos[0] == 1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Down);
                } else if iy - building.pos[0] == -1 {
                    building.task = Task::Excavate(EXCAVATE_TIME, Direction::Up);
                }
            }
        }
        Ok(JsValue::from(true))
    }

    fn move_(&mut self, ix: i32, iy: i32) -> Result<JsValue, JsValue> {
        if matches!(
            self.cells[ix as usize + iy as usize * WIDTH],
            CellState::Solid
        ) {
            return Err(JsValue::from("Needs excavation before moving"));
        }
        for building in &mut self.buildings {
            if building.type_ != BuildingType::Excavator {
                continue;
            }
            if iy == building.pos[1] {
                if ix - building.pos[0] == 1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Right);
                } else if ix - building.pos[0] == -1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Left);
                }
            }
            if ix == building.pos[0] {
                if iy - building.pos[0] == 1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Down);
                } else if iy - building.pos[0] == -1 {
                    building.task = Task::Move(MOVE_TIME, Direction::Up);
                }
            }
        }
        Ok(JsValue::from(true))
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        for building in &mut self.buildings {
            match building.task {
                Task::Excavate(ref mut t, dir) => {
                    if *t == 0 {
                        building.task = Task::None;
                        let dir_vec = dir.to_vec();
                        let [x, y] = [building.pos[0] + dir_vec[0], building.pos[1] + dir_vec[1]];
                        self.cells[x as usize + y as usize * WIDTH] = CellState::Empty;
                    } else {
                        *t -= 1;
                    }
                }
                Task::Move(ref mut t, dir) => {
                    if *t == 0 {
                        building.task = Task::None;
                        let dir_vec = dir.to_vec();
                        building.pos[0] += dir_vec[0];
                        building.pos[1] += dir_vec[1];
                    } else {
                        *t -= 1;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
