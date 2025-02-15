use std::collections::HashMap;

use eyre::{OptionExt, Result};
use tracing::info;

use crate::actor::actor::Actor;
use crate::actor::handle::Handle;
use crate::actor::outcome::Outcome;
use crate::torrent::connection_actor::ConnectionActor;
use crate::{ConnectionRead, ConnectionWrite, InfoHash, PeerId};

/// This actor handles the lifecycle of a single torrent, and its multiple connections to peers.
#[derive(Debug)]
pub struct TorrentActor {
    handle: Option<Handle<TorrentActor>>,
    own_peer_id: PeerId,
    info_hash: InfoHash,
    connections: HashMap<PeerId, Handle<ConnectionActor>>,
}

impl TorrentActor {
    pub fn new(own_peer_id: PeerId, info_hash: InfoHash) -> Self {
        Self {
            handle: None,
            own_peer_id,
            info_hash,
            connections: HashMap::new(),
        }
    }

    pub fn connect_to_peer(
        &mut self,
        expected_peer_id: Option<PeerId>,
        connection_read: impl ConnectionRead + Send + 'static,
        connection_write: impl ConnectionWrite + Send + 'static,
    ) -> Result<Outcome> {
        let actor = Handle::spawn(ConnectionActor::new(
            self.own_peer_id,
            expected_peer_id,
            connection_read,
            connection_write,
            self.info_hash,
            self.handle.clone().ok_or_eyre("Handle not set")?,
        ));
        actor.act(ConnectionActor::initiate_handshake)?;
        Ok(Outcome::Continue)
    }

    pub fn accept_peer_connection(
        &mut self,
        expected_peer_id: Option<PeerId>,
        connection_read: impl ConnectionRead + Send + 'static,
        connection_write: impl ConnectionWrite + Send + 'static,
    ) -> Result<Outcome> {
        let actor = Handle::spawn(ConnectionActor::new(
            self.own_peer_id,
            expected_peer_id,
            connection_read,
            connection_write,
            self.info_hash,
            self.handle.clone().ok_or_eyre("Handle not set")?,
        ));
        actor.act(ConnectionActor::await_handshake)?;
        Ok(Outcome::Continue)
    }

    pub fn send(&mut self, peer_id: PeerId, message: String) -> Result<Outcome> {
        self.connections
            .get(&peer_id)
            .ok_or_eyre("Peer not connected")?
            .act(move |connection| {
                info!("TorrentActor sending message to peer {}", peer_id);
                connection.send(message)?;
                Ok(Outcome::Continue)
            })?;
        Ok(Outcome::Continue)
    }

    pub fn add_connection(&mut self, peer_id: PeerId, connection: Handle<ConnectionActor>) {
        self.connections.insert(peer_id, connection);
        info!("TorrentActor added connection to peer {}", peer_id);
    }

    pub fn remove_connection(&mut self, peer_id: PeerId) {
        self.connections.remove(&peer_id);
        info!("TorrentActor removed connection to peer {}", peer_id);
    }

    pub fn send_keep_alive(&self) -> Result<()> {
        for connection in self.connections.values() {
            connection.act(move |connection| {
                connection.send_keep_alive()?;
                Ok(Outcome::Continue)
            })?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn has_connection(&self, peer_id: PeerId) -> bool {
        self.connections.contains_key(&peer_id)
    }
}

impl Actor for TorrentActor {
    fn set_handle(&mut self, handle: &Handle<TorrentActor>) {
        self.handle = Some(handle.clone());
    }
}

impl Drop for TorrentActor {
    fn drop(&mut self) {
        for connection in self.connections.values() {
            let _ = connection.stop();
        }
    }
}
