use std::io::{BufReader, BufWriter};
use std::net::{IpAddr, TcpListener, TcpStream};

use clap::Parser;
use tracing::{info, warn};

use torrent_poc::{std_io_connection, InfoHash, PeerId, Torrent};

/// A simple program to handshake with a known BitTorrent peer for a given Torrent info hash.
///
/// Normally torrent clients and servers are the same thing (as it's a P2P protocol),
/// and the "leechers" would find "seeders" (and other leechers) from a tracker or DHT.
/// Having this CLI lets me implement the core protocol without first having to implement
/// tracker or DHT protocols.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Cli {
    /// Connect to a known peer and start downloading a torrent.
    Leech {
        /// IP address of the known peer.
        #[arg(long)]
        ip: IpAddr,

        /// Port of the known peer.
        #[arg(long)]
        port: u16,

        /// Info hash of the torrent to leech.
        #[arg(long)]
        info_hash: InfoHash,
    },
    /// Listen for incoming connections and start seeding a torrent.
    Seed {
        /// IP address to listen on (defaults to all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        ip: IpAddr,

        /// Port to listen on (defaults to a random port)
        #[arg(long, default_value_t = 0)]
        port: u16,

        /// Info hash of the torrent to seed
        #[arg(long)]
        info_hash: InfoHash,
    },
}

fn main() -> Result<(), eyre::Report> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let major = env!("CARGO_PKG_VERSION_MAJOR");
    let minor = env!("CARGO_PKG_VERSION_MINOR");
    let patch = env!("CARGO_PKG_VERSION_PATCH");
    let own_peer_id = PeerId::random(b"Rp", major.parse()?, minor.parse()?, patch.parse()?)?;
    info!("My peer ID: {}", own_peer_id);

    let cli = Cli::parse();
    match cli {
        Cli::Leech {
            ip,
            port,
            info_hash,
        } => {
            info!("Connecting to peer at {}:{}", ip, port);
            info!("Info hash: {}", info_hash);
            let torrent = Torrent::new(own_peer_id, info_hash);
            let stream = TcpStream::connect((ip, port))?;
            let reader = BufReader::new(stream.try_clone()?);
            let writer = BufWriter::new(stream);
            let (connection_write, connection_read) = std_io_connection(1024, reader, writer);
            torrent.connect_to_peer(None, connection_read, connection_write)?;
            // Since actor threads are stopped on Drop, we just sleep here to let them tick a bit.
            // In a real application the Torrents would be stored in some kind of data structure
            // and the actor threads would be started and stopped as the user is manipulating the GUI.
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        Cli::Seed {
            ip,
            port,
            info_hash,
        } => {
            info!("Listening on {}:{}", ip, port);
            info!("Info hash: {}", info_hash);
            let torrent = Torrent::new(own_peer_id, info_hash);
            for stream in TcpListener::bind((ip, port))?.incoming() {
                let stream = stream?;
                let reader = BufReader::new(stream.try_clone()?);
                let writer = BufWriter::new(stream);
                let (connection_write, connection_read) = std_io_connection(1024, reader, writer);
                torrent.accept_peer_connection(None, connection_read, connection_write)?;
            }
        }
    }

    Ok(())
}
