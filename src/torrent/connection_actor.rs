use std::fmt::Debug;

use eyre::{bail, OptionExt, Result};
use tracing::{info, trace, warn};

use crate::actor::actor::Actor;
use crate::actor::handle::Handle;
use crate::actor::outcome::Outcome;
use crate::messages::Message;
use crate::messages::{Handshake, KeepAlive};
use crate::torrent::torrent_actor::TorrentActor;
use crate::{ConnectionRead, ConnectionWrite, InfoHash, PeerId};

/// This actor handles the connection to a single peer.
pub struct ConnectionActor {
    handle: Option<Handle<ConnectionActor>>,
    own_peer_id: PeerId,
    peer_id: Option<PeerId>,
    info_hash: InfoHash,
    torrent: Handle<TorrentActor>,
    connection_read: Option<Box<dyn ConnectionRead + Send + 'static>>,
    connection_write: Box<dyn ConnectionWrite + Send + 'static>,
}

impl ConnectionActor {
    pub fn new(
        own_peer_id: PeerId,
        expected_peer_id: Option<PeerId>,
        connection_read: impl ConnectionRead + Send + 'static,
        connection_write: impl ConnectionWrite + Send + 'static,
        info_hash: InfoHash,
        torrent: Handle<TorrentActor>,
    ) -> Self {
        Self {
            handle: None,
            own_peer_id,
            peer_id: expected_peer_id,
            info_hash,
            torrent,
            connection_read: Some(Box::new(connection_read)),
            connection_write: Box::new(connection_write),
        }
    }

    /// Initiate handshake with a peer on an outgoing connection.
    pub fn initiate_handshake(&mut self) -> Result<Outcome> {
        self.connection_write
            .send(Message::Handshake(Handshake::new(
                self.info_hash,
                self.own_peer_id,
            )))?;
        let connection_read = self
            .connection_read
            .take()
            .expect("connection_read to be set");
        let message = connection_read.receive()?;
        if let Message::Handshake(handshake) = message {
            if handshake.info_hash != self.info_hash {
                bail!("Peer sent an incorrect info hash");
            }

            if self
                .peer_id
                .is_some_and(|expected| expected != handshake.peer_id)
            {
                bail!("Peer sent an incorrect peer ID");
            }
            self.peer_id = Some(handshake.peer_id);

            let handle = self.handle.clone().ok_or_eyre("Handle not set")?;
            self.torrent.act({
                let handle = handle.clone();
                move |torrent| {
                    torrent.add_connection(handshake.peer_id, handle);
                    Ok(Outcome::Continue)
                }
            })?;

            info!("Connection established with peer {}", handshake.peer_id);
            Self::start_receive_loop(connection_read, handle);
        } else {
            bail!("Expected handshake message, peer sent something else: {message:?}");
        }

        Ok(Outcome::Continue)
    }

    fn start_receive_loop(
        connection_read: Box<dyn ConnectionRead + Send>,
        handle: Handle<ConnectionActor>,
    ) {
        // TODO: Join handle?
        let _ = std::thread::spawn(move || {
            // `receive()` will block until a message is received, so it needs to be run in a
            // separate thread.
            while let Ok(message) = connection_read.receive() {
                trace!("Actor received message: {:?}", message);
            }
            handle.stop().expect("thread to not panic");
        });
    }

    /// Wait for a handshake from a peer on an incoming connection.
    pub fn await_handshake(&mut self) -> Result<Outcome> {
        // TODO: This has a lot of shared code with `initiate_handshake()`, refactor?
        let connection_read = self.connection_read.take().expect("connection to be set");

        let message = connection_read.receive()?;
        if let Message::Handshake(handshake) = message {
            if handshake.info_hash != self.info_hash {
                bail!("Peer sent an incorrect info hash");
            }

            if self
                .peer_id
                .is_some_and(|expected| expected != handshake.peer_id)
            {
                bail!("Peer sent an incorrect peer ID");
            }
            self.peer_id = Some(handshake.peer_id);

            self.connection_write
                .send(Message::Handshake(Handshake::new(
                    self.info_hash,
                    self.own_peer_id,
                )))?;

            let handle = self.handle.clone().ok_or_eyre("Handle not set")?;
            self.torrent.act({
                let handle = handle.clone();
                move |torrent| {
                    torrent.add_connection(handshake.peer_id, handle);
                    Ok(Outcome::Continue)
                }
            })?;

            info!("Connection established with peer {}", handshake.peer_id);
            Self::start_receive_loop(connection_read, handle);
        } else {
            bail!("Expected handshake message, peer sent something else: {message:?}");
        }

        Ok(Outcome::Continue)
    }

