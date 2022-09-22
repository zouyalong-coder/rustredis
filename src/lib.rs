pub mod client;
pub mod config;
pub mod conn;
pub mod error;
pub mod protocol;
pub mod server;
pub mod command;

// fn main() {
//     let conf = Config{ addr: "".to_string(), };
//     let mut server = Server::from_config(&conf).unwrap();
//     server.run();
// }
