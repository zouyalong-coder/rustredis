use core::time;
use std::{process, thread::sleep};

use tokio::net::TcpStream;
use std::io::Write;

#[tokio::main]
async fn main()  {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await.unwrap();
    let pid = process::id();
    for i in 0..20 {
        let mut buf = vec![0u8; 1024];
        sleep(time::Duration::from_millis(300));
        write!(&mut buf, "+get {}\n {}\r\n", pid, i);
        stream.writable().await.unwrap();
        stream.try_write(&buf).unwrap();
    }

}