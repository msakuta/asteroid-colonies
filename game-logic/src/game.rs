use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read};

use crate::{
    btree_map,
    building::{Building, BuildingType, Recipe},
    console_log,
    construction::{get_build_menu, Construction, ConstructionType},
    conveyor::Conveyor,
    crew::Crew,
    direction::Direction,
    entity::{EntitySet, RefOption},
    items::{recipes, ItemType},
    perlin_noise::gen_terms,
    push_pull::send_item,
    task::{BuildingTask, GlobalTask, MOVE_TIME},
    tile::CHUNK_SIZE,
    transport::{find_path, Transport},
    Pos, Position, Tile, TileState, Tiles, Xor128, HEIGHT, WIDTH,
};

pub(crate) const PERLIN_BITS: u32 = 4;

pub type CalculateBackImage = Box<dyn Fn(&mut Tiles) + Send + Sync>;

pub struct AsteroidColoniesGame {
    pub(crate) tiles: Tiles,
    pub(crate) buildings: EntitySet<Building>,
    pub(crate) crews: EntitySet<Crew>,
    pub(crate) global_tasks: EntitySet<GlobalTask>,
    pub(crate) power_ratio: f64,
    /// Used power for the last tick, in kW
    pub(crate) used_power: f64,
    pub(crate) global_time: usize,
    pub(crate) transports: EntitySet<Transport>,
    pub(crate) constructions: EntitySet<Construction>,
    /// Ghost conveyors staged for commit. After committing, they will be queued to construction plans
    pub(crate) conveyor_staged: HashMap<Pos, Conveyor>,
    /// Preview of ghost conveyors, just for visualization.
    pub(crate) conveyor_preview: HashMap<Pos, Conveyor>,
    pub(crate) calculate_back_image: Option<CalculateBackImage>,
    pub(crate) rng: Xor128,
}

