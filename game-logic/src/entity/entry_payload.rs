use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) enum EntryPayload<T> {
    Occupied(T),         // Contains a component
    Free(Option<usize>), // Points to the next free slot
}

// Option-like methods
impl<T> EntryPayload<T> {
    pub(super) fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied(_))
    }

    pub(super) fn is_free(&self) -> bool {
        matches!(self, Self::Free(_))
    }

    pub(super) fn as_ref_unwrap(&self) -> &T {
        match self {
            Self::Occupied(ref payload) => payload,
            _ => unreachable!(),
        }
    }

    pub(super) fn as_mut_unwrap(&mut self) -> &mut T {
        match self {
            Self::Occupied(ref mut payload) => payload,
            _ => unreachable!(),
        }
    }

    pub(super) fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Occupied(ref mut payload) => Some(payload),
            _ => None,
        }
    }

    pub(super) fn take(&mut self) -> Option<T> {
        let mut swap = Self::Free(None);
        std::mem::swap(self, &mut swap);
        match swap {
            Self::Occupied(payload) => Some(payload),
            _ => None,
        }
    }
}
