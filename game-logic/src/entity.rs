use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Display,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
/// An entry in entity list with generational ids, with the payload and the generation
pub struct EntityEntry<T> {
    pub gen: u32,
    pub payload: RefCell<Option<T>>,
}

impl<T> Default for EntityEntry<T> {
    fn default() -> Self {
        Self {
            gen: 0,
            payload: RefCell::new(None),
        }
    }
}

impl<T> EntityEntry<T> {
    pub(crate) fn new(payload: T) -> Self {
        Self {
            gen: 0,
            payload: RefCell::new(Some(payload)),
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

    /// Returns the number of active elements in this EntitySet.
    /// It does _not_ return the buffer length.
    pub fn len(&self) -> usize {
        // TODO: optimize by caching active elements
        self.v
            .iter()
            .filter(|entry| entry.payload.borrow().is_some())
            .count()
    }

    /// Return an iterator over Ref<T>.
    /// It borrows the T immutably.
    pub fn iter(&self) -> impl Iterator<Item = RefOption<T>> {
        self.v.iter().filter_map(|v| RefOption::new(&v.payload))
    }

    /// Return an iterator over &mut T. It does not borrow the T with a RefMut,
    /// because the self is already exclusively referenced.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.v
            .iter_mut()
            .filter_map(|v| v.payload.get_mut().as_mut())
    }

    /// Return an iterator over RefMut<T>, skipping already borrowed items.
    /// It borrows the T mutablly.
    pub fn iter_borrow_mut(&self) -> impl Iterator<Item = RefMutOption<T>> {
        self.v.iter().filter_map(|v| RefMutOption::new(&v.payload))
    }

    /// Return an iterator over (id, Ref<T>)
    /// It is convenient when you want the EntityId of the iterated items.
    /// It borrows the T immutably.
    pub fn items(&self) -> impl Iterator<Item = (EntityId, RefOption<T>)> {
        self.v.iter().enumerate().filter_map(|(i, v)| {
            Some((EntityId::new(i as u32, v.gen), RefOption::new(&v.payload)?))
        })
    }

    /// Return an iterator over (id, &mut T).
    /// It is convenient when you want the EntityId of the iterated items.
    /// It does not borrow the T with a RefMut, because the self is already exclusively referenced.
    pub fn items_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.v.iter_mut().enumerate().filter_map(|(i, v)| {
            Some((
                EntityId::new(i as u32, v.gen),
                v.payload.get_mut().as_mut()?,
            ))
        })
    }

    /// Return an iterator over (id, RefMut<T>), skipping already borrowed items.
    /// It is convenient when you want the EntityId of the iterated items.
    /// It borrows the T mutablly.
    pub fn items_borrow_mut(&self) -> impl Iterator<Item = (EntityId, RefMutOption<T>)> {
        self.v.iter().enumerate().filter_map(|(i, v)| {
            Some((
                EntityId::new(i as u32, v.gen),
                RefMutOption::new(&v.payload)?,
            ))
        })
    }

    // pub fn split_mid(&self, idx: usize) -> Option<(Ref<T>, &[EntityEntry<T>], &[EntityEntry<T>])> {
    //     if self.v.len() <= idx {
    //         return None;
    //     }
    //     let (first, mid) = self.v.split_at(idx);
    //     let (center, last) = mid.split_first()?;
    //     Some((center.payload.as_ref()?.try_borrow().ok()?, first, last))
    // }

    // pub fn split_mid_mut(&mut self, idx: usize) -> Option<(&mut T, EntitySliceMut<T>)> {
    //     if self.v.len() <= idx {
    //         return None;
    //     }
    //     let (first, mid) = self.v.split_at_mut(idx);
    //     let (center, last) = mid.split_first_mut()?;
    //     Some((center.payload.as_mut()?.get_mut(), EntitySliceMut([first, last])))
    // }

    pub fn insert(&mut self, val: T) -> EntityId {
        for (i, entry) in self.v.iter_mut().enumerate() {
            let payload = entry.payload.get_mut();
            if payload.is_none() {
                entry.gen += 1;
                entry.payload = RefCell::new(Some(val));
                return EntityId::new(i as u32, entry.gen);
            }
        }
        self.v.push(EntityEntry::new(val));
        EntityId::new(self.v.len() as u32 - 1, 0)
    }

    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        self.v.get_mut(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                entry.payload.get_mut().take()
            } else {
                None
            }
        })
    }

    // Will we need removal of an element through a shared reference? If so, we need `RefCell<Option<T>>`
    // instead of `Option<RefCell<T>>`.
    // pub fn borrow_remove(&self, id: EntityId) -> Option<T> {
    //     self.v.get(id.id as usize).and_then(|entry| {
    //         if id.gen == entry.gen {
    //             entry.payload.borrow_mut().take().map(|v| v.into_inner())
    //         } else {
    //             None
    //         }
    //     })
    // }

    pub fn retain(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        for entry in &mut self.v {
            let Some(payload) = entry.payload.get_mut().as_mut() else {
                continue;
            };
            if !f(payload) {
                entry.payload = RefCell::new(None);
            }
        }
    }

    pub fn retain_borrow_mut(&self, mut f: impl FnMut(&mut T, EntityId) -> bool) {
        for (id, entry) in self.v.iter().enumerate() {
            let Ok(mut payload) = entry.payload.try_borrow_mut() else {
                continue;
            };
            if payload.is_none() {
                continue;
            }
            if !f(
                payload.as_mut().unwrap(),
                EntityId::new(id as u32, entry.gen),
            ) {
                *payload = None;
            }
        }
    }

    pub fn get(&self, id: EntityId) -> Option<RefOption<T>> {
        self.v.get(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                RefOption::new(&entry.payload)
            } else {
                None
            }
        })
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        self.v.get_mut(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                entry.payload.get_mut().as_mut()
            } else {
                None
            }
        })
    }

    /// Get without generation check
    pub fn get_mut_at(&mut self, idx: usize) -> Option<&mut T> {
        self.v
            .get_mut(idx)
            .and_then(|entry| entry.payload.get_mut().as_mut())
    }

    pub fn borrow_mut_at(&self, idx: usize) -> Option<RefMutOption<T>> {
        self.v
            .get(idx)
            .and_then(|entry| RefMutOption::new(&entry.payload))
    }

    pub fn update_with_id(&mut self, id: EntityId, val: T)
    where
        T: Clone,
    {
        let new_id = id.id as usize;
        if self.v.len() <= new_id {
            self.v.resize(new_id, EntityEntry::default());
        }
        if let Some(entry) = self.v.get_mut(new_id) {
            entry.gen = id.gen;
            *entry.payload.get_mut() = Some(val);
        }
    }
}

