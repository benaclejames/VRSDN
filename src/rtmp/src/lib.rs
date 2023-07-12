use std::collections::HashMap;
use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc::channel;
use crate::server::RtmpConnection;

mod server;
mod handshake;
mod chunk;
mod control_message;
mod command_message;
mod socket;

pub trait Serializable {
    fn serialize(&self) -> Result<Vec<u8>, &'static str>;
    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R : Read, Self: Sized;
}

pub struct RtmpServer {

}

impl RtmpServer {
    pub fn new() -> RtmpServer {
        RtmpServer {
        }
    }

    pub fn start(&mut self) {
        // Start a TCP server
        let listener = TcpListener::bind("127.0.0.1:1935").unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            println!("Connection established!");
            let mut connection = server::RtmpConnection::new(stream);
            // Start a thread to handle the connection, and pass a reference to ourselves
            std::thread::spawn(move || {
                connection.handle_connection();
            });
        }
    }
}