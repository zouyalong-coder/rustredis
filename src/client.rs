use bytes::{BytesMut, Buf};
use log::{info, debug};
use tokio::{net::TcpStream, io::AsyncReadExt};

use crate::frame::Command;


pub struct Client {
    id: u64,
    stream: TcpStream,
    // rs: ReadHalf<TcpStream>,
    // ws: WriteHalf<TcpStream>,
    buffer: BytesMut,
}

impl Client {
    pub fn new(id: u64, stream: TcpStream) -> Self {
        // let (rs, ws) = tio::split(stream);
        Self { id, stream, buffer: BytesMut::new(), }
    }

    pub async fn read_command(&mut self) -> Option<Command> {
        loop {
            info!("read_command now");
            if let Some(cmd) = self.parse_command() {
                return Some(cmd);
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await.unwrap() {
                if self.buffer.is_empty() {
                    return None;
                } else {
                    return Some(Command::Quit)
                }
            }
        }
    }

    pub fn execute_command(&self, cmd: Command) {

    }

    fn parse_command(&mut self) -> Option<Command> {
        if self.buffer.is_empty() {
            return None;
        }
        let buf = self.buffer.to_vec();
        // let mut buf = Cursor::new(&self.buffer[..]);
        debug!("Got buf {:?}", buf);
        self.buffer.advance(buf.len());
        Some(Command::Get { key: unsafe{String::from_utf8_unchecked(buf) }})
    }
}
