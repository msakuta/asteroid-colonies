mod entry_payload;
mod ref_option;

use std::{
    cell::{Cell, RefCell},
    fmt::Display,
    marker::PhantomData,
};

pub(crate) use self::entry_payload::EntryPayload;
pub use self::ref_option::{RefMutOption, RefOption};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
/// An entry in entity list with generational ids, with the payload and the generation
pub struct EntityEntry<T> {
    pub(crate) gen: u32,
    pub(crate) payload: RefCell<EntryPayload<T>>,
}

impl<T> EntityEntry<T> {
    pub(crate) fn new(payload: T) -> Self {
        Self {
            gen: 0,
            payload: RefCell::new(EntryPayload::Occupied(payload)),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EntitySet<T> {
    v: Vec<EntityEntry<T>>,

    /// Index into `v` which is the start of free list, wrapped in a Cell to allow
    /// mutation through a shared reference, which can happen in `retain_borrow_mut`.
    free: Cell<Option<usize>>,
}

impl<T> EntitySet<T> {
    pub fn new() -> Self {
        Self {
            v: vec![],
            free: Cell::new(None),
        }
    }

    /// Returns the number of active elements in this EntitySet.
    /// It does _not_ return the buffer length.
    pub fn len(&self) -> usize {
        // TODO: optimize by caching active elements
        self.v
            .iter()
            .filter(|entry| entry.payload.borrow().is_occupied())
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
    pub fn items(&self) -> impl Iterator<Item = (EntityId<T>, RefOption<T>)> {
        self.v.iter().enumerate().filter_map(|(i, v)| {
            Some((EntityId::new(i as u32, v.gen), RefOption::new(&v.payload)?))
        })
    }

    /// Return an iterator over (id, &mut T).
    /// It is convenient when you want the EntityId of the iterated items.
    /// It does not borrow the T with a RefMut, because the self is already exclusively referenced.
    pub fn items_mut(&mut self) -> impl Iterator<Item = (EntityId<T>, &mut T)> {
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
    pub fn items_borrow_mut(&self) -> impl Iterator<Item = (EntityId<T>, RefMutOption<T>)> {
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

    pub fn insert(&mut self, val: T) -> EntityId<T> {
        for (i, entry) in self.v.iter_mut().enumerate() {
            let payload = entry.payload.get_mut();
            if payload.is_free() {
                entry.gen += 1;
                entry.payload = RefCell::new(EntryPayload::Occupied(val));
                return EntityId::new(i as u32, entry.gen);
            }
        }
        self.v.push(EntityEntry::new(val));
        EntityId::new(self.v.len() as u32 - 1, 0)
    }

    pub fn remove(&mut self, id: EntityId<T>) -> Option<T> {
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
                entry.payload = RefCell::new(EntryPayload::Free(self.free.get()));
            }
        }
    }

    pub fn retain_borrow_mut(&self, mut f: impl FnMut(&mut T, EntityId<T>) -> bool) {
        for (i, entry) in self.v.iter().enumerate() {
            let Ok(mut payload) = entry.payload.try_borrow_mut() else {
                continue;
            };
            if payload.is_free() {
                continue;
            }
            if !f(
                payload.as_mut().unwrap(),
                EntityId::new(i as u32, entry.gen),
            ) {
                *payload = EntryPayload::Free(Some(i));
            }
        }
    }

    pub fn get(&self, id: EntityId<T>) -> Option<RefOption<T>> {
        self.v.get(id.id as usize).and_then(|entry| {
            if id.gen == entry.gen {
                RefOption::new(&entry.payload)
            } else {
                None
            }
        })
    }

    pub fn get_mut(&mut self, id: EntityId<T>) -> Option<&mut T> {
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
        Self {
            v,
            free: Cell::new(None),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct EntityId<T> {
    id: u32,
    gen: u32,
    _ph: PhantomData<fn(T)>,
}

impl<T> std::clone::Clone for EntityId<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            gen: self.gen,
            _ph: self._ph,
        }
    }
}

impl<T> std::marker::Copy for EntityId<T> {}

impl<T> std::fmt::Debug for EntityId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityId({}, {})", self.id, self.gen)
    }
}

impl<T> std::cmp::PartialEq for EntityId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.gen == other.gen
    }
}

impl<T> std::cmp::Eq for EntityId<T> {}

impl<T> std::hash::Hash for EntityId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.gen.hash(state);
    }
}

impl<T> EntityId<T> {
    fn new(id: u32, gen: u32) -> Self {
        Self {
            id,
            gen,
            _ph: PhantomData,
        }
    }
}

impl<T> Display for EntityId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.id, self.gen)
    }
}

/// An extension trait to allow a container to iterate over valid items
pub trait EntityIterExt<T> {
    /// Iterate items in each entry's payload
    #[allow(dead_code)]
    fn items<'a>(&'a self) -> impl Iterator<Item = RefOption<'a, T>>
    where
        T: 'a;
}

/// An extension trait to allow a container to mutably iterate over valid items
pub trait EntityIterMutExt<T>: EntityIterExt<T> {
    /// Mutably iterate items in each entry's payload
    #[allow(dead_code)]
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