#[derive(Debug)]
/// A wrapper around Ref<Option> that always has Some.
/// We need a Ref to release the refcounter, but we would never return
/// a Ref(None).
pub struct RefOption<'a, T>(Ref<'a, Option<T>>);

impl<'a, T> RefOption<'a, T> {
    fn new(val: &'a RefCell<Option<T>>) -> Option<Self> {
        let v = val.try_borrow().ok()?;
        if v.is_none() {
            return None;
        }
        Some(Self(v))
    }
}

impl<'a, T> Deref for RefOption<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

/// A wrapper around RefMut<Option> that always has Some.
/// We need a RefMut to release the refcounter, but we would never return
/// a RefMut(None).
pub struct RefMutOption<'a, T>(RefMut<'a, Option<T>>);

impl<'a, T> RefMutOption<'a, T> {
    fn new(val: &'a RefCell<Option<T>>) -> Option<Self> {
        let v = val.try_borrow_mut().ok()?;
        if v.is_none() {
            return None;
        }
        Some(Self(v))
    }
}

impl<'a, T> Deref for RefMutOption<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl<'a, T> DerefMut for RefMutOption<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

// pub struct EntitySlice<'a, T>([&'a [EntityEntry<T>]; 2]);

// impl<'a, T> EntitySlice<'a, T> {
//     pub fn iter(&self) -> impl Iterator<Item = &'a T> {
//         self.0[0]
//             .iter()
//             .chain(self.0[1].iter())
//             .filter_map(|e| e.payload.as_ref())
//     }
// }

// pub struct EntitySliceMut<'a, T>([&'a mut [EntityEntry<T>]; 2]);

// impl<'a, T> EntitySliceMut<'a, T> {
//     pub fn iter_mut<'b, 'c>(&'b mut self) -> impl Iterator<Item = &'a mut T> + 'c
//     where
//         'a: 'b,
//         'b: 'c,
//     {
//         self.0
//             .iter_mut()
//             .map(|v| v.iter_mut())
//             .flatten()
//             .filter_map(move |e| e.payload.as_mut())
//     }
// }

// impl<'a, T> From<EntitySliceMut<'a, T>> for EntitySlice<'a, T> {
//     fn from(value: EntitySliceMut<'a, T>) -> Self {
//         Self([value.0[0], value.0[1]])
//     }
// }

// impl<'a, 'b, T> From<&'b EntitySet<T>> for EntitySlice<'a, T> where 'b: 'a {
//     fn from(value: &'b EntitySet<T>) -> Self {
//         Self([&value.v, &[]])
//     }
// }

// impl<'a, 'b, T> From<&'b mut EntitySet<T>> for EntitySliceMut<'a, T> where 'b: 'a {
//     fn from(value: &'b mut EntitySet<T>) -> Self {
//         Self([&mut value.v, &mut []])
//     }
// }

impl<T> AsMut<Vec<EntityEntry<T>>> for EntitySet<T> {
    fn as_mut(&mut self) -> &mut Vec<EntityEntry<T>> {
        &mut self.v
    }
}

/// Index operator. You should prefer `get()` since it will panic if the entity is destroyed
// impl<'a, T> Index<EntityId> for &'a EntitySet<T> {
//     type Output = Ref<'a, T>;
//     fn index(&self, index: EntityId) -> &Self::Output {
//         &self.v[index.id as usize].payload.as_ref().unwrap().borrow()
//     }
// }

/// An inefficient implementation of IntoIterator.
/// TODO: remove Box
impl<'a, T> IntoIterator for &'a EntitySet<T> {
    type Item = RefOption<'a, T>;
    type IntoIter = Box<dyn Iterator<Item = RefOption<'a, T>> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.v.iter().filter_map(|v| RefOption::new(&v.payload)))
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
    fn items<'a>(&'a self) -> impl Iterator<Item = RefOption<'a, T>>
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
    fn items<'a>(&'a self) -> impl Iterator<Item = RefOption<'a, T>>
    where
        T: 'a,
    {
        self.iter().filter_map(|b| RefOption::new(&b.payload))
    }
}

impl<T> EntityIterMutExt<T> for [EntityEntry<T>] {
    fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        self.iter_mut().filter_map(|b| b.payload.get_mut().as_mut())
    }
}

impl<T> EntityIterExt<T> for Vec<EntityEntry<T>> {
    fn items<'a>(&'a self) -> impl Iterator<Item = RefOption<'a, T>>
    where
        T: 'a,
    {
        self.iter().filter_map(|b| RefOption::new(&b.payload))
    }
}

impl<T> EntityIterMutExt<T> for Vec<EntityEntry<T>> {
    fn items_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T>
    where
        T: 'a,
    {
        self.iter_mut().filter_map(|b| b.payload.get_mut().as_mut())
    }
}
