use nom_derive::Nom;

use crate::{InfoHash, PeerId};

// Nom-derive Ignore+Tag fails with () but works on a type alias
type Ignore = ();

const BITTORRENT_PROTOCOL: &[u8] = b"BitTorrent protocol";
const RESERVED_ZEROES: &[u8] = b"\0\0\0\0\0\0\0\0";

#[derive(Nom, Debug)]
#[nom(Debug)]
pub struct Handshake {
    #[nom(Ignore, Tag(& [BITTORRENT_PROTOCOL.len()]))]
    _pstrlen: Ignore,
    #[nom(Ignore, Tag(BITTORRENT_PROTOCOL))]
    _protocol: Ignore,
    #[nom(Ignore, Tag(RESERVED_ZEROES))]
    _reserved: Ignore,
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
}

impl Handshake {
    pub fn new(info_hash: InfoHash, peer_id: PeerId) -> Self {
        Self {
            _pstrlen: (),
            _protocol: (),
            _reserved: (),
            info_hash,
            peer_id,
        }
    }
}

impl From<Handshake> for Vec<u8> {
    fn from(handshake: Handshake) -> Self {
        let mut bytes = Vec::with_capacity(1 + 19 + 8 + 20 + 20);
        bytes.push(BITTORRENT_PROTOCOL.len() as u8);
        bytes.extend_from_slice(BITTORRENT_PROTOCOL);
        bytes.extend_from_slice(RESERVED_ZEROES);
        bytes.extend_from_slice(&Vec::<u8>::from(handshake.info_hash));
        bytes.extend_from_slice(&Vec::<u8>::from(handshake.peer_id));
        bytes
    }
}
