use std::io::BufRead;

use bytes::{Buf, BytesMut};
use log::{debug, info, warn};
use tokio::{
    io::{AsyncReadExt, BufReader},
    net::TcpStream,
};

use crate::command::Command;

pub struct Client {
    id: u64,
    stream: BufReader<TcpStream>,
    flags: u32,
    // rs: ReadHalf<TcpStream>,
    // ws: WriteHalf<TcpStream>,
}

impl Client {
    pub fn new(id: u64, stream: TcpStream) -> Self {
        // let (rs, ws) = tio::split(stream);
        Self {
            id,
            stream: BufReader::new(stream),
            flags: 0,
        }
    }

    pub async fn read_command(&mut self) -> Option<Command> {
        loop {
            match Command::from_resp2(&mut self.stream).await {
                Ok(cmd) => {return Some(cmd)},
                Err(err) => {
                    match err {
                        crate::error::Error::EOF => {
                            return None
                        },
                        _ => {
                            warn!("error on reading command: {:?}", err)
                        }
                    }
                },
            }
        }
        // let cmd = Command::from_resp2(&mut self.stream).await;

        // loop {
        //     info!("read_command now");
        //     if let Some(cmd) = self.parse_command() {
        //         return Some(cmd);
        //     }
        //     if 0 == self.stream.read_buf(&mut self.buffer).await.unwrap() {
        //         if self.buffer.is_empty() {
        //             return None;
        //         } else {
        //             return Some(Command::Quit);
        //         }
        //     }
        // }
    }

    pub fn execute_command(&self, cmd: Command) {}


}
