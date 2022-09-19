extern crate rustredis;
// #[macro_use]
// extern crate log;

use log::{info, warn};
use rustredis::config::Config;
use rustredis::server::Server;

fn main() {
    env_logger::init();
    // Builder::new()
    //     .parse_env(&env::var("LOG").unwrap_or_default())
    //     .init();
    let conf = Config{ addr: "127.0.0.1:6379".to_string(), };
    let mut server = Server::from_config(&conf).unwrap();
    warn!("warnning");
    info!("run server now");
    server.run();
}