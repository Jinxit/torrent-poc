# torrent-poc

A sans-io proof-of-concept implementation of the torrent protocol,
implemented in Rust as a programming challenge for recruitment purposes.

## Usage

<pre>
<b>$</b> cargo run -- --help
<b><u>Usage:</u> torrent-poc.exe</b> &lt;COMMAND&gt;

<b><u>Commands:</u></b>
  <b>leech</b>  Connect to a known peer and start downloading a torrent
  <b>seed</b>   Listen for incoming connections and start seeding a torrent
  <b>help</b>   Print this message or the help of the given subcommand(s)

<b><u>Options:</u></b>
  <b>-h, --help</b>     Print help
  <b>-V, --version</b>  Print version
</pre>

## Demo

1. Start a torrent "seeder" by running `cargo run -- seed --info-hash <info-hash> --port <port>`.

   It will listen on all interfaces by default, but you can specify a specific IP address with `--ip <ip>`.

2. Start a torrent "leecher" by running `cargo run -- leech --info-hash <info-hash> --port <port> --ip <ip>`.

   The port and info hash must match the ones used by the seeder.
3. You should now see something similar to this:
   <pre>
   <b>$</b> cargo run -- seed --info-hash 018e50b58106b84a42c223ccf0494334f8d55958 --port 12345
   <span style="color: grey">2024-06-05T12:27:22.220504Z  <span style="color: green">INFO</span> torrent_poc:</span> My peer ID: <span style="color: blue">-Rp1121-THMZfvNhcurL</span>
   <span style="color: grey">2024-06-05T12:27:22.220911Z  <span style="color: green">INFO</span> torrent_poc:</span> Listening on 0.0.0.0:12345
   <span style="color: grey">2024-06-05T12:27:22.220982Z  <span style="color: green">INFO</span> torrent_poc:</span> Info hash: <span style="color: purple">b6ae8e98e360a3d5d547dd43d42548ee786845ff</span>
   <span style="color: grey">2024-06-05T12:27:25.548916Z  <span style="color: green">INFO</span> torrent_poc::torrent::connection_actor:</span> Connection established with peer <span style="color: red">-Rp1121-8gJCKF636JRe</span>
   <span style="color: grey">2024-06-05T12:27:25.548922Z  <span style="color: green">INFO</span> torrent_poc::torrent::torrent_actor:</span> TorrentActor added connection to peer <span style="color: red">-Rp1121-8gJCKF636JRe</span>
   # 10 seconds later
   <span style="color: grey">2024-06-05T12:27:35.613601Z  <span style="color: goldenrod">WARN</span> torrent_poc::connections::std_io_connection:</span> error reading from the connection: Os { code: 10054, kind: ConnectionReset, message: "An existing connection was forcibly closed by the remote host." }
   <span style="color: grey">2024-06-05T12:27:35.639136Z  <span style="color: green">INFO</span> torrent_poc::torrent::torrent_actor:</span> TorrentActor removed connection to peer <span style="color: red">-Rp1121-8gJCKF636JRe</span>
   </pre>
   <pre>
   <b>$</b> cargo run -- leech --info-hash 018e50b58106b84a42c223ccf0494334f8d55958 --port 12345 --ip 127.0.0.1
   <span style="color: grey">2024-06-05T12:27:25.542483Z  <span style="color: green">INFO</span> torrent_poc:</span> My peer ID: <span style="color: red">-Rp1121-8gJCKF636JRe</span>
   <span style="color: grey">2024-06-05T12:27:25.543401Z  <span style="color: green">INFO</span> torrent_poc:</span> Connecting to peer at 127.0.0.1:12345
   <span style="color: grey">2024-06-05T12:27:25.543532Z  <span style="color: green">INFO</span> torrent_poc:</span> Info hash: <span style="color: purple">b6ae8e98e360a3d5d547dd43d42548ee786845ff</span>
   <span style="color: grey">2024-06-05T12:27:25.549016Z  <span style="color: green">INFO</span> torrent_poc::torrent::connection_actor:</span> Connection established with peer <span style="color: blue">-Rp1121-THMZfvNhcurL</span>
   <span style="color: grey">2024-06-05T12:27:25.549031Z  <span style="color: green">INFO</span> torrent_poc::torrent::torrent_actor:</span> TorrentActor added connection to peer <span style="color: blue">-Rp1121-THMZfvNhcurL</span>
   # 10 seconds later
   <span style="color: grey">2024-06-05T12:27:35.613681Z  <span style="color: goldenrod">WARN</span> torrent_poc::connections::std_io_connection:</span> error reading from the connection: Os { code: 10053, kind: ConnectionAborted, message: "An established connection was aborted by the software in your host machine." }
   </pre>