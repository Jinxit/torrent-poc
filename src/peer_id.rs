use std::fmt::{Debug, Display, Formatter};

use base58::ToBase58;
use eyre::{bail, Result};
use nom::bytes::streaming::take;
use nom::combinator::map_res;
use rand::Rng;

use crate::SansIo;

/// A 20 byte hash of a torrent, technically _any_ bytes but usually implemented as:
/// -XY1234-\<random characters\>
///
/// where XY is an application-specific identifier, 1234 is a version number, and the random
/// characters are a unique identifier for the peer.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId([u8; 20]);

impl PeerId {
    /// Create a fixed peer ID from a byte array.
    #[must_use]
    pub fn new(hash: [u8; 20]) -> Self {
        Self(hash)
    }

    /// Create a random peer ID using a supplied identifier and version number.
    ///
    /// This makes a few assumptions about the version number:
    /// - Major fits in a single base58 character (0-57)
    /// - Minor fits in two base58 characters (0-3363)
    /// - Patch fits in a single base58 character (0-57)
    pub fn random(identifier: &[u8; 2], major: u8, minor: u16, patch: u8) -> Result<Self> {
        let mut hash = Vec::with_capacity(20);
        hash.push(b'-');
        hash.extend_from_slice(identifier);

        let major_str = [major].to_base58();
        let major_bytes = major_str.as_bytes();
        if major_bytes.len() != 1 {
            bail!("Couldn't parse major version {major} as a single base58 character (was: \"{major_str}\")");
        }
        hash.push(major_bytes[0]);

        let minor_str = [
            [(minor / 58) as u8].to_base58(),
            [(minor % 58) as u8].to_base58(),
        ];
        if minor_str[0].len() != 1 || minor_str[1].len() != 1 {
            bail!(
                "Couldn't parse minor version {minor} as two base58 characters (was: \"{}\")",
                minor_str.join("")
            );
        }
        let minor_bytes = [minor_str[0].as_bytes()[0], minor_str[1].as_bytes()[0]];
        hash.extend_from_slice(&minor_bytes);

        let patch_str = [patch].to_base58();
        let patch_bytes = patch_str.as_bytes();
        if patch_bytes.len() != 1 {
            bail!("Couldn't parse patch version {patch} as a single base58 character (was: \"{patch_str}\")");
        }
        hash.push(patch_bytes[0]);

        hash.push(b'-');

        let mut rng = rand::thread_rng();
        // Using base58 encoding for random bytes is certainly a choice,
        // but I just like base58. Compact but readable.
        let random_bytes = random_base58_bytes(&mut rng, 12);
        hash.extend_from_slice(&random_bytes);
        let hash = hash.try_into().expect("hash to be 20 bytes");
        Ok(Self(hash))
    }
}

const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn random_base58_bytes(rng: &mut impl Rng, length: usize) -> Vec<u8> {
    let dist = rand::distributions::Uniform::new(0, ALPHABET.len());
    rng.sample_iter(dist)
        .take(length)
        .map(|index| ALPHABET[index])
        .collect()
}

impl SansIo for PeerId {
    fn decode(i: &[u8]) -> nom::IResult<&[u8], Self> {
        let (i, peer_id) = map_res(take(20usize), TryInto::try_into)(i)?;
        Ok((i, Self(peer_id)))
    }

    fn encode(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Display for PeerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Most torrent clients assume the peer ID is a string, so we'll do the same.
        // Even if it displays poorly for some clients, most users won't even see this.
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

// Manually implemented because the derived Vec<u8> Debug reads awfully.
impl Debug for PeerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PeerId({})", String::from_utf8_lossy(&self.0))
    }
}

impl From<PeerId> for Vec<u8> {
    fn from(peer_id: PeerId) -> Self {
        peer_id.0.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use eyre::{eyre, WrapErr};

    use super::*;

    const PEER: &str = "-Rp0123-HahW9F2VDDzU";
    const PEER_BYTES: &[u8; 20] = b"-Rp0123-HahW9F2VDDzU";

    #[test]
    fn random_matches_format() {
        let random = PeerId::random(b"Rp", 22, 502, 11).unwrap();
        assert_eq!(&random.0[0..8], b"-RpP9fC-");
        for byte in &random.0[8..] {
            assert!(ALPHABET.contains(byte));
        }
    }

    #[test]
    fn random_using_crate_version_matches_format() {
        fn test(major: u8, minor: u16, patch: u8) {
            let random = PeerId::random(b"Rp", major, minor, patch)
                .wrap_err_with(|| eyre!("{major}.{minor}.{patch}"))
                .unwrap();
            assert_eq!(&random.0[0..3], b"-Rp");
            assert_eq!(random.0[7], b'-');
            for byte in &random.0[3..7] {
                assert!(ALPHABET.contains(byte));
            }
            for byte in &random.0[8..] {
                assert!(ALPHABET.contains(byte));
            }
        }
        let mut rng = rand::thread_rng();
        // Test limits
        test(0, 0, 0);
        test(57, 3363, 57);

        // Test a few random values
        for _ in 0..100 {
            test(
                rng.gen_range(0..=57),
                rng.gen_range(0..=3363),
                rng.gen_range(0..=57),
            );
        }
    }

    #[test]
    fn random_using_crate_version_out_of_range_err() {
        let err = PeerId::random(b"Rp", 58, 0, 0).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Couldn't parse major version 58 as a single base58 character (was: \"21\")"
        );
        let err = PeerId::random(b"Rp", 0, 5002, 0).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Couldn't parse minor version 5002 as two base58 characters (was: \"2VF\")"
        );
        let err = PeerId::random(b"Rp", 0, 0, 255).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Couldn't parse patch version 255 as a single base58 character (was: \"5Q\")"
        );
    }

    #[test]
    fn display() {
        let hash = PeerId::new(*PEER_BYTES);
        let formatted = format!("{}", hash);
        assert_eq!(formatted, PEER);
    }

    #[test]
    fn debug() {
        let hash = PeerId::new(*PEER_BYTES);
        let formatted = format!("{:?}", hash);
        assert_eq!(formatted, format!("PeerId({})", PEER));
    }
}
