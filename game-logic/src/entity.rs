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

    pub fn items(&self) -> impl Iterator<Item = (usize, &T)> {
        self.v
            .iter()
            .enumerate()
            .filter_map(|(i, v)| Some((i, v.payload.as_ref()?)))
    }

    pub fn insert(&mut self, val: T) -> usize {
        for (i, entry) in self.v.iter_mut().enumerate() {
            if entry.payload.is_none() {
                entry.payload = Some(val);
                return i;
            }
        }
        self.v.push(EntityEntry::new(val));
        self.v.len() - 1
    }

    pub fn remove(&mut self, idx: usize) -> Option<T> {
        self.v.get_mut(idx).and_then(|entry| entry.payload.take())
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
