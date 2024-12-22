use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{building::OreAccum, ItemType};

pub type CountableInventory = BTreeMap<ItemType, usize>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Inventory {
    /// A container for countable items
    countable: CountableInventory,
    ores: OreAccum,
}

impl<const N: usize> From<[(ItemType, usize); N]> for Inventory {
    fn from(value: [(ItemType, usize); N]) -> Self {
        Self {
            countable: BTreeMap::from(value),
            ores: OreAccum::default(),
        }
    }
}

impl From<CountableInventory> for Inventory {
    fn from(value: CountableInventory) -> Self {
        Self {
            countable: value,
            ores: OreAccum::default(),
        }
    }
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = (&'a ItemType, &'a usize);
    type IntoIter = std::collections::btree_map::Iter<'a, ItemType, usize>;
    fn into_iter(self) -> Self::IntoIter {
        self.countable.iter()
    }
}

impl FromIterator<(ItemType, usize)> for Inventory {
    fn from_iter<T: IntoIterator<Item = (ItemType, usize)>>(iter: T) -> Self {
        Self {
            countable: iter.into_iter().collect(),
            ores: OreAccum::default(),
        }
    }
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            countable: BTreeMap::new(),
            ores: OreAccum::default(),
        }
    }

    pub fn countable(&self) -> &CountableInventory {
        &self.countable
    }

    pub fn countable_size(&self) -> usize {
        self.countable.iter().map(|(_, v)| *v).sum()
    }

    pub fn get(&self, ty: &ItemType) -> usize {
        *self.countable.get(ty).unwrap_or(&0)
    }

    pub fn get_mut(&mut self, ty: &ItemType) -> Option<&mut usize> {
        self.countable.get_mut(ty)
    }

    pub fn entry(&mut self, ty: ItemType) -> std::collections::btree_map::Entry<ItemType, usize> {
        self.countable.entry(ty)
    }

    pub fn remove(&mut self, ty: &ItemType) -> Option<usize> {
        self.countable.remove(ty)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ItemType, &usize)> {
        self.countable.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ItemType, &mut usize)> {
        self.countable.iter_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.countable.is_empty()
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<ItemType, usize> {
        self.countable.keys()
    }

    pub fn insert(&mut self, key: ItemType, value: usize) -> Option<usize> {
        self.countable.insert(key, value)
    }
}
