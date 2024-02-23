use super::AsteroidColonies;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{
    construction::ConstructionType,
    task::{
        Direction, GlobalTask, Task, EXCAVATE_TIME, LABOR_EXCAVATE_TIME, MOVE_ITEM_TIME, MOVE_TIME,
    },
    BuildingType, Cell, CellState, Conveyor, ItemType, HEIGHT, WIDTH,
};

pub(crate) const TILE_SIZE: f64 = 32.;
const ITEM_SIZE: f64 = 16.;
const BAR_MARGIN: f64 = 4.;
const BAR_WIDTH: f64 = TILE_SIZE - BAR_MARGIN * 2.;
const BAR_HEIGHT: f64 = 6.;
const SPACE_BIT: u8 = 32;
const NEIGHBOR_BITS: u8 = 0x1f;

#[wasm_bindgen]
impl AsteroidColonies {
    pub fn render(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#000000"));
        context.clear_rect(0., 0., self.viewport.size[0], self.viewport.size[1]);
        context.set_fill_style(&JsValue::from("#ff0000"));
        let vp = &self.viewport;
        let offset = [vp.offset[0].round(), vp.offset[1].round()];

        let render_power_grid = |context: &CanvasRenderingContext2d, x, y| {
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &self.assets.img_power_grid,
                0.,
                0.,
                TILE_SIZE,
                TILE_SIZE,
                x,
                y,
                TILE_SIZE,
                TILE_SIZE,
            )
        };

