use bytes::{Buf, Bytes, BytesMut};
use futures::select;
use log::{debug, info, warn};
use std::io::Cursor;
use std::{
    io::{self},
    pin::Pin,
};
use tokio::io::{self as tio, AsyncReadExt, ReadHalf, WriteHalf};

use rax::RaxMap;
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::{self, Runtime},
};
use tokio_stream::Stream;
use tokio_stream::{StreamExt, StreamMap};

use crate::client::Client;
use crate::error::Result;
use crate::{config::Config, error::Error, frame::Command};
use async_stream::try_stream;

pub struct Server<'a> {
    // rt: Runtime,
    addr: String,
    running: bool,
    id_slots: bitmaps::Bitmap<1024>,
    /// 当前连接的 clients
    clients: RaxMap<u64, Client>,
    streams: StreamMap<u64, Pin<Box<dyn Stream<Item = (&'a Client, Command)> + Send>>>,
}

impl Server<'_> {
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
        let id = self
            .id_slots
            .first_false_index()
            .and_then(|id| Some(id as u64))?;
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
        let rs: Pin<Box<dyn Stream<Item = (&Client, Command)> + Send>> =
            Box::pin(async_stream::stream! {
                while let Some(cmd) = client.read_command().await {
                    yield (&client, cmd);
                }
            });

        // self.clients.insert(id, Box::new(client));
        self.streams.insert(id, rs);
    }

    fn handle_command(&mut self, client_id: u64, client: &Client, cmd: Command) {
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
        let accept_stream = Self::bind_and_accept(self.addr.clone());
        tokio::pin!(accept_stream);
        loop {
            tokio::select! {
                Some(v) = accept_stream.next() => {
                    self.on_client_created(v.unwrap());
                }
                Some((id, (client, cmd))) = self.streams.next() => {
                    self.handle_command(id, client, cmd);
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
