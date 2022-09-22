use log::{debug, info, warn};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, sync::Mutex};

use rax::RaxMap;
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::{self, Runtime},
};

use crate::client::Client;
use crate::config::Config;
use crate::error::Result;

struct IdGen {
    id_slots: bitmaps::Bitmap<1024>,
}

impl IdGen {
    fn new() -> Self {
        let mut id_slots = bitmaps::Bitmap::new();
        id_slots.set(0, true);
        Self { id_slots }
    }

    fn new_id(&mut self) -> Option<u64> {
        let id = self
            .id_slots
            .first_false_index()
            .and_then(|id| Some(id as u64))?;
        self.id_slots.set(id as usize, true);
        if id > 1 {
            return None;
        }
        Some(id)
    }

    fn recycle_id(&mut self, id: u64) {
        self.id_slots.set(id as usize, false);
    }
}

pub struct Server {
    rt: Runtime,
    addr: String,
    running: bool,
    id_gen: Arc<Mutex<IdGen>>,
    // clients: RaxMap<u64, Client>,
    // streams: StreamMap<u64, Pin<Box<dyn Stream<Item = (Command, &mut Client)>>>>,
}

impl Server {
    pub fn from_config(conf: &Config) -> Result<Self> {
        let id_gen = Arc::new(Mutex::new(IdGen::new()));
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        Ok(Self {
            rt,
            id_gen,
            addr: conf.addr.clone(),
            running: true,
            // clients: RaxMap::new(),
        })
    }

    async fn on_client_created(&self, mut stream: TcpStream, id_gen: Arc<Mutex<IdGen>>) {
        let id = id_gen.lock().await.new_id();
        if id.is_none() {
            warn!("too many client");
            // stream.shutdown().await;
            // stream will be dropped here and the connection will be closed.
            return;
        }
        let id = id.unwrap();
        let mut client = Client::new(id, stream);
        self.rt.spawn(async move {
            while let Some(cmd) = client.read_command().await {
                debug!("Got cmd by client({:?}): {:?}", id, cmd);
                client.execute_command(cmd);
            }
            debug!("Client {:?} exits", id);
            id_gen.lock().await.recycle_id(id);
        });
    }

    async fn do_run(&self) -> Result<()> {
        info!("server starts");
        // let mut id_gen = Arc::new(Mutex::new(IdGen::new()));
        let listener = TcpListener::bind(self.addr.clone()).await?;
        while self.running {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("received on {:?}", addr);
                    self.on_client_created(stream, self.id_gen.clone()).await;
                }
                Err(err) => {
                    warn!("error on accepting new socket: {:?}", err);
                }
            }
        }
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        info!("server runs now");
        self.rt.block_on(self.do_run())
    }
}
