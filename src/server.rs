use std::{io::{self}, pin::Pin};
use std::io::Cursor;
use futures::{select};
use tokio::io::{self as tio, ReadHalf, WriteHalf, AsyncReadExt};
use log::{info, debug, warn};
use bytes::{Bytes, Buf, BytesMut};


use rax::RaxMap;
use tokio::{runtime::{Runtime, self}, net::{TcpListener, TcpStream}};
use tokio_stream::{StreamMap, StreamExt};
use tokio_stream::Stream;

use crate::{config::Config, frame::Command, error::Error};
use crate::error::Result;
use async_stream::try_stream;

struct Client {
    id: u64,
    stream: TcpStream,
    // rs: ReadHalf<TcpStream>,
    // ws: WriteHalf<TcpStream>,
    buffer: BytesMut,
}

impl Client {
    fn new(id: u64, stream: TcpStream) -> Self {
        // let (rs, ws) = tio::split(stream);
        Self { id, stream, buffer: BytesMut::new(), }
    }

    async fn read_command(&mut self) -> Option<Command> {
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

    fn execute_command(&self, cmd: Command) {

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


pub struct Server {
    // rt: Runtime,
    addr: String,
    running: bool,
    id_slots: bitmaps::Bitmap<1024>,
    /// 当前连接的 clients
    clients: RaxMap<u64, Client>,
    streams: StreamMap<u64, Pin<Box<dyn Stream<Item = Command> + Send>>>,
}

impl Server {
    pub fn from_config(conf: &Config) -> Result<Self> {
        let mut id_slots = bitmaps::Bitmap::new();
        id_slots.set(0, true);
        Ok(Self {
            addr: conf.addr.clone(),
            running: false,
            id_slots,
            clients: RaxMap::new(),
            streams: StreamMap::new(),
        })
    }

    fn new_client_id(&mut self) -> Option<u64> {
        let id = self.id_slots.first_false_index().and_then(|id| Some(id as u64))?;
        self.id_slots.set(id as usize, true);
        Some(id)
    }

    fn bind_and_accept(addr: String) -> impl Stream<Item = io::Result<TcpStream>> {
        try_stream! {
            let listener = TcpListener::bind(addr).await?;
    
            loop {
                let (stream, addr) = listener.accept().await?;
                println!("received on {:?}", addr);
                yield stream;
            }
        }
    }

    fn on_client_created(&mut self, stream: TcpStream) {
        let id = self.new_client_id();
        if id.is_none() {
            warn!("too many client");
            return;
        }
        let id = id.unwrap();
        let mut client = Client::new(id, stream);
        let rs: Pin<Box<dyn Stream<Item = Command> + Send>> = Box::pin(async_stream::stream! {
            while let Some(cmd) = client.read_command().await {
                yield cmd;
            }
        });
        // self.clients.insert(id, Box::new(client));
        self.streams.insert(id, rs);
    }

    fn handle_command(&mut self, client_id: u64, cmd: Command) {
        debug!("client({}) => cmd {:?}", client_id, cmd);
        let client = self.clients.find(client_id);
        if client.is_none() {
            warn!("invalid client id: {:?}", client_id);
            return;
        }
        let client = client.unwrap();
        client.execute_command(cmd);
    }

    async fn do_run(&mut self) -> ! {
        info!("server starts");
        let mut accept_stream = Box::pin(Self::bind_and_accept(self.addr.clone()));
        loop {
            tokio::select! {
                Some(v) = accept_stream.next() => {
                    self.on_client_created(v.unwrap());
                }
                Some((id, cmd)) = self.streams.next() => {
                    self.handle_command(id, cmd);
                }
            }
        }
    }

    pub fn run(&mut self) {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        info!("server runs now");
        rt.block_on(self.do_run())
    }

    // pub fn run(&mut self) -> io::Result<()> {
    //     self.rt.block_on(async {
    //         let listener = TcpListener::bind(self.addr).await?;
            
    //     })
    // }
}


