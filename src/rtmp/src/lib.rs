use std::io;
use std::net::TcpListener;

mod server;
mod handshake;
mod chunk;
mod control_message;
mod command_message;

trait Serializable {
    fn serialize(&self) -> Result<Vec<u8>, &'static str>;
    fn deserialize<R>(reader: R) -> Result<Self, &'static str> where R : io::Read, Self: Sized;
}

pub fn start() {
    // Start a TCP server
    let listener = TcpListener::bind("127.0.0.1:1935").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("Connection established!");
        let mut connection = server::RtmpConnection::new(stream);
        connection.handle_connection();
    }
}
