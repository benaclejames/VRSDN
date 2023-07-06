use std::io;
use std::net::TcpStream;
use crate::Serializable;
use std::io::{Read, Write};
use crate::chunk::chunk_headers::{ChunkBasicHeader, ChunkHeader};

pub struct RtmpSocket {
    socket: TcpStream,
}

impl io::Read for RtmpSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.read(buf)
    }
}

impl io::Write for RtmpSocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.socket.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

impl RtmpSocket {
    pub fn new(socket: TcpStream) -> Self {
        Self { socket }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.read(buf)
    }

    pub fn send_bytes(&mut self, msg: Vec<u8>, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) {
        let header = ChunkHeader {
            basic_header: ChunkBasicHeader {
                fmt: 0,
                csid: chunk_stream_id,
            },
            timestamp: 0,
            message_length: msg.len() as u32,
            message_type_id: type_id,
            message_stream_id,
        };

        match header.serialize() {
            Ok(buf) => {
                self.socket.write_all(&buf).unwrap();
                self.socket.write_all(&msg).unwrap();
            }
            Err(err) => {
                eprintln!("Error serializing chunk_headers header: {}", err);
            }
        }
    }

    pub fn send_message<S>(&mut self, msg: S, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) where S : Serializable {
        let data = match msg.serialize() {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Error serializing message: {}", err);
                return;
            }
        };

        self.send_bytes(data, chunk_stream_id, type_id, message_stream_id);
    }
}