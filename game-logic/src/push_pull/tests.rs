use super::*;
use crate::{btree_map, building::BuildingType, items::Inventory};

struct MockTiles;

impl TileSampler for MockTiles {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile> {
        use {Conveyor::*, Direction::*};
        static SOLID: Tile = Tile::new();
        static RD: Tile = Tile::new_with_conveyor(One(Right, Down));
        static UD: Tile = Tile::new_with_conveyor(One(Up, Down));
        static UR: Tile = Tile::new_with_conveyor(One(Up, Right));
        static LR: Tile = Tile::new_with_conveyor(One(Left, Right));
        static LU: Tile = Tile::new_with_conveyor(One(Left, Up));
        static DU: Tile = Tile::new_with_conveyor(One(Down, Up));
        static DL: Tile = Tile::new_with_conveyor(One(Down, Left));
        static RL: Tile = Tile::new_with_conveyor(One(Right, Left));
        let ret = match pos {
            [0, 0] => Some(&RD),
            [0, 1] => Some(&UD),
            [0, 2] => Some(&UR),
            [1, 2] => Some(&LR),
            [2, 2] => Some(&LU),
            [2, 1] => Some(&DU),
            [2, 0] => Some(&DL),
            [1, 0] => Some(&RL),
            _ => Some(&SOLID),
        };
        ret
    }
}

#[test]
fn test_pull_inputs() {
    let inputs = btree_map!(ItemType::RawOre => 1);

    let storage: EntitySet<_> = [Building::new_inventory(
        [1, -1],
        BuildingType::Storage,
        inputs.clone(),
    )]
    .into_iter()
    .collect();

    let mut transports = EntitySet::new();

    pull_inputs(
        &inputs,
        &MockTiles,
        &mut transports,
        &mut HashSet::new(),
        [1, 3],
        [1, 1],
        &mut Inventory::new(),
        &storage,
    );

    let mut expected = EntitySet::new();
    expected.insert(Transport {
        src: [1, -1],
        dest: [1, 3],
        item: ItemType::RawOre,
        amount: 1,
        path: vec![[1, 3], [1, 2], [0, 2], [0, 1], [0, 0], [1, 0], [1, -1]],
    });

    assert_eq!(transports, expected)
}

struct MockInventory(Pos, Inventory);

impl HasInventory for MockInventory {
    fn pos(&self) -> Pos {
        self.0
    }

    fn size(&self) -> [usize; 2] {
        [1, 1]
    }

    fn inventory(&mut self) -> &mut Inventory {
        &mut self.1
    }
}

#[test]
fn test_push_outputs() {
    let storage: EntitySet<_> = [Building::new([1, -1], BuildingType::Storage)]
        .into_iter()
        .collect();

    let mut transports = EntitySet::new();

    let mut outputs = Inventory::new();
    outputs.insert(ItemType::RawOre, 1);

    let mut mock_inventory = MockInventory([1, 3], outputs);

    push_outputs(
        &MockTiles,
        &mut transports,
        &mut mock_inventory,
        &storage,
        &|_| true,
    );

    let mut expected = EntitySet::new();
    expected.insert(Transport {
        src: [1, 3],
        dest: [1, -1],
        item: ItemType::RawOre,
        amount: 1,
        path: vec![[1, -1], [1, 0], [2, 0], [2, 1], [2, 2], [1, 2], [1, 3]],
    });

    assert_eq!(transports, expected)
}

struct MockTiles2;

