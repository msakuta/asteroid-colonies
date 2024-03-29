use rand::random;
use serde::{ser::Serialize, Deserialize, Serializer};
use std::cmp::Eq;

const SESSION_DIGITS: usize = 10;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize)]
pub struct SessionId(pub [u8; SESSION_DIGITS]);

impl SessionId {
    pub fn new() -> Self {
        Self(random())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .fold("".to_string(), |acc, cur| acc + &format!("{:02x}", cur))
        )
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        let mut ret = [0; SESSION_DIGITS];
        for (i, c) in s.bytes().enumerate() {
            let c = if (b'0'..=b'9').contains(&c) {
                c - b'0'
            } else if (b'a'..=b'f').contains(&c) {
                c - b'a' + 10
            } else {
                panic!();
            };
            ret[i / 2] |= c << ((1 - i % 2) * 4);
        }
        Self(ret)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self::from(&s as &str)
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        s.serialize(serializer)
    }
}