impl AsteroidColoniesGame {
    pub fn new(calculate_back_image: Option<CalculateBackImage>) -> Result<Self, String> {
        let mut tiles = Tiles::new();
        let r2_thresh = (WIDTH as f64 * 3. / 8.).powi(2);
        let mut rng = Xor128::new(4155235);
        let terms = [
            gen_terms(&mut rng, PERLIN_BITS),
            gen_terms(&mut rng, PERLIN_BITS),
            gen_terms(&mut rng, PERLIN_BITS),
            gen_terms(&mut rng, PERLIN_BITS),
        ];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let r2 = ((x as f64 - WIDTH as f64 / 2.) as f64).powi(2)
                    + ((y as f64 - HEIGHT as f64 / 2.) as f64).powi(2);
                if r2 < r2_thresh {
                    let tile = &mut tiles[[x as i32, y as i32]];
                    *tile = Tile::new_solid(x as i32, y as i32, &terms);
                }
            }
        }
        let start_ofs = |pos: [i32; 2]| {
            [
                pos[0] + 23, //+ WIDTH as i32 / 8,
                pos[1] - 5 + HEIGHT as i32 / 2,
            ]
        };
        let buildings: EntitySet<_> = [
            Building::new(start_ofs([1, 7]), BuildingType::CrewCabin),
            Building::new(start_ofs([3, 4]), BuildingType::Power),
            Building::new(start_ofs([2, 3]), BuildingType::Battery),
            Building::new(start_ofs([3, 3]), BuildingType::Battery),
            Building::new(start_ofs([4, 4]), BuildingType::Excavator),
            Building::new(start_ofs([5, 4]), BuildingType::Storage),
            Building::new_inventory(
                start_ofs([6, 3]),
                BuildingType::MediumStorage,
                btree_map!(ItemType::ConveyorComponent => 20, ItemType::PowerGridComponent => 2)
                    .into(),
            ),
            Building::new(start_ofs([1, 10]), BuildingType::Assembler),
            Building::new(start_ofs([1, 4]), BuildingType::Furnace),
        ]
        .into_iter()
        .collect();
        for building in buildings.iter() {
            let pos = building.pos;
            let size = building.type_.size();
            for iy in 0..size[1] {
                let y = pos[1] as usize + iy;
                for ix in 0..size[0] {
                    let x = pos[0] as usize + ix;
                    tiles[[x as i32, y as i32]] = Tile::building();
                }
            }
        }
        let convs = [
            [3, 5],
            [3, 6],
            [3, 7],
            [3, 8],
            [3, 9],
            [3, 10],
            [4, 10],
            [5, 10],
            [6, 10],
            [6, 9],
            [7, 9],
            [7, 8],
            [7, 7],
            [6, 7],
            [6, 6],
            [6, 5],
            [5, 5],
            [4, 5],
        ];
        for ((pos0, pos1), pos2) in convs
            .iter()
            .zip(convs.iter().skip(1).chain(std::iter::once(&convs[0])))
            .zip(convs.iter().skip(2).chain(convs.iter().take(2)))
        {
            let ofs = start_ofs(*pos1);
            tiles[ofs].state = TileState::Empty;
            let conv = Conveyor::One(
                Direction::from_vec([pos0[0] - pos1[0], pos0[1] - pos1[1]]).unwrap(),
                Direction::from_vec([pos2[0] - pos1[0], pos2[1] - pos1[1]]).unwrap(),
            );
            console_log!("conv {:?}: {:?}", pos1, conv);
            tiles[ofs].conveyor = conv;
            tiles[ofs].power_grid = true;
        }
        for iy in 4..10 {
            for ix in 2..7 {
                let iofs = start_ofs([ix, iy]);
                tiles[iofs].state = TileState::Empty;
            }
        }
        tiles.uniformify();
        if let Some(ref f) = calculate_back_image {
            f(&mut tiles);
        }
        Ok(Self {
            tiles,
            buildings,
            crews: EntitySet::new(),
            global_tasks: EntitySet::new(),
            power_ratio: 1.,
            used_power: 0.,
            global_time: 0,
            transports: EntitySet::new(),
            constructions: EntitySet::new(),
            conveyor_staged: HashMap::new(),
            conveyor_preview: HashMap::new(),
            calculate_back_image,
            rng: Xor128::new(412135),
        })
    }

    pub fn get_global_time(&self) -> usize {
        self.global_time
    }

    /// Get the last power ratio. Used for interpolation of buildings animation.
    ///
    /// TODO: it shouldn't be global, should be per building of power grid.
    pub fn get_power_ratio(&self) -> f64 {
        self.power_ratio
    }

    // pub fn iter_tile(&self) -> impl Iterator<Item = &Tile> {
    //     self.tiles.iter().map(|(_, c)| c)
    // }

    pub fn tiles(&self) -> &Tiles {
        &self.tiles
    }

    pub fn count_tiles(&self) -> usize {
        self.tiles.chunks.len() * CHUNK_SIZE * CHUNK_SIZE
    }

    pub fn tile_at(&self, pos: [i32; 2]) -> &Tile {
        &self.tiles[pos]
    }

    pub fn iter_building(&self) -> impl Iterator<Item = RefOption<Building>> {
        self.buildings.iter()
    }

    pub fn iter_construction(&self) -> impl Iterator<Item = RefOption<Construction>> {
        self.constructions.iter()
    }

    pub fn iter_crew(&self) -> impl Iterator<Item = RefOption<Crew>> {
        self.crews.iter()
    }

    pub fn iter_global_task(&self) -> impl Iterator<Item = RefOption<GlobalTask>> {
        self.global_tasks.iter()
    }

    pub fn num_transports(&self) -> usize {
        self.transports.len()
    }

    pub fn iter_transport(&self) -> impl Iterator<Item = RefOption<Transport>> {
        self.transports.iter()
    }

    pub fn iter_conveyor_plan(&self) -> impl Iterator<Item = (&Pos, &Conveyor)> {
        self.conveyor_staged
            .iter()
            .filter(|(pos, _)| !self.conveyor_preview.contains_key(*pos))
            .chain(self.conveyor_preview.iter())
    }

    pub fn move_building(&mut self, src: Pos, dest: Pos) -> Result<(), String> {
        let Some(building) = self.buildings.iter_mut().find(|b| b.pos == src) else {
            return Err(String::from("Building does not exist at that position"));
        };
        if !building.type_.is_mobile() {
            return Err(String::from("Building at that position is not mobile"));
        }
        if !matches!(building.task, BuildingTask::None) {
            return Err(String::from(
                "The building is busy; wait for the building to finish the current task",
            ));
        }
        let tiles = &self.tiles;
        let buildings = &self.buildings;

        let intersects = |pos: [i32; 2]| {
            buildings.iter().any(|b| {
                let size = b.type_.size();
                b.pos[0] <= pos[0]
                    && pos[0] < size[0] as i32 + b.pos[0]
                    && b.pos[1] <= pos[1]
                    && pos[1] < size[1] as i32 + b.pos[1]
            })
        };

        let mut path = find_path(src, dest, |pos| {
            let tile = &tiles[pos];
            !intersects(pos) && matches!(tile.state, TileState::Empty) && tile.power_grid
        })
        .ok_or_else(|| String::from("Failed to find the path"))?;

        // Re-borrow to avoid borrow checker
        let Some(building) = self.buildings.iter_mut().find(|b| b.pos == src) else {
            return Err(String::from("Building does not exist at that position"));
        };
        path.pop();
        building.task = BuildingTask::Move(MOVE_TIME, path);
        Ok(())
    }

    pub fn move_item(&mut self, from: Pos, to: Pos, item: ItemType) -> Result<(), String> {
        let (src_id, mut src) = self
            .buildings
            .items_borrow_mut()
            .find(|(_, b)| b.intersects(from))
            .ok_or_else(|| "Moving an item needs a building at the source")?;
        send_item(
            &mut self.tiles,
            &mut self.transports,
            &mut *src,
            to,
            &self.buildings,
            &|it| it == item,
        )
        .or_else(|e| {
            let item = *src
                .inventory
                .keys()
                .next()
                .ok_or_else(|| "Moving item source does not have an item")?;
            let crew = if matches!(src.type_, BuildingType::CrewCabin) && 0 < src.crews {
                Crew::new_deliver(src_id, src.pos, to, item, &self.tiles).map(|crew| (crew, src))
            } else {
                self.buildings.items_borrow_mut().find_map(|(from_id, b)| {
                    Crew::new_pickup(from_id, b.pos, from, to, item, &self.tiles)
                        .map(|crew| (crew, b))
                })
            };
            if let Some((crew, mut cabin)) = crew {
                self.crews.insert(crew);
                cabin.crews -= 1;
                Ok(())
            } else {
                Err(format!(
                    "Neither conveyors ({e}) or a crew cannot move the item"
                ))
            }
        })
    }

    pub fn build(&mut self, ix: i32, iy: i32, type_: BuildingType) -> Result<(), String> {
        let size = type_.size();
        for jy in iy..iy + size[1] as i32 {
            for jx in ix..ix + size[0] as i32 {
                let tile = &self.tiles[[jx, jy]];
                if matches!(tile.state, TileState::Solid) {
                    return Err(String::from("Needs excavation before building"));
                }
                if matches!(tile.state, TileState::Space) {
                    return Err(String::from("You cannot build in space!"));
                }
            }
        }

        if let Some((id, _)) = self
            .buildings
            .items()
            .find(|(_, b)| b.intersects_rect([ix, iy], size))
        {
            return Err(format!(
                "The destination is already occupied by a building {id}",
            ));
        }

        if self
            .constructions
            .iter()
            .any(|c| c.intersects_rect([ix, iy], size))
        {
            return Err(String::from(
                "The destination is already occupied by a construction plan",
            ));
        }

        if let Some(build) = get_build_menu()
            .iter()
            .find(|it| it.type_ == ConstructionType::Building(type_))
        {
            self.constructions
                .insert(Construction::new(build, [ix, iy]));
            // self.build_building(ix, iy, type_)?;
        }
        Ok(())
    }

    pub fn build_plan(&mut self, constructions: &[Construction]) {
        for c in constructions {
            self.constructions.insert(c.clone());
        }
    }

    pub fn cancel_build(&mut self, ix: i32, iy: i32) {
        if let Some(c) = self.constructions.iter_mut().find(|c| c.pos == [ix, iy]) {
            c.toggle_cancel();
        }
    }

    pub fn deconstruct(&mut self, ix: i32, iy: i32) -> Result<(), String> {
        let (id, b) = self
            .buildings
            .items_mut()
            .find(|(_, b)| b.pos == [ix, iy])
            .ok_or_else(|| String::from("Building not found at given position"))?;
        let decon = Construction::new_deconstruct(b.type_, [ix, iy], &b.inventory)
            .ok_or_else(|| String::from("No build recipe was found to deconstruct"))?;
        self.constructions.insert(decon);

        self.buildings.remove(id);

        Ok(())
    }

    pub fn deconstruct_conveyor(&mut self, ix: i32, iy: i32) -> Result<(), &'static str> {
        let tile = self
            .tiles
            .try_get_mut([ix, iy])
            .ok_or("Tile does not exist")?;
        if matches!(tile.conveyor, Conveyor::None) {
            return Err("Conveyor does not exist");
        }
        let decon = Construction::new_conveyor([ix, iy], tile.conveyor, true);
        tile.conveyor = Conveyor::None;
        self.constructions.insert(decon);
        Ok(())
    }

    pub fn deconstruct_power_grid(&mut self, ix: i32, iy: i32) -> Result<(), &'static str> {
        let tile = self
            .tiles
            .try_get_mut([ix, iy])
            .ok_or("Tile does not exist")?;
        if !tile.power_grid {
            return Err("Power grid does not exist");
        }
        let decon = Construction::new_power_grid([ix, iy], true);
        tile.power_grid = false;
        self.constructions.insert(decon);
        Ok(())
    }

    pub fn get_recipes(&self, ix: i32, iy: i32) -> Result<Vec<&'static Recipe>, String> {
        if ix < 0 || WIDTH as i32 <= ix || iy < 0 || HEIGHT as i32 <= iy {
            return Err(String::from("Point outside tile"));
        }

        let Some(assembler) = self.buildings.iter().find(|b| b.intersects([ix, iy])) else {
            return Err(String::from("The building does not exist at the target"));
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(String::from("The building is not an assembler"));
        }
        Ok(recipes().iter().collect::<Vec<_>>())
    }

    pub fn set_recipe(&mut self, ix: i32, iy: i32, name: Option<&str>) -> Result<(), String> {
        let Some(assembler) = self.buildings.iter_mut().find(|b| b.intersects([ix, iy])) else {
            return Err("The building does not exist at the target".to_string());
        };
        if !matches!(assembler.type_, BuildingType::Assembler) {
            return Err(String::from("The building is not an assembler"));
        }
        let Some(name) = name else {
            assembler.set_recipe(None)?;
            return Ok(());
        };
        for recipe in recipes() {
            let Some((key, _)) = recipe.outputs.iter().next() else {
                continue;
            };
            if format!("{:?}", key) == name {
                assembler.set_recipe(Some(recipe))?;
                break;
            }
        }
        Ok(())
    }

    pub fn cleanup_item(&mut self, pos: Pos) -> Result<(), String> {
        self.global_tasks.insert(GlobalTask::Cleanup(pos));
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), String> {
        self.process_global_tasks();
        self.process_transports();
        self.process_constructions();
        self.process_buildings();
        self.process_crews();

        self.global_time += 1;

        Ok(())
    }

    pub fn uniformify_tiles(&mut self) {
        self.tiles.uniformify();
    }

    pub fn serialize(&self, pretty: bool) -> serde_json::Result<String> {
        let ser_game = SerializeGame::from(self);
        if pretty {
            serde_json::to_string_pretty(&ser_game)
        } else {
            serde_json::to_string(&ser_game)
        }
    }

    pub fn serialize_bin(&self) -> Result<Vec<u8>, String> {
        let ser_game = SerializeGame::from(self);
        bincode::serialize(&ser_game).map_err(|e| format!("{e}"))
    }

    pub fn deserialize(&mut self, rdr: impl Read) -> serde_json::Result<()> {
        let ser_data: SerializeGame = serde_json::from_reader(rdr)?;
        self.from_serialized(ser_data);
        Ok(())
    }

    pub fn deserialize_bin(&mut self, rdr: &[u8]) -> Result<(), String> {
        let ser_data: SerializeGame = bincode::deserialize(rdr).map_err(|e| format!("{e}"))?;
        self.from_serialized(ser_data);
        Ok(())
    }

    fn from_serialized(&mut self, ser_data: SerializeGame) {
        for (pos, chunk) in ser_data.tiles.chunks {
            self.tiles.chunks.insert(pos, chunk);
        }
        self.buildings = ser_data.buildings;
        self.crews = ser_data.crews;
        self.global_tasks = ser_data.global_tasks;
        self.global_time = ser_data.global_time;
        self.transports = ser_data.transports;
        self.constructions = ser_data.constructions;
        self.rng = ser_data.rng;

        // Clear transports expectation cache
        for building in self.buildings.iter_mut() {
            building.expected_transports.clear();
        }

        // Clear transports expectation cache
        for construction in self.constructions.iter_mut() {
            construction.clear_expected_all();
        }

        // Reconstruct transports expectation cache from actual data
        for (id, t) in self.transports.items() {
            let Some(t_pos) = t.path.last() else {
                continue;
            };
            if let Some(building) = self.buildings.iter_mut().find(|b| b.intersects(*t_pos)) {
                building.expected_transports.insert(id);
            } else if let Some(construction) =
                self.constructions.iter_mut().find(|c| c.intersects(*t_pos))
            {
                construction.insert_expected_transports(id);
            }
        }

        if let Some(ref f) = self.calculate_back_image {
            f(&mut self.tiles);
        }
    }

    pub fn serialize_chunks_digest(&self) -> bincode::Result<Vec<u8>> {
        let digests = self
            .tiles
            .chunks()
            .iter()
            .map(|(pos, chunk)| (pos, chunk.get_hash()))
            .collect::<HashMap<_, _>>();
        bincode::serialize(&digests)
    }

    pub fn serialize_with_diffs(
        &self,
        chunks_digest: &HashMap<Position, u64>,
    ) -> Result<Vec<u8>, String> {
        let tiles = self.tiles.filter_with_diffs(chunks_digest)?;
        let ser_game = SerializeGame {
            tiles,
            buildings: self.buildings.clone(),
            crews: self.crews.clone(),
            global_tasks: self.global_tasks.clone(),
            global_time: self.global_time,
            transports: self.transports.clone(),
            constructions: self.constructions.clone(),
            rng: self.rng.clone(),
        };
        bincode::serialize(&ser_game).map_err(|e| format!("{e}"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializeGame {
    tiles: Tiles,
    buildings: EntitySet<Building>,
    crews: EntitySet<Crew>,
    global_tasks: EntitySet<GlobalTask>,
    global_time: usize,
    transports: EntitySet<Transport>,
    constructions: EntitySet<Construction>,
    rng: Xor128,
}

impl From<&AsteroidColoniesGame> for SerializeGame {
    fn from(value: &AsteroidColoniesGame) -> Self {
        Self {
            tiles: value.tiles.clone(),
            buildings: value.buildings.clone(),
            crews: value.crews.clone(),
            global_tasks: value.global_tasks.clone(),
            global_time: value.global_time,
            transports: value.transports.clone(),
            constructions: value.constructions.clone(),
            rng: value.rng.clone(),
        }
    }
}
