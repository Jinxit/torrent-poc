use nom::bytes::streaming::{tag, take};
use nom::combinator::cut;

use crate::{InfoHash, PeerId, SansIo};

const BITTORRENT_PROTOCOL: &[u8] = b"BitTorrent protocol";
const RESERVED_ZEROES: &[u8] = b"\0\0\0\0\0\0\0\0";

/// The handshake is the first message sent by either peer when they start a connection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Handshake {
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
}

impl Handshake {
    #[must_use]
    pub fn new(info_hash: InfoHash, peer_id: PeerId) -> Self {
        Self { info_hash, peer_id }
    }
}

impl SansIo for Handshake {
    fn decode(i: &[u8]) -> nom::IResult<&[u8], Self> {
        // We only support the BitTorrent protocol, this is necessary to distinguish between
        // the handshake and the other messages.
        // (without building some kind of "only parse the handshake once" logic)
        let (i, _) = tag([19])(i)?;
        let (i, _) = tag(BITTORRENT_PROTOCOL)(i)?;
        // Past this point, we're definitely in the handshake, so we can cut other message types.
        // 8 bytes reserved for future use
        let (i, _) = cut(take(8usize))(i)?;
        let (i, info_hash) = InfoHash::decode(i)?;
        let (i, peer_id) = PeerId::decode(i)?;
        Ok((i, Self::new(info_hash, peer_id)))
    }

    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 19 + 8 + 20 + 20);
        buf.push(19u8);
        buf.extend(BITTORRENT_PROTOCOL);
        // 8 bytes reserved for future use
        buf.extend(RESERVED_ZEROES);
        buf.extend(self.info_hash.encode());
        buf.extend(self.peer_id.encode());
        buf
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use nom::Err::Incomplete;
    use nom::Needed;

    use super::*;

    const PEER_BYTES: [u8; 20] = *b"-Rp0123-HahW9F2VDDzU";

    #[test]
    fn roundtrip() {
        let handshake = Handshake::new(InfoHash::new([0; 20]), PeerId::new(PEER_BYTES));

        let encoded = handshake.encode();
        let (remaining, decoded) = Handshake::decode(&encoded).unwrap();

        assert_eq!(handshake, decoded);
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn roundtrip_with_extra_bytes() {
        let handshake = Handshake::new(InfoHash::new([0; 20]), PeerId::new(PEER_BYTES));

        let mut encoded = handshake.encode();
        encoded.push(0);
        encoded.push(2);
        encoded.push(5);

        let (remaining, decoded) = Handshake::decode(&encoded).unwrap();

        assert_eq!(handshake, decoded);
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn roundtrip_with_missing_bytes() {
        let handshake = Handshake::new(InfoHash::new([0; 20]), PeerId::new(PEER_BYTES));

        let mut encoded = handshake.encode();
        encoded.pop();
        encoded.pop();
        encoded.pop();

        let err = Handshake::decode(&encoded).unwrap_err();
        if let Incomplete(Needed::Size(needed)) = err {
            assert_eq!(needed, NonZeroUsize::new(3).unwrap());
        } else {
            panic!("expected Incomplete");
        }
    }
}