impl TileSampler for MockTiles2 {
    fn at(&self, pos: [i32; 2]) -> Option<&Tile> {
        use {Conveyor::*, Direction::*};
        static SOLID: Tile = Tile::new();
        static RD: Tile = Tile::new_with_conveyor(One(Right, Down));
        static UD: Tile = Tile::new_with_conveyor(One(Up, Down));
        static UR: Tile = Tile::new_with_conveyor(One(Up, Right));
        static LR: Tile = Tile::new_with_conveyor(One(Left, Right));
        static LU: Tile = Tile::new_with_conveyor(One(Left, Up));
        static DU: Tile = Tile::new_with_conveyor(One(Down, Up));
        static DL: Tile = Tile::new_with_conveyor(One(Down, Left));
        static RL: Tile = Tile::new_with_conveyor(One(Right, Left));
        static DULR: Tile = Tile::new_with_conveyor(Two((Down, Up), (Left, Right)));
        let ret = match pos {
            [0, 0] => Some(&RD),
            [0, 1] => Some(&UD),
            [0, 2] => Some(&UD),
            [0, 3] => Some(&UR),
            [1, 3] => Some(&LR),
            [2, 3] => Some(&LU),
            [2, 2] => Some(&DULR),
            [2, 1] => Some(&DL),
            [1, 1] => Some(&RD),
            [1, 2] => Some(&UR),
            // [2, 2] => Some(&LR),
            [3, 2] => Some(&LU),
            [3, 1] => Some(&DU),
            [3, 0] => Some(&DL),
            [2, 0] => Some(&RL),
            [1, 0] => Some(&RL),
            _ => Some(&SOLID),
        };
        ret
    }
}

fn print_board(tiles: &impl TileSampler) {
    use Direction::*;
    let vert_bar = |t: &Tile, c| {
        if t.conveyor.has_from(c) || t.conveyor.has_to(c) {
            "|"
        } else {
            " "
        }
    };
    let horz_bar = |t: &Tile, c| {
        if t.conveyor.has_from(c) || t.conveyor.has_to(c) {
            "-"
        } else {
            " "
        }
    };
    print!("   ");
    for ix in 0..5 {
        print!("{:2} ", ix);
    }
    for iy in 0..5 {
        print!("   ");
        for ix in 0..5 {
            print!(
                " {} ",
                tiles.at([ix, iy]).map(|t| vert_bar(t, Up)).unwrap_or(" ")
            );
        }
        println!("");
        print!("{:2} ", iy);
        for ix in 0..5 {
            if let Some(t) = tiles.at([ix, iy]) {
                print!("{}{}{}", horz_bar(t, Left), "+", horz_bar(t, Right));
            } else {
                print!("   ");
            }
        }
        println!("");
        print!("   ");
        for ix in 0..5 {
            print!(
                " {} ",
                tiles.at([ix, iy]).map(|t| vert_bar(t, Down)).unwrap_or(" ")
            );
        }
        println!("");
    }
}

#[test]
fn test_pull_inputs2() {
    let inputs = btree_map!(ItemType::RawOre => 1);

    let storage = [Building::new_inventory(
        [1, -1],
        BuildingType::Storage,
        inputs.clone(),
    )]
    .into_iter()
    .collect();

    let mut transports = EntitySet::new();

    pull_inputs(
        &inputs,
        &MockTiles2,
        &mut transports,
        &mut HashSet::new(),
        [1, 4],
        [1, 1],
        &mut Inventory::new(),
        &storage,
    );

    let mut expected = EntitySet::new();
    expected.insert(Transport {
        src: [1, -1],
        dest: [1, 4],
        item: ItemType::RawOre,
        amount: 1,
        path: vec![
            [1, 4],
            [1, 3],
            [0, 3],
            [0, 2],
            [0, 1],
            [0, 0],
            [1, 0],
            [1, -1],
        ],
    });

    assert_eq!(transports, expected)
}

#[test]
fn test_push_outputs2() {
    let storage: EntitySet<_> = [Building::new([1, -1], BuildingType::Storage)]
        .into_iter()
        .collect();

    let mut transports = EntitySet::new();

    let mut outputs = Inventory::new();
    outputs.insert(ItemType::RawOre, 1);

    let mut mock_inventory = MockInventory([1, 4], outputs);

    print_board(&MockTiles2);

    push_outputs(
        &MockTiles2,
        &mut transports,
        &mut mock_inventory,
        &storage,
        &|_| true,
    );

    let mut expected = EntitySet::new();
    expected.insert(Transport {
        src: [1, 4],
        dest: [1, -1],
        item: ItemType::RawOre,
        amount: 1,
        path: vec![
            [1, -1],
            [1, 0],
            [2, 0],
            [3, 0],
            [3, 1],
            [3, 2],
            [2, 2],
            [1, 2],
            [1, 1],
            [2, 1],
            [2, 2],
            [2, 3],
            [1, 3],
            [1, 4],
        ],
    });

    assert_eq!(transports, expected)
}
