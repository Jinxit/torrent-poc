use std::cmp::min;
use std::io::{Read, Write};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::Arc;

use eyre::Result;
use eyre::WrapErr;
use tracing::{error, warn};

use crate::messages::{DecodedMessage, Message};
use crate::{ConnectionRead, ConnectionWrite, SansIo};

// 64 kB * 10 messages => at most 640 kB per connection
// In practice the first connection causes the application to allocate about ~10mB of memory,
// but after that even malicious connections actually use a lot less than 640 kB each.
const MAX_BUFFER_SIZE: usize = 64 * 1024;
const MAX_BUFFERED_MESSAGES: usize = 10;

/// A [ConnectionRead] implementation built on top of [std::io::Read].
pub struct StdIoConnectionRead {
    receiver: Receiver<Message>,
    #[allow(dead_code)]
    state: Arc<ConnectionState>,
}

/// A [ConnectionWrite] implementation built on top of [std::io::Write].
pub struct StdIoConnectionWrite<W> {
    writer: W,
    #[allow(dead_code)]
    state: Arc<ConnectionState>,
}

/// Create a Connection built on top of [std::io::Read] and [std::io::Write].
pub fn std_io_connection<R, W>(
    initial_buffer_size: usize,
    reader: R,
    writer: W,
) -> (StdIoConnectionWrite<W>, StdIoConnectionRead)
where
    R: Read + Send + 'static,
    W: Write,
{
    let (sender, receiver) = std::sync::mpsc::sync_channel(MAX_BUFFERED_MESSAGES);
    let state = Arc::new(ConnectionState::new());
    // Letting this thread die on shutdown is fine, since the connection doesn't directly write
    // to disk or anything, it's just a buffer that then communicates with the actors.
    let _ = std::thread::spawn({
        let state = state.clone();
        move || receive_loop(initial_buffer_size, reader, sender, state)
    });
    let write = StdIoConnectionWrite {
        writer,
        state: state.clone(),
    };
    let read = StdIoConnectionRead { receiver, state };
    (write, read)
}

fn receive_loop<R: Read>(
    initial_buffer_size: usize,
    mut reader: R,
    sender: SyncSender<Message>,
    _state: Arc<crate::connections::std_io_connection::ConnectionState>,
) {
    let mut buffer = vec![255; initial_buffer_size];
    let mut buffer_offset = 0;
    'thread: loop {
        'message: loop {
            let bytes_read = match reader.read(&mut buffer[buffer_offset..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => {
                    warn!("error reading from the connection: {:?}", e);
                    break 'thread;
                }
            };

            if bytes_read == 0 {
                break 'thread;
            }

            let opt_message =
                match Message::from_partial_buffer(&buffer[..buffer_offset + bytes_read]) {
                    Ok(opt_message) => opt_message,
                    Err(e) => {
                        error!("unexpected error decoding a message: {:?}", e);
                        break 'thread;
                    }
                };

            if let Some(DecodedMessage {
                consumed_bytes,
                message,
            }) = opt_message
            {
                // Reset the buffer, but keep the bytes we didn't consume.
                // This could probably be done more efficiently, perhaps with a separate offset
                // or using virtual memory tricks, but eh.
                buffer.copy_within(consumed_bytes.., 0);
                buffer_offset = buffer_offset + bytes_read - consumed_bytes;
                if sender.try_send(message.clone()).is_err() {
                    warn!("Receiver is full, waiting");
                    if sender.send(message).is_err() {
                        // The receiver is gone, we're probably about to exit; stop the thread
                        break 'thread;
                    }
                }
                break 'message;
            } else {
                // Either the buffer wasn't big enough to hold the message...
                if buffer.len() - buffer_offset == bytes_read {
                    if buffer.len() == MAX_BUFFER_SIZE {
                        // This client seems malicious, no messages should be this big.
                        // Let's not use up all the available memory.
                        break 'thread;
                    }

                    // Grow the buffer and try again.
                    // `255` here is not a requirement, but it makes debugging easier.
                    let mut new_buffer = vec![255; min(buffer.len() * 2, MAX_BUFFER_SIZE)];
                    new_buffer[..buffer_offset + bytes_read]
                        .copy_from_slice(&buffer[..buffer_offset + bytes_read]);
                    buffer_offset += bytes_read;
                    buffer = new_buffer;
                } else {
                    // ...or the message was incomplete, just try again.
                    buffer_offset += bytes_read;
                }
            }
        }
    }
}

impl ConnectionRead for StdIoConnectionRead {
    fn receive(&self) -> Result<Message> {
        self.receiver
            .recv()
            .wrap_err("Connection closed, no more messages coming")
    }
}

impl<W: Write> ConnectionWrite for StdIoConnectionWrite<W> {
    fn send(&mut self, message: Message) -> Result<()> {
        self.writer.write_all(&message.encode())?;
        // TODO: excessive flushing might not be a good idea, figure it out later
        self.writer.flush()?;
        Ok(())
    }
}

#[allow(dead_code)]
struct ConnectionState {
    am_choking: AtomicBool,
    am_interested: AtomicBool,
    peer_choking: AtomicBool,
    peer_interested: AtomicBool,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            am_choking: AtomicBool::new(true),
            am_interested: AtomicBool::new(false),
            peer_choking: AtomicBool::new(true),
            peer_interested: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;
    use std::io;
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    use crate::messages::Handshake;
    use crate::{InfoHash, PeerId};

    use super::*;

    #[derive(Debug, Default, Clone)]
    struct MockReader {
        responses: Arc<Vec<Vec<u8>>>,
        reads: Arc<Mutex<Vec<usize>>>,
        current_index: usize,
        current_offset: usize,
    }

    impl MockReader {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses: Arc::new(responses),
                current_index: 0,
                current_offset: 0,
                reads: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl Read for MockReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.current_index >= self.responses.len() {
                return Ok(0);
            }
            let source = &self.responses[self.current_index][self.current_offset..];
            let limit = min(buf.len(), source.len());

            buf[..limit].copy_from_slice(
                &self.responses[self.current_index]
                    [self.current_offset..self.current_offset + limit],
            );
            self.reads.lock().unwrap().push(limit);

            if limit == source.len() {
                self.current_index += 1;
                self.current_offset = 0;
            } else {
                self.current_offset += limit;
            }
            Ok(limit)
        }
    }

    #[derive(Debug, Default, Clone)]
    struct MockWriter {
        responses: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.responses.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            // let's represent flushing as an empty vec
            self.responses.lock().unwrap().push(vec![]);
            Ok(())
        }
    }

    #[test]
    fn test_send_ok() {
        let writer = MockWriter::default();
        let reader = MockReader::default();
        let (mut connection_write, _) = std_io_connection(1024, reader.clone(), writer.clone());
        let handshake = Handshake::new(InfoHash::new([1; 20]), PeerId::new([2; 20]));

        connection_write
            .send(Message::Handshake(handshake))
            .unwrap();

        assert_eq!(
            *writer.responses.lock().unwrap(),
            vec![handshake.encode(), vec![]]
        );
    }

    #[test]
    fn test_receive_within_buffer_size() {
        let writer = MockWriter::default();
        let handshake = Handshake::new(InfoHash::new([1; 20]), PeerId::new([2; 20]));
        let reader = MockReader::new(vec![handshake.encode()]);
        let (_, connection_read) = std_io_connection(1024, reader.clone(), writer.clone());

        let message = connection_read.receive().unwrap();

        assert_eq!(message, Message::Handshake(handshake));
        assert_eq!(*reader.reads.lock().unwrap(), vec![68]);
    }

    #[test]
    fn test_receive_outside_buffer_size() {
        let writer = MockWriter::default();
        let handshake = Handshake::new(InfoHash::new([11; 20]), PeerId::new([22; 20]));
        let reader = MockReader::new(vec![handshake.encode()]);
        let (_, connection_read) = std_io_connection(1, reader.clone(), writer.clone());

        let message = connection_read.receive().unwrap();

        assert_eq!(message, Message::Handshake(handshake));
        assert_eq!(
            *reader.reads.lock().unwrap(),
            vec![1, 1, 2, 4, 8, 16, 32, 4]
        );
    }

    #[test]
    fn test_receive_incomplete_message() {
        let writer = MockWriter::default();
        let handshake = Handshake::new(InfoHash::new([11; 20]), PeerId::new([22; 20]));
        let handshake_bytes = handshake.encode();

        let split_point = 30;

        let reader = MockReader::new(vec![
            handshake_bytes[..split_point].to_vec(),
            handshake_bytes[split_point..].to_vec(),
        ]);
        let (_, connection_read) = std_io_connection(1024, reader.clone(), writer.clone());

        let message = connection_read.receive().unwrap();

        assert_eq!(message, Message::Handshake(handshake));
        assert_eq!(
            *reader.reads.lock().unwrap(),
            vec![split_point, handshake_bytes.len() - split_point]
        );
    }

    #[test]
    fn test_receive_two_incomplete_messages() {
        let writer = MockWriter::default();
        let handshake1 = Handshake::new(InfoHash::new([11; 20]), PeerId::new([22; 20]));
        let handshake1_bytes = handshake1.encode();
        let handshake2 = Handshake::new(InfoHash::new([33; 20]), PeerId::new([44; 20]));
        let handshake2_bytes = handshake2.encode();

        let split_point = 30;
        let mut part1_bytes = handshake1_bytes;
        part1_bytes.extend(handshake2_bytes[..split_point].to_vec());
        let part2_bytes = handshake2_bytes[split_point..].to_vec();

        let reader = MockReader::new(vec![part1_bytes.clone(), part2_bytes.clone()]);
        let (_, connection_read) = std_io_connection(1024, reader.clone(), writer.clone());

        let message1 = connection_read.receive().unwrap();
        let message2 = connection_read.receive().unwrap();

        assert_eq!(message1, Message::Handshake(handshake1));
        assert_eq!(message2, Message::Handshake(handshake2));
        assert_eq!(
            *reader.reads.lock().unwrap(),
            vec![part1_bytes.len(), part2_bytes.len()]
        );
    }

    #[test]
    fn test_receive_unknown_message() {
        let writer = MockWriter::default();

        // id 15 is not a valid message type
        // length of the message is 4 + 1 + 4 = 9, but the length is encoded as a u32, so split it
        let reader = MockReader::new(vec![[0, 0, 0, 9, 15].to_vec(), b"test".to_vec()]);
        let (_, connection_read) = std_io_connection(1024, reader.clone(), writer.clone());

        let _ = connection_read.receive().unwrap_err();
    }
}
