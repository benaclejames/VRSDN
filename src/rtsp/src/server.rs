use std::io::Read;
use std::net::TcpStream;

pub struct RtspConnection {
    socket: TcpStream
}

impl RtspConnection {
    pub fn new(socket: TcpStream) -> RtspConnection {
        RtspConnection {
            socket
        }
    }

    pub fn handle_connection(&mut self) {
        // Read as many bytes as we can from the socket
        let mut buffer = [0; 1024];
        self.socket.read(&mut buffer).unwrap();
        println!("Request: {}", String::from_utf8_lossy(&buffer[..]));
    }
}