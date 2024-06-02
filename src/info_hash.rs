use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

/// A 20 byte hash of a torrent, usually represented as a hex string.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct InfoHash([u8; 20]);

impl InfoHash {
    /// Create a new InfoHash from a byte array.
    #[must_use]
    pub fn new(hash: [u8; 20]) -> Self {
        Self(hash)
    }
}

impl FromStr for InfoHash {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for InfoHash {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut hash = [0u8; 20];
        hex::decode_to_slice(value, &mut hash)?;
        Ok(Self(hash))
    }
}

impl Display for InfoHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// Manually implemented because the derived Vec<u8> Debug reads awfully.
impl Debug for InfoHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "InfoHash({})", hex::encode(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "018e50b58106b84a42c223ccf0494334f8d55958";
    const HASH_BYTES: [u8; 20] = [
        0x01, 0x8e, 0x50, 0xb5, 0x81, 0x06, 0xb8, 0x4a, 0x42, 0xc2, 0x23, 0xcc, 0xf0, 0x49, 0x43,
        0x34, 0xf8, 0xd5, 0x59, 0x58,
    ];

    #[test]
    fn parse() {
        let hash = InfoHash::try_from(HASH).unwrap();
        assert_eq!(hash, InfoHash(HASH_BYTES));
    }

    #[test]
    fn display() {
        let hash = InfoHash::new(HASH_BYTES);
        let formatted = format!("{}", hash);
        assert_eq!(formatted, HASH);
    }

    #[test]
    fn debug() {
        let hash = InfoHash::new(HASH_BYTES);
        let formatted = format!("{:?}", hash);
        assert_eq!(formatted, format!("InfoHash({})", HASH));
    }
}
