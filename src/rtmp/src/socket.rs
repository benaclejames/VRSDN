use std::io;
use crate::Serializable;
use std::io::{Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use crate::chunk::chunk_headers::{ChunkBasicHeader, ChunkHeader};

pub struct RtmpSocket {
    pub socket: TcpStream,
}

impl RtmpSocket {
    pub fn new(socket: TcpStream) -> Self {
        Self { socket }
    }

    pub async fn send_bytes(&mut self, msg: Vec<u8>, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) {
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
                self.socket.write_all(&buf).await;
                self.socket.write_all(&msg).await;
            }
            Err(err) => {
                eprintln!("Error serializing chunk_headers header: {}", err);
            }
        }
    }

    pub async fn send_message<S>(&mut self, msg: S, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) where S : Serializable {
        let data = match msg.serialize() {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Error serializing message: {}", err);
                return;
            }
        };

        self.send_bytes(data, chunk_stream_id, type_id, message_stream_id).await;
    }
}