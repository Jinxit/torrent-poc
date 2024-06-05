use eyre::Result;

use crate::messages::Message;

pub mod std_io_connection;

/// A `Connection` is the bridge between the sans-io world of the protocol/client implementation
/// and the real world connection to a network.
// TODO: Could this be adjusted to support both async and sync connections?
//       We're probably stuck with colored functions locking us out of this,
//       but it would be cool if we could have that flexibility without
//       specializing the Torrent/Connection actors too much.
//       Maybe Async-first with Sync Connections being wrapped as blocking sections?
pub trait Connection {
    /// Send a message to the peer. The `Connection` is in charge of encoding the message
    /// (using the `SansIo` trait) and sending it over whatever transport it is using.
    fn send(&mut self, message: Message) -> Result<()>;

    /// Wait for a message from the peer, blocking the execution thread until one arrives.
    /// The `Connection` is also in charge of decoding the message (using the `SansIo` trait)
    /// as well as any necessary buffering/retrying if the message is incomplete.
    fn receive(&self) -> Result<Message>;
}
