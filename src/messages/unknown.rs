use nom::combinator::map_res;
use nom::error::Error;
use nom::multi::count;
use nom::number::streaming::be_u32;

use crate::sans_io::SansIo;

/// This message type will catch any unimplemented message types, as the BitTorrent protocol
/// specifies that all non-handshake messages have the same format, and that format also
/// includes the message length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    pub id: u8,
    pub bytes: Vec<u8>,
}

impl Unknown {
    #[must_use]
    pub fn new(id: u8, bytes: Vec<u8>) -> Self {
        Unknown { id, bytes }
    }
}

impl SansIo for Unknown {
    fn decode(i: &[u8]) -> nom::IResult<&[u8], Self> {
        // no sensible messages should be longer than 1MB
        let (i, message_length) = map_res(be_u32, |length| {
            if length < 1024 * 1024 {
                Ok(length)
            } else {
                Err(nom::Err::Error(Error::new(
                    i,
                    nom::error::ErrorKind::TooLarge,
                )))
            }
        })(i)?;
        let (i, id) = nom::number::streaming::u8(i)?;
        let (i, bytes) = count(nom::number::streaming::u8, (message_length - 1) as usize)(i)?;
        Ok((i, Self::new(id, bytes)))
    }

    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 1 + self.bytes.len());
        // the max length of the byte array is measured with a u32, so the cast is safe
        #[allow(clippy::cast_possible_truncation)]
        buf.extend(((1 + self.bytes.len()) as u32).to_be_bytes());
        buf.push(self.id);
        buf.extend(&self.bytes);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let unknown = Unknown::new(23, vec![3, 4, 5]);

        let encoded = unknown.encode();
        let (remaining, decoded) = Unknown::decode(&encoded).unwrap();

        assert_eq!(unknown, decoded);
        assert_eq!(remaining.len(), 0);
    }
}