        let render_conveyor_layer =
            |context: &CanvasRenderingContext2d, x, y, conv: (Direction, Direction)| {
                let (sx, sy) = match conv {
                    (from, to) => {
                        let mut sy = match to {
                            Direction::Left => 0.,
                            Direction::Up => TILE_SIZE,
                            Direction::Right => 2. * TILE_SIZE,
                            Direction::Down => 3. * TILE_SIZE,
                        };
                        let sx = match from {
                            Direction::Left => 0.,
                            Direction::Up => TILE_SIZE,
                            Direction::Right => 2. * TILE_SIZE,
                            Direction::Down => 3. * TILE_SIZE,
                        };
                        if sx <= sy {
                            sy -= TILE_SIZE;
                        }
                        (sx, sy)
                    }
                };
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_conveyor,
                        sx,
                        sy,
                        TILE_SIZE,
                        TILE_SIZE,
                        x,
                        y,
                        TILE_SIZE,
                        TILE_SIZE,
                    )
            };

        let render_conveyor = |context: &CanvasRenderingContext2d,
                               x,
                               y,
                               conv: Conveyor|
         -> Result<(), JsValue> {
            match conv {
                Conveyor::One(from, to) => render_conveyor_layer(context, x, y, (from, to))?,
                Conveyor::Two(first, second) => {
                    render_conveyor_layer(context, x, y, first)?;
                    render_conveyor_layer(context, x, y, second)?;
                }
                Conveyor::Splitter(from) => {
                    let sx = match from {
                        Direction::Left => 0.,
                        Direction::Up => TILE_SIZE,
                        Direction::Right => 2. * TILE_SIZE,
                        Direction::Down => 3. * TILE_SIZE,
                    };
                    context
                        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &self.assets.img_conveyor,
                            sx,
                            3. * TILE_SIZE,
                            TILE_SIZE,
                            TILE_SIZE,
                            x,
                            y,
                            TILE_SIZE,
                            TILE_SIZE,
                        )?;
                }
                _ => {}
            };
            Ok(())
        };

        let mut rendered_cells = 0;
        let mut render_cell = |ix: i32, iy: i32| -> Result<(), JsValue> {
            let y = iy as f64 * TILE_SIZE + offset[1];
            let x = ix as f64 * TILE_SIZE + offset[0];
            if ix < 0 || (WIDTH as i32) <= ix || iy < 0 || (HEIGHT as i32) <= iy {
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_bg,
                        0.,
                        2. * TILE_SIZE,
                        TILE_SIZE,
                        TILE_SIZE,
                        x,
                        y,
                        TILE_SIZE,
                        TILE_SIZE,
                    )?;
                rendered_cells += 1;
                return Ok(());
            }
            let cell = &self.cells[ix as usize + iy as usize * WIDTH];
            let (sx, sy) = match cell.state {
                CellState::Empty => (0., TILE_SIZE),
                CellState::Solid => (0., 0.),
                CellState::Space => (0., 2. * TILE_SIZE),
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

            let mut render_quarter_tile = |image: u8, xofs, yofs| -> Result<(), JsValue> {
                let srcx = (image & NEIGHBOR_BITS) % 4;
                let srcy = (image & NEIGHBOR_BITS) / 4;
                let bg_y = if image & SPACE_BIT != 0 {
                    2. * TILE_SIZE
                } else {
                    TILE_SIZE
                };
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_bg,
                        0.,
                        bg_y,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        x + xofs,
                        y + yofs,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                    )?;
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        &self.assets.img_bg,
                        srcx as f64 * TILE_SIZE / 2.,
                        srcy as f64 * TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                        x + xofs,
                        y + yofs,
                        TILE_SIZE / 2.,
                        TILE_SIZE / 2.,
                    )?;
                rendered_cells += 1;
                Ok(())
            };

            if cell.image_lt & NEIGHBOR_BITS != 0 {
                render_quarter_tile(cell.image_lt, 0., 0.)?;
            }
            if cell.image_lb & NEIGHBOR_BITS != 0 {
                render_quarter_tile(cell.image_lb, 0., TILE_SIZE / 2.)?;
            }
            if cell.image_rb & NEIGHBOR_BITS != 0 {
                render_quarter_tile(cell.image_rb, TILE_SIZE / 2., TILE_SIZE / 2.)?;
            }
            if cell.image_rt & NEIGHBOR_BITS != 0 {
                render_quarter_tile(cell.image_rt, TILE_SIZE / 2., 0.)?;
            }

            if cell.power_grid {
                render_power_grid(context, x, y)?;
            }
            render_conveyor(context, x, y, cell.conveyor)?;
            rendered_cells += 1;
            Ok(())
        };

        let ymin = ((-offset[1]).div_euclid(TILE_SIZE)) as i32;
        let ymax = (-offset[1] + vp.size[1] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        let xmin = ((-offset[0]).div_euclid(TILE_SIZE)) as i32;
        let xmax = (-offset[0] + vp.size[0] + TILE_SIZE).div_euclid(TILE_SIZE) as i32;
        for iy in ymin..ymax {
            for ix in xmin..xmax {
                render_cell(ix, iy)?;
            }
        }
        // console_log!("rendered_cells: {}", rendered_cells);

        for building in &self.buildings {
            let img = self.assets.building_to_img(building.type_);
            let (sx, sy) = match building.type_ {
                BuildingType::Assembler => {
                    if !matches!(building.task, Task::None) {
                        ((self.global_time % 4) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                BuildingType::Furnace => {
                    if !matches!(building.task, Task::None) {
                        ((self.global_time % 2 + 1) as f64 * TILE_SIZE * 2., 0.)
                    } else {
                        (0., 0.)
                    }
                }
                _ => (0., 0.),
            };
            let x = building.pos[0] as f64 * TILE_SIZE + offset[0];
            let y = building.pos[1] as f64 * TILE_SIZE + offset[1];
            let size = building.type_.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, sx, sy, width, height, x, y, width, height,
            )?;

            let task_target = match building.task {
                Task::Excavate(t, _) => Some((t, EXCAVATE_TIME)),
                Task::Move(t, _) => Some((t, MOVE_TIME)),
                Task::MoveItem { t, .. } => Some((t, MOVE_ITEM_TIME)),
                Task::Assemble { t, max_t, .. } => Some((t, max_t)),
                _ => None,
            };

            if let Some((t, max_time)) = task_target {
                RenderBar {
                    context,
                    x,
                    y,
                    v: t as f64,
                    max: max_time as f64,
                    scale: size[1] as f64,
                    color: "#00af00",
                }
                .render_bar();
            }

            let inventory_count: usize = building.inventory.iter().map(|item| *item.1).sum();
            if 0 < inventory_count {
                RenderBar {
                    context,
                    x,
                    y: y + TILE_SIZE - BAR_HEIGHT - BAR_MARGIN * 2.,
                    v: inventory_count as f64,
                    max: building.type_.capacity() as f64,
                    scale: size[1] as f64,
                    color: "#afaf00",
                }
                .render_bar();
            }

            if let Task::Move(_, path) = &building.task {
                context.set_stroke_style(&JsValue::from("#ff7f00"));
                context.set_line_width(3.);
                context.begin_path();
                for node in std::iter::once(&building.pos).chain(path.iter()) {
                    context.line_to(
                        (node[0] as f64 + 0.5) * TILE_SIZE + offset[0],
                        (node[1] as f64 + 0.5) * TILE_SIZE + offset[1],
                    );
                }
                context.stroke();
            }
        }

        const CREW_SIZE: f64 = 16.;
        const CREW_OFFSET: f64 = (TILE_SIZE - CREW_SIZE) * 0.5;

        for crew in &self.crews {
            let x = crew.pos[0] as f64 * TILE_SIZE + CREW_OFFSET + offset[0];
            let y = crew.pos[1] as f64 * TILE_SIZE + CREW_OFFSET + offset[1];
            let img = &self.assets.img_crew;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., CREW_SIZE, CREW_SIZE, x, y, CREW_SIZE, CREW_SIZE,
            )?;

            if let Some(path) = &crew.path {
                context.set_stroke_style(&JsValue::from("#7f00ff"));
                context.set_line_width(2.);
                context.begin_path();
                let mut first = true;
                for node in path.iter().chain(std::iter::once(&crew.pos)) {
                    let x = (node[0] as f64 + 0.5) * TILE_SIZE + offset[0];
                    let y = (node[1] as f64 + 0.5) * TILE_SIZE + offset[1];
                    if first {
                        first = false;
                        context.move_to(x, y);
                    } else {
                        context.line_to(x, y);
                    }
                }
                context.stroke();
            }
        }

        for task in &self.global_tasks {
            let task_target = match task {
                GlobalTask::Excavate(t, pos) => Some((*t, pos, LABOR_EXCAVATE_TIME)),
            };

            if let Some((t, pos, max_time)) = task_target {
                let x = pos[0] as f64 * TILE_SIZE + offset[0];
                let y = pos[1] as f64 * TILE_SIZE + offset[1];
                render_global_task_bar(context, [x, y], t, max_time);
            }
        }

        for construction in &self.constructions {
            let x = construction.pos[0] as f64 * TILE_SIZE + offset[0];
            let y = construction.pos[1] as f64 * TILE_SIZE + offset[1];
            match construction.get_type() {
                ConstructionType::Building(ty) => {
                    let img = self.assets.building_to_img(ty);
                    let size = ty.size();
                    let width = size[0] as f64 * TILE_SIZE;
                    let height = size[1] as f64 * TILE_SIZE;
                    context
                        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img, 0., 0., width, height, x, y, width, height,
                        )?;
                }
                ConstructionType::PowerGrid => render_power_grid(context, x, y)?,
                ConstructionType::Conveyor(conv) => render_conveyor(context, x, y, conv)?,
            }
            let img = if construction.canceling() {
                &self.assets.img_deconstruction
            } else {
                &self.assets.img_construction
            };
            let size = construction.size();
            let width = size[0] as f64 * TILE_SIZE;
            let height = size[1] as f64 * TILE_SIZE;
            const SRC_WIDTH: f64 = 64.;
            const SRC_HEIGHT: f64 = 64.;
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., SRC_WIDTH, SRC_HEIGHT, x, y, width, height,
            )?;
            render_global_task_bar(
                context,
                [x, y],
                construction.progress(),
                construction.recipe.time,
            );
            // if let Some((t, pos, max_time)) = task_target {
            //     render_global_task_bar(context, pos, t, max_time);
            // }
        }

        for (pos, conv) in self
            .conveyor_staged
            .iter()
            .filter(|(pos, _)| !self.conveyor_preview.contains_key(*pos))
            .chain(self.conveyor_preview.iter())
        {
            let x = pos[0] as f64 * TILE_SIZE + offset[0];
            let y = pos[1] as f64 * TILE_SIZE + offset[1];
            render_conveyor(context, x, y, *conv)?;
        }

        for t in &self.transports {
            context.set_stroke_style(&JsValue::from("#ffff00"));
            context.set_line_width(2.);
            context.begin_path();
            for node in &t.path {
                context.line_to(
                    (node[0] as f64 + 0.5) * TILE_SIZE + offset[0],
                    (node[1] as f64 + 0.5) * TILE_SIZE + offset[1],
                );
            }
            context.stroke();
            if let Some(pos) = t.path.last() {
                let (img, sw, sh) = match t.item {
                    ItemType::RawOre => (&self.assets.img_raw_ore, 16., 16.),
                    ItemType::IronIngot => (&self.assets.img_iron_ingot, 16., 16.),
                    ItemType::CopperIngot => (&self.assets.img_copper_ingot, 16., 16.),
                    ItemType::Cilicate => (&self.assets.img_cilicate, 16., 16.),
                    ItemType::Gear => (&self.assets.img_gear, 32., 32.),
                    ItemType::Wire => (&self.assets.img_wire, 32., 32.),
                    ItemType::Circuit => (&self.assets.img_circuit, 32., 32.),
                    ItemType::PowerGridComponent => (&self.assets.img_power_grid, 32., 32.),
                    ItemType::ConveyorComponent => (&self.assets.img_conveyor, 32., 32.),
                    ItemType::AssemblerComponent => (&self.assets.img_assembler, 32., 32.),
                };
                let tile_offset = (TILE_SIZE as f64 - ITEM_SIZE as f64) / 2.;
                let x = pos[0] as f64 * TILE_SIZE + tile_offset + offset[0];
                let y = pos[1] as f64 * TILE_SIZE + tile_offset + offset[1];
                context
                    .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                        img, 0., 0., sw, sh, x, y, ITEM_SIZE, ITEM_SIZE,
                    )?;
            }
        }

        if let Some(cursor) = self.cursor {
            let img = &self.assets.img_cursor;
            let x = cursor[0] as f64 * TILE_SIZE + offset[0];
            let y = cursor[1] as f64 * TILE_SIZE + offset[1];
            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                img, 0., 0., TILE_SIZE, TILE_SIZE, x, y, TILE_SIZE, TILE_SIZE,
            )?;
        }

        Ok(())
    }
}

