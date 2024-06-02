use nom::branch::alt;
use nom::combinator::map;
use nom::IResult;

use crate::messages::handshake::Handshake;
use crate::messages::unknown::Unknown;
use crate::SansIo;

pub mod handshake;
mod unknown;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Handshake(Handshake),
    Unknown(Unknown),
}

impl SansIo for Message {
    fn decode(i: &[u8]) -> IResult<&[u8], Self> {
        let handshake = map(Handshake::decode, Message::Handshake);
        let unknown = map(Unknown::decode, Message::Unknown);
        alt((handshake, unknown))(i)
    }

    fn encode(&self) -> Vec<u8> {
        match self {
            Message::Handshake(handshake) => handshake.encode(),
            Message::Unknown(unknown) => unknown.encode(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{InfoHash, PeerId};

    use super::*;

    #[test]
    fn roundtrip_handshake() {
        let message =
            Message::Handshake(Handshake::new(InfoHash::new([1; 20]), PeerId::new([2; 20])));

        let encoded = message.encode();
        let (remaining, decoded) = Message::decode(&encoded).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn roundtrip_unknown() {
        let message = Message::Unknown(Unknown::new(23, vec![3, 4, 5]));

        let encoded = message.encode();
        let (remaining, decoded) = Message::decode(&encoded).unwrap();

        assert_eq!(message, decoded);
        assert_eq!(remaining.len(), 0);
    }
}
