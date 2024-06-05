use eyre::Result;
use tracing::info;

use crate::actor::handle::Handle;
use crate::actor::outcome::Outcome;
use crate::torrent::torrent_actor::TorrentActor;
use crate::{Connection, InfoHash, PeerId};

/// This is the main entry point for this library, a "root aggregate" if you will.
/// It's a cloneable handle (reference) to the torrent actor.
#[derive(Clone)]
pub struct Torrent {
    actor: Handle<TorrentActor>,
}

impl Torrent {
    /// Create a new torrent with the given peer ID and info hash.
    ///
    /// After this call, the torrent is not connected to any peers, so make sure to call
    /// `connect_to_peer` or `accept_peer_connection` to actually initiate communication.
    pub fn new(own_peer_id: PeerId, info_hash: InfoHash) -> Self {
        let actor = Handle::spawn(TorrentActor::new(own_peer_id, info_hash));
        Self { actor }
    }

    /// Connects to a known peer, optionally with an expected peer ID.
    /// In a real application peers would be discovered using a DHT or a tracker.
    ///
    /// If a specific peer ID is expected and the connection's peer ID does not match,
    /// the connection will be closed. If the info hash of the `Torrent` does not match
    /// the connection's info hash, the connection will be closed. If the first received
    /// message is not a handshake, the connection will be closed.
    pub fn connect_to_peer(
        &self,
        expected_peer_id: Option<PeerId>,
        connection: impl Connection + Send + 'static,
    ) -> Result<()> {
        self.actor.act(move |torrent| {
            torrent.connect_to_peer(expected_peer_id, connection)?;
            Ok(Outcome::Continue)
        })
    }

    /// Accept a connection from a peer that connected to us, optionally with an expected peer ID.
    ///
    /// If a specific peer ID is expected and the connection's peer ID does not match,
    /// the connection will be closed. If the info hash of the `Torrent` does not match
    /// the connection's info hash, the connection will be closed. If the first received
    /// message is not a handshake, the connection will be closed.
    pub fn accept_peer_connection(
        &self,
        expected_peer_id: Option<PeerId>,
        connection: impl Connection + Send + 'static,
    ) -> Result<()> {
        self.actor.act(move |torrent| {
            torrent.accept_peer_connection(expected_peer_id, connection)?;
            Ok(Outcome::Continue)
        })
    }

    /// Dummy method to send a "message" to a peer.
    pub fn send(&self, peer_id: PeerId, message: String) -> Result<()> {
        self.actor.act(move |torrent| {
            info!("Torrent sending message to peer {}", peer_id);
            torrent.send(peer_id, message)?;
            Ok(Outcome::Continue)
        })
    }
}

/// Ensures any in-progress actions finish running before the torrent is dropped, avoiding
/// disk corruption.
impl Drop for Torrent {
    fn drop(&mut self) {
        let _ = self.actor.stop();
    }
}
