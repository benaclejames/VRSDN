use std::io::Read;
use std::net::TcpListener;

mod server;

pub trait Serializable {
    fn serialize(&self) -> Result<Vec<u8>, &'static str>;
    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R : Read, Self: Sized;
}

pub struct RtspServer {

}

impl RtspServer {
    pub fn new() -> RtspServer {
        RtspServer {
        }
    }

    pub fn start(&mut self) {
        // Start a TCP server
        let listener = TcpListener::bind("127.0.0.1:554").unwrap();

        for stream in listener.incoming() {
            let stream = stream.unwrap();
            println!("Connection established!");
            let mut connection = server::RtspConnection::new(stream);
            // Start a thread to handle the connection, and pass a reference to ourselves
            std::thread::spawn(move || {
                connection.handle_connection();
            });
        }
    }
}