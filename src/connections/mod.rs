use eyre::Result;

use crate::messages::Message;

pub mod std_io_connection;

pub trait Connection {
    fn send(&mut self, message: Message) -> Result<()>;
    fn receive(&self) -> Result<Message>;
}
