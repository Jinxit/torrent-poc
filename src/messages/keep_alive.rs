use nom::bytes::streaming::tag;
use nom::combinator::{cut, success};

use crate::SansIo;

/// The keep-alive is sent periodically by either peer to keep the connection alive.
/// It's a simple message that doesn't contain any information.
/// It is encoded as a 4-byte message only containing the length of the message,
/// and that length is always 0.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KeepAlive;

impl SansIo for KeepAlive {
    fn decode(i: &[u8]) -> nom::IResult<&[u8], Self> {
        let (i, _) = tag([0; 4])(i)?;
        // Keep-alive messages are the only zero-length messages, cut other message types.
        // Sidenote: how do you cut -after- the last parser? This works but looks odd.
        let (i, ()) = cut(success(()))(i)?;
        Ok((i, Self))
    }

    fn encode(&self) -> Vec<u8> {
        vec![0; 4]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let keep_alive = KeepAlive;

        let encoded = keep_alive.encode();
        let (remaining, decoded) = KeepAlive::decode(&encoded).unwrap();

        assert_eq!(keep_alive, decoded);
        assert_eq!(remaining.len(), 0);
    }
}
