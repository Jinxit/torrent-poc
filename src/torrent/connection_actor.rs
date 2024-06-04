use std::fmt::Debug;

use eyre::{bail, OptionExt};
use tracing::info;

use crate::actor::actor::Actor;
use crate::actor::handle::Handle;
use crate::actor::outcome::Outcome;
use crate::messages::handshake::Handshake;
use crate::messages::Message;
use crate::torrent::torrent_actor::TorrentActor;
use crate::{Connection, InfoHash, PeerId};

/// This actor handles the connection to a single peer.
pub struct ConnectionActor {
    handle: Option<Handle<ConnectionActor>>,
    own_peer_id: PeerId,
    expected_peer_id: Option<PeerId>,
    info_hash: InfoHash,
    torrent: Handle<TorrentActor>,
    connection: Option<Box<dyn Connection + Send + 'static>>,
}

impl ConnectionActor {
    pub fn new(
        own_peer_id: PeerId,
        expected_peer_id: Option<PeerId>,
        connection: impl Connection + Send + 'static,
        info_hash: InfoHash,
        torrent: Handle<TorrentActor>,
    ) -> Self {
        Self {
            handle: None,
            own_peer_id,
            expected_peer_id,
            info_hash,
            torrent,
            connection: Some(Box::new(connection)),
        }
    }

    pub fn initiate_handshake(&mut self) -> eyre::Result<Outcome> {
        let mut connection = self.connection.take().expect("connection to be set");
        connection.send(Message::Handshake(Handshake::new(
            self.info_hash,
            self.own_peer_id,
        )))?;
        let message = connection.receive()?;
        if let Message::Handshake(handshake) = message {
            if handshake.info_hash != self.info_hash {
                bail!("Peer sent an incorrect info hash");
            }

            if self
                .expected_peer_id
                .is_some_and(|expected| expected != handshake.peer_id)
            {
                bail!("Peer sent an incorrect peer id");
            }
            self.expected_peer_id = Some(handshake.peer_id);

            let handle = self.handle.clone().ok_or_eyre("Handle not set")?;
            self.torrent.act(move |torrent| {
                torrent.add_connection(handshake.peer_id, handle);
                Ok(Outcome::Continue)
            })?;

            info!("Connection established with peer {}", handshake.peer_id);
            // TODO: Join handle?
            let _ = std::thread::spawn(move || {
                // `receive()` will block until a message is received, so it needs to be run in a
                // separate thread.
                while let Ok(message) = connection.receive() {
                    info!("Actor received message: {:?}", message);
                }
            });
        } else {
            bail!("Expected handshake message, peer sent something else: {message:?}");
        }
        Ok(Outcome::Continue)
    }

    pub fn send(&mut self, _message: String) -> eyre::Result<Outcome> {
        info!(
            "TorrentActor sending message to peer {}",
            self.expected_peer_id.unwrap()
        );
        // TODO: This doesn't do anything yet, but showcases the expected structure of the code.
        Ok(Outcome::Continue)
    }
}

impl Actor for ConnectionActor {
    fn set_handle(&mut self, handle: &Handle<ConnectionActor>) {
        self.handle = Some(handle.clone());
    }

    fn stop(&mut self) {
        if let Some(peer_id) = self.expected_peer_id {
            let _ = self.torrent.act(move |torrent| {
                torrent.remove_connection(peer_id);
                Ok(Outcome::Continue)
            });
        }
    }
}

impl Debug for ConnectionActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionActor")
            .field("own_peer_id", &self.own_peer_id)
            .field("expected_peer_id", &self.expected_peer_id)
            .field("info_hash", &self.info_hash)
            .field("torrent", &self.torrent)
            .field("handle", &self.handle)
            .finish()
    }
}
