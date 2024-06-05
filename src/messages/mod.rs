use eyre::Result;
use nom::branch::alt;
use nom::combinator::map;
use nom::{IResult, Offset};

use crate::messages::handshake::Handshake;
use crate::messages::keep_alive::KeepAlive;
use crate::messages::unknown::Unknown;
use crate::SansIo;

pub mod handshake;
pub mod keep_alive;
pub mod unknown;

/// Wrapper type for all messages that can be sent or received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Handshake(Handshake),
    KeepAlive(KeepAlive),
    Unknown(Unknown),
}

impl Message {
    /// Decode a message from a buffer, which might only contain a part of the message.
    /// Returns `Ok(None)` if the message was incomplete, and more data is needed.
    /// Returns `Err` if the message format was invalid.
    pub fn from_partial_buffer(buffer: &[u8]) -> Result<Option<DecodedMessage>> {
        let (i, message) = map(Message::decode, Some)(buffer).or_else(|e| match e {
            nom::Err::Incomplete(_) => Ok((buffer, None)),
            e => Err(e.to_owned()),
        })?;
        if let Some(message) = message {
            Ok(Some(DecodedMessage {
                consumed_bytes: buffer.offset(i),
                message,
            }))
        } else {
            Ok(None)
        }
    }
}

/// The outcome of trying to decode a message from a buffer.
pub struct DecodedMessage {
    /// The number of bytes consumed by the decoder.
    pub consumed_bytes: usize,
    pub message: Message,
}

impl SansIo for Message {
    fn decode(i: &[u8]) -> IResult<&[u8], Self> {
        let handshake = map(Handshake::decode, Message::Handshake);
        let keep_alive = map(KeepAlive::decode, Message::KeepAlive);
        let unknown = map(Unknown::decode, Message::Unknown);
        alt((handshake, keep_alive, unknown))(i)
    }

    fn encode(&self) -> Vec<u8> {
        match self {
            Message::Handshake(handshake) => handshake.encode(),
            Message::KeepAlive(keep_alive) => keep_alive.encode(),
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
    fn roundtrip_keep_alive() {
        let message = Message::KeepAlive(KeepAlive);

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
