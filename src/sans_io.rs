use nom::IResult;

/// The [SansIo] trait is used to encode and decode messages without any knowledge of the
/// underlying transport. This is useful for testing, and also for implementing the torrent
/// protocol without having to implement a specific transport.
///
/// Maybe the name could be better.
pub trait SansIo: Sized {
    /// Decode a message from a buffer, which might only contain a part of the message.
    /// Returns `Ok(None)` if the message was incomplete, and more data is needed.
    /// Returns `Err` if the message format was invalid.
    ///
    /// The API currently makes two big assumptions:
    /// 1. The buffer is a simple contiguous byte slice.
    /// 2. The `nom` package is used to parse the message. (due to the use of [nom::IResult])
    fn decode(i: &[u8]) -> IResult<&[u8], Self>;

    /// Encode a message into a buffer. This is infallible.
    ///
    /// The API currently assumes that the message is small enough that fitting it in
    /// memory is not a problem.
    fn encode(&self) -> Vec<u8>;
}