fn render_global_task_bar(
    context: &CanvasRenderingContext2d,
    [x, y]: [f64; 2],
    t: f64,
    max_time: f64,
) {
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#7f0000"));
    context.fill_rect(x + BAR_MARGIN, y + BAR_MARGIN, BAR_WIDTH, BAR_HEIGHT);
    context.set_stroke_style(&JsValue::from("#000"));
    context.set_fill_style(&JsValue::from("#007f00"));
    context.fill_rect(
        x + BAR_MARGIN,
        y + BAR_MARGIN,
        t * BAR_WIDTH / max_time,
        BAR_HEIGHT,
    );
}

struct RenderBar<'a> {
    context: &'a CanvasRenderingContext2d,
    x: f64,
    y: f64,
    v: f64,
    max: f64,
    scale: f64,
    color: &'a str,
}

impl<'a> RenderBar<'a> {
    fn render_bar(&self) {
        let context = self.context;
        let (x, y) = (self.x, self.y);
        context.set_stroke_style(&JsValue::from("#000"));
        context.set_fill_style(&JsValue::from("#7f0000"));
        context.fill_rect(
            x + BAR_MARGIN,
            y + BAR_MARGIN,
            BAR_WIDTH * self.scale,
            BAR_HEIGHT,
        );
        context.set_stroke_style(&JsValue::from("#000"));
        context.set_fill_style(&JsValue::from(self.color));
        context.fill_rect(
            x + BAR_MARGIN,
            y + BAR_MARGIN,
            self.v * BAR_WIDTH * self.scale / self.max,
            BAR_HEIGHT,
        );
    }
}

