use super::*;
use crate::{BuildingType, Inventory};

struct MockTiles;

impl TileSampler for MockTiles {
    fn at(&self, pos: [i32; 2]) -> Option<&Cell> {
        use {Conveyor::*, Direction::*};
        static SOLID: Cell = Cell::new();
        static RD: Cell = Cell::new_with_conveyor(One(Right, Down));
        static UD: Cell = Cell::new_with_conveyor(One(Up, Down));
        static UR: Cell = Cell::new_with_conveyor(One(Up, Right));
        static LR: Cell = Cell::new_with_conveyor(One(Left, Right));
        static LU: Cell = Cell::new_with_conveyor(One(Left, Up));
        static DU: Cell = Cell::new_with_conveyor(One(Down, Up));
        static DL: Cell = Cell::new_with_conveyor(One(Down, Left));
        static RL: Cell = Cell::new_with_conveyor(One(Right, Left));
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
    let mut inputs = HashMap::new();
    inputs.insert(ItemType::RawOre, 1);

    let mut storage = [Building::new_inventory(
        [1, -1],
        BuildingType::Storage,
        inputs.clone(),
    )];

    let mut transports = vec![];

    pull_inputs(
        &inputs,
        &MockTiles,
        &mut transports,
        [1, 3],
        [1, 1],
        &mut HashMap::new(),
        &mut storage,
        &mut [],
    );

    assert_eq!(
        transports,
        vec![Transport {
            src: [1, -1],
            dest: [1, 3],
            item: ItemType::RawOre,
            amount: 1,
            path: vec![[1, 3], [1, 2], [0, 2], [0, 1], [0, 0], [1, 0], [1, -1]],
        }]
    )
}

struct MockInventory(Inventory);

impl HasInventory for MockInventory {
    fn pos(&self) -> Pos {
        [1, 3]
    }

    fn size(&self) -> [usize; 2] {
        [1, 1]
    }

    fn inventory(&mut self) -> &mut Inventory {
        &mut self.0
    }
}

#[test]
fn test_push_outputs() {
    let mut storage = [Building::new([1, -1], BuildingType::Storage)];

    let mut transports = vec![];

    let mut outputs = Inventory::new();
    outputs.insert(ItemType::RawOre, 1);

    let mut mock_inventory = MockInventory(outputs);

    push_outputs(
        &MockTiles,
        &mut transports,
        &mut mock_inventory,
        &mut storage,
        &mut [],
        &|_| true,
    );

    assert_eq!(
        transports,
        vec![Transport {
            src: [1, 3],
            dest: [1, -1],
            item: ItemType::RawOre,
            amount: 1,
            path: vec![[1, -1], [1, 0], [2, 0], [2, 1], [2, 2], [1, 2], [1, 3]],
        }]
    )
}
