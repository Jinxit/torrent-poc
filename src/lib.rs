#![warn(clippy::unwrap_used)]
#![allow(clippy::module_inception)]
#![warn(missing_docs)]

//! A sans-io proof-of-concept implementation of the torrent protocol,
//! implemented in Rust as a programming challenge for recruitment purposes.
//!
//! This crate contains the core logic of the torrent protocol, and is intended to be used
//! as a library for building torrent clients and servers.
//!
//! The paradigm used in this crate is that of an actor model, where each piece of logic
//! (in this case, a `Torrent` and its individual `Connection`s) is an actor that can be
//! independently started and stopped, and runs on a separate thread.

pub use connections::std_io_connection::StdIoConnection;
pub use connections::Connection;
pub use info_hash::InfoHash;
pub use peer_id::PeerId;
pub use sans_io::SansIo;
pub use torrent::torrent::Torrent;

pub(crate) mod actor;
mod connections;
mod info_hash;
pub(crate) mod messages;
mod peer_id;
mod sans_io;
mod torrent;
