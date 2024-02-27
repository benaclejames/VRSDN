use std::io::Read;
use std::net::TcpListener;
use tokio::sync::mpsc::{channel};
use std::sync::{Arc, Mutex};
use crate::chunk::chunk_router::ChunkRouter;

mod server;
mod handshake;
mod chunk;
mod control_message;
mod command_message;
mod socket;

pub trait Serializable {
    fn serialize(&self) -> Result<Vec<u8>, &'static str>;
    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str>
        where
            R: Read,
            Self: Sized;
}

pub struct RtmpServer {
    pub chunk_router: Arc<Mutex<ChunkRouter>>,
}

impl RtmpServer {
    pub fn new() -> RtmpServer {
        RtmpServer {
            chunk_router: Arc::new(Mutex::new(ChunkRouter::new())),
        }
    }

    pub async fn start(&mut self) {
        // Start a TCP server
        let listener = TcpListener::bind("127.0.0.1:1935").unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            println!("Connection established!");
            let (tx, rx) = channel(9999);
            self.chunk_router.lock().unwrap().recievers.insert("test".to_string(), rx);
            let mut connection = server::RtmpConnection::new(stream, tx);
            // Start a thread to handle the connection, and pass a reference to ourselves
            tokio::spawn(async move {
                connection.handle_connection().await;
            });
        }
    }
}
