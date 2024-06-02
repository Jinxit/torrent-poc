use std::net::IpAddr;

use clap::Parser;
use tracing::info;

use torrent_poc::InfoHash;

/// A simple program to handshake with a known BitTorrent peer for a given Torrent info hash
///
/// Normally torrent clients and servers are the same thing (as it's a P2P protocol),
/// and the "leechers" would find "seeders" from a tracker or DHT. Having this CLI lets me
/// implement the core protocol without first having to implement tracker or DHT protocols.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Cli {
    Leech {
        /// IP address of the known peer
        #[arg(long)]
        ip: IpAddr,

        /// Port of the known peer
        #[arg(long)]
        port: u16,

        /// Info hash of the torrent to leech
        #[arg(long)]
        info_hash: InfoHash,
    },
    Seed {
        /// IP address to listen on (defaults to all interfaces)
        #[arg(long, default_value = "0.0.0.0")]
        ip: IpAddr,

        /// Port to listen on (defaults to a random port)
        #[arg(long, default_value = "0")]
        port: u16,

        /// Info hash of the torrent to seed
        #[arg(long)]
        info_hash: InfoHash,
    },
}

fn main() -> Result<(), eyre::Report> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let cli = Cli::parse();
    match cli {
        Cli::Leech {
            ip,
            port,
            info_hash,
        } => {
            info!("Connecting to peer at {}:{}", ip, port);
            info!("Info hash: {}", info_hash);
        }
        Cli::Seed {
            ip,
            port,
            info_hash,
        } => {
            // TODO: get random port from OS
            info!("Listening on {}:{}", ip, port);
            info!("Info hash: {}", info_hash);
            todo!()
        }
    }

    Ok(())
}
