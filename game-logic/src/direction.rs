use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl std::hash::Hash for Direction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        ((*self) as u8).hash(state)
    }
}

impl Direction {
    pub(crate) const fn all() -> [Direction; 4] {
        [Self::Left, Self::Up, Self::Right, Self::Down]
    }

    pub(crate) fn to_vec(&self) -> [i32; 2] {
        match self {
            Self::Left => [-1, 0],
            Self::Up => [0, -1],
            Self::Right => [1, 0],
            Self::Down => [0, 1],
        }
    }

    pub(crate) fn from_vec(v: [i32; 2]) -> Option<Self> {
        Some(match (v[0].signum(), v[1].signum()) {
            (-1, _) => Self::Left,
            (1, _) => Self::Right,
            (0, -1) => Self::Up,
            (0, 1) => Self::Down,
            _ => return None,
        })
    }

    pub(crate) fn reverse(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
        }
    }
}
