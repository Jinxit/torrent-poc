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
    pub fn new(own_peer_id: PeerId, info_hash: InfoHash) -> Self {
        let actor = Handle::spawn(TorrentActor::new(own_peer_id, info_hash));
        Self { actor }
    }

    /// Connects to a known peer, optionally with an expected peer id.
    /// In a real application peers would be discovered using a DHT or a tracker.
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
