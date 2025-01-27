use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

use super::EntryPayload;

#[derive(Debug)]
/// A wrapper around Ref<EntryPayload> that always has Occupied.
/// We need a Ref to release the refcounter, but we would never return
/// a Ref(EntryPayload::Free).
///
/// It used to wrap Ref<Option>, therefore the name, but we implemented EntryPayload
/// in place of Option, so the name isn't strictly accurate anymore.
pub struct RefOption<'a, T>(Ref<'a, EntryPayload<T>>);

impl<'a, T> RefOption<'a, T> {
    pub(super) fn new(val: &'a RefCell<EntryPayload<T>>) -> Option<Self> {
        let v = val.try_borrow().ok()?;
        if matches!(&*v, EntryPayload::Free(_)) {
            return None;
        }
        Some(Self(v))
    }
}

impl<'a, T> Deref for RefOption<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref_unwrap()
    }
}

/// A wrapper around RefMut<EntryPayload> that always has Occupied.
/// We need a RefMut to release the refcounter, but we would never return
/// a RefMut(EntryPayload::Free).
///
/// It used to wrap RefMut<Option>, therefore the name, but we implemented EntryPayload
/// in place of Option, so the name isn't strictly accurate anymore.
pub struct RefMutOption<'a, T>(RefMut<'a, EntryPayload<T>>);

impl<'a, T> RefMutOption<'a, T> {
    pub(super) fn new(val: &'a RefCell<EntryPayload<T>>) -> Option<Self> {
        let v = val.try_borrow_mut().ok()?;
        if matches!(&*v, EntryPayload::Free(_)) {
            return None;
        }
        Some(Self(v))
    }
}

impl<'a, T> Deref for RefMutOption<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref_unwrap()
    }
}

impl<'a, T> DerefMut for RefMutOption<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_unwrap()
    }
}