#[allow(clippy::many_single_char_names)]
pub(crate) fn calculate_back_image(ret: &mut [Cell]) {
    for uy in 0..HEIGHT {
        let y = uy as i32;
        for ux in 0..WIDTH {
            let x = ux as i32;
            if !matches!(ret[(ux + uy * WIDTH) as usize].state, CellState::Solid) {
                let cell = &mut ret[(ux + uy * WIDTH) as usize];
                cell.image_lt = 0;
                cell.image_lb = 0;
                cell.image_rb = 0;
                cell.image_rt = 0;
                continue;
            }
            let get_at = |x: i32, y: i32| {
                if x < 0 || WIDTH as i32 <= x || y < 0 || HEIGHT as i32 <= y {
                    (0u8, 0u8)
                } else {
                    let state = ret[x as usize + y as usize * WIDTH].state;
                    (
                        !matches!(state, CellState::Solid) as u8,
                        matches!(state, CellState::Space) as u8,
                    )
                }
            };
            let l = get_at(x - 1, y);
            let t = get_at(x, y - 1);
            let r = get_at(x + 1, y);
            let b = get_at(x, y + 1);
            let lt = get_at(x - 1, y - 1);
            let rt = get_at(x + 1, y - 1);
            let rb = get_at(x + 1, y + 1);
            let lb = get_at(x - 1, y + 1);

            // Voting filter. If 2 neighboring tiles out of 3 around a corner is space, its background
            // should be space too.
            let vote = |a, b, c| if 2 <= a + b + c { SPACE_BIT } else { 0 };

            // Encode information of neighboring tiles around a quater piece of a tile into a byte.
            // Lower 5 bits (0-31) contains the offset of the image coordinates,
            // and the 6th bit indicates if it's Space (or Empty if unset)
            let cell = &mut ret[(ux + uy * WIDTH) as usize];
            cell.image_lt = match (l.0, lt.0, t.0) {
                (1, _, 1) => 2,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 2 + 4 * 4,
                (0, 1, 0) => 3 + 3 * 4,
                _ => 0,
            } | vote(l.1, lt.1, t.1);
            cell.image_lb = match (b.0, lb.0, l.0) {
                (1, _, 1) => 2 + 4,
                (0, _, 1) => 2 + 4 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 3 + 2 * 4,
                _ => 0,
            } | vote(b.1, lb.1, l.1);
            cell.image_rb = match (b.0, rb.0, r.0) {
                (1, _, 1) => 3 + 4,
                (0, _, 1) => 3 + 5 * 4,
                (1, _, 0) => 2 + 5 * 4,
                (0, 1, 0) => 2 + 2 * 4,
                _ => 0,
            } | vote(b.1, rb.1, r.1);
            cell.image_rt = match (r.0, rt.0, t.0) {
                (1, _, 1) => 3,
                (0, _, 1) => 3 + 4 * 4,
                (1, _, 0) => 3 + 5 * 4,
                (0, 1, 0) => 2 + 3 * 4,
                _ => 0,
            } | vote(r.1, rt.1, t.1);
        }
    }
}