    pub fn send(&mut self, _message: String) -> Result<Outcome> {
        info!(
            "TorrentActor sending message to peer {}",
            self.peer_id.expect("peer to be connected")
        );
        // TODO: This doesn't do anything yet, but showcases the expected structure of the code.
        Ok(Outcome::Continue)
    }

    pub fn send_keep_alive(&mut self) -> Result<Outcome> {
        warn!("Sending 10 keep-alives");
        for _ in 0..10 {
            self.connection_write.send(Message::KeepAlive(KeepAlive))?;
        }
        Ok(Outcome::Continue)
    }
}

impl Actor for ConnectionActor {
    fn set_handle(&mut self, handle: &Handle<ConnectionActor>) {
        self.handle = Some(handle.clone());
    }

    fn stop(&mut self) {
        if let Some(peer_id) = self.peer_id {
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
            .field("expected_peer_id", &self.peer_id)
            .field("info_hash", &self.info_hash)
            .field("torrent", &self.torrent)
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use thread::sleep;

    use eyre::{eyre, Result};

    use super::*;

    #[derive(Clone)]
    struct MockConnection {
        sent_messages: Arc<Mutex<Vec<Message>>>,
        queued_for_receive: Arc<Mutex<VecDeque<Message>>>,
    }

    impl MockConnection {
        fn new(queued_for_receive: VecDeque<Message>) -> Self {
            Self {
                sent_messages: Arc::default(),
                queued_for_receive: Arc::new(Mutex::new(queued_for_receive)),
            }
        }
    }

    impl ConnectionRead for MockConnection {
        fn receive(&self) -> Result<Message> {
            self.queued_for_receive
                .lock()
                .unwrap()
                .pop_front()
                // This simulates not getting any more network messages for 1 second, then
                // closing the connection.
                // The reason for this is that the `receive()` method will block until a message
                // is received, and in the test we want to verify that a connection exists -
                // if it is closed instantly, there's no way to verify that.
                .ok_or_else(|| {
                    sleep(Duration::from_secs(1));
                    eyre!("no message")
                })
        }
    }

    impl ConnectionWrite for MockConnection {
        fn send(&mut self, message: Message) -> Result<()> {
            self.sent_messages.lock().unwrap().push(message.clone());
            Ok(())
        }
    }

    #[test]
    fn initiate_handshake() {
        // This test is a bit of a doozy.
        // I would love to improve it, but it works for now.
        let client_id = PeerId::new([1; 20]);
        let server_id = PeerId::new([3; 20]);
        let info_hash = InfoHash::new([2; 20]);
        let torrent_actor = Handle::spawn(TorrentActor::new(client_id, info_hash));

        let client_handshake = Message::Handshake(Handshake::new(info_hash, client_id));
        let server_handshake = Message::Handshake(Handshake::new(info_hash, server_id));
        let connection = MockConnection::new(VecDeque::from([server_handshake]));

        let connection_actor = Handle::spawn(ConnectionActor::new(
            client_id,
            None,
            connection.clone(),
            connection.clone(),
            info_hash,
            torrent_actor.clone(),
        ));

        connection_actor
            .act(ConnectionActor::initiate_handshake)
            .unwrap();

        sleep(Duration::from_millis(100));

        connection_actor
            .act(move |connection_actor| {
                assert_eq!(Some(server_id), connection_actor.peer_id);
                Ok(Outcome::Continue)
            })
            .unwrap();
        torrent_actor
            .act(move |torrent_actor| {
                assert!(torrent_actor.has_connection(server_id));
                Ok(Outcome::Continue)
            })
            .unwrap();

        sleep(Duration::from_millis(100));

        assert_eq!(
            *connection.sent_messages.lock().unwrap(),
            vec![client_handshake]
        );
        assert_eq!(*connection.queued_for_receive.lock().unwrap(), vec![]);

        connection_actor.stop().unwrap();

        sleep(Duration::from_millis(100));

        torrent_actor
            .act(move |torrent_actor| {
                assert!(!torrent_actor.has_connection(server_id));
                Ok(Outcome::Continue)
            })
            .unwrap();

        sleep(Duration::from_millis(100));

        torrent_actor.stop().unwrap();
    }
}
