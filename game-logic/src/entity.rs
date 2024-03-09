use std::{fmt::Display, ops::Index};

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
/// An entry in entity list with generational ids, with the payload and the generation
pub struct EntityEntry<T> {
    pub gen: u32,
    pub payload: Option<T>,
}

impl<T> EntityEntry<T> {
    pub(crate) fn new(payload: T) -> Self {
        Self {
            gen: 0,
            payload: Some(payload),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EntitySet<T> {
    v: Vec<EntityEntry<T>>,
}

impl<T> EntitySet<T> {
    pub fn new() -> Self {
        Self { v: vec![] }
    }

    pub fn len(&self) -> usize {
        // TODO: optimize by caching active elements
        self.v
            .iter()
            .filter(|entry| entry.payload.is_some())
            .count()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.v.iter().filter_map(|v| v.payload.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.v.iter_mut().filter_map(|v| v.payload.as_mut())
    }

    pub fn items(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.v
            .iter()
            .enumerate()
            .filter_map(|(i, v)| Some((EntityId::new(i as u32, v.gen), v.payload.as_ref()?)))
    }

    pub fn items_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.v
            .iter_mut()
            .enumerate()
            .filter_map(|(i, v)| Some((EntityId::new(i as u32, v.gen), v.payload.as_mut()?)))
    }

    pub fn split_mid(&self, idx: usize) -> Option<(&T, &[EntityEntry<T>], &[EntityEntry<T>])> {
        if self.v.len() <= idx {
            return None;
        }
        let (first, mid) = self.v.split_at(idx);
        let (center, last) = mid.split_first()?;
        Some((center.payload.as_ref()?, first, last))
    }

    pub fn split_mid_mut(
        &mut self,
        idx: usize,
    ) -> Option<(&mut T, &mut [EntityEntry<T>], &mut [EntityEntry<T>])> {
        if self.v.len() <= idx {
            return None;
        }
        let (first, mid) = self.v.split_at_mut(idx);
        let (center, last) = mid.split_first_mut()?;
        Some((center.payload.as_mut()?, first, last))
    }

    pub fn insert(&mut self, val: T) -> EntityId {
        for (i, entry) in self.v.iter_mut().enumerate() {
            if entry.payload.is_none() {
                entry.gen += 1;
                entry.payload = Some(val);
                return EntityId::new(i as u32, entry.gen);
            }
        }
        self.v.push(EntityEntry::new(val));
        EntityId::new(self.v.len() as u32 - 1, 0)
    }

    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        self.v.get_mut(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                entry.payload.take()
            } else {
                None
            }
        })
    }

    pub fn retain(&mut self, f: impl Fn(&T) -> bool) {
        for entry in &mut self.v {
            let Some(payload) = entry.payload.as_mut() else {
                continue;
            };
            if !f(payload) {
                entry.payload = None;
            }
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&T> {
        self.v.get(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                entry.payload.as_ref()
            } else {
                None
            }
        })
    }
}

impl<T> AsMut<Vec<EntityEntry<T>>> for EntitySet<T> {
    fn as_mut(&mut self) -> &mut Vec<EntityEntry<T>> {
        &mut self.v
    }
}

/// Index operator. You should prefer `get()` since it will panic if the entity is destroyed
impl<T> Index<EntityId> for EntitySet<T> {
    type Output = T;
    fn index(&self, index: EntityId) -> &Self::Output {
        &self.v[index.id as usize].payload.as_ref().unwrap()
    }
}

/// An inefficient implementation of IntoIterator.
/// TODO: remove Box
impl<'a, T> IntoIterator for &'a EntitySet<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = &'a T> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.v.iter().filter_map(|v| v.payload.as_ref()))
    }
}

impl<A> FromIterator<A> for EntitySet<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let v = iter.into_iter().map(EntityEntry::new).collect();
        Self { v }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId {
    id: u32,
    gen: u32,
}

impl EntityId {
    fn new(id: u32, gen: u32) -> Self {
        Self { id, gen }
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.id, self.gen)
    }
}

/// An extension trait to allow a container to iterate over valid items
pub trait EntityIterExt<T> {
    /// Iterate items in each entry's payload
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;
}

/// An extension trait to allow a container to mutably iterate over valid items
pub trait EntityIterMutExt<T>: EntityIterExt<T> {
    /// Mutably iterate items in each entry's payload
    fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a;
}

impl<T> EntityIterExt<T> for [EntityEntry<T>] {
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.iter().filter_map(|b| b.payload.as_ref())
    }
}

impl<T> EntityIterMutExt<T> for [EntityEntry<T>] {
    fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        self.iter_mut().filter_map(|b| b.payload.as_mut())
    }
}

impl<T> EntityIterExt<T> for Vec<EntityEntry<T>> {
    fn items<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.iter().filter_map(|b| b.payload.as_ref())
    }
}

impl<T> EntityIterMutExt<T> for Vec<EntityEntry<T>> {
    fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        self.iter_mut().filter_map(|b| b.payload.as_mut())
    }
}
