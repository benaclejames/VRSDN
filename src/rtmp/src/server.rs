use std::net::TcpStream;
use std::io::{Cursor, ErrorKind, Read, Write};
use tokio::sync::mpsc::Sender;
use crate::Serializable;
use crate::command_message::{AMFCall, AMFMessage, PlayMessage};
use amf::amf0::Value::{String, Number};
use crate::chunk::chunk_headers::ChunkHeader;
use crate::chunk::chunk_wrangler::ChunkWrangler;
use crate::control_message::{SetChunkSize, SetPeerBandwidth, WindowAcknowledgementSize};
use crate::socket::RtmpSocket;
use crate::handshake::{CS0, CS1};
use crate::server::PublishingType::Live;

#[derive(Debug, PartialEq)]
pub enum PublishingType {
    Live,
    Play
}

pub struct RtmpConnection {
    pub socket: RtmpSocket,
    pub multiplexer: ChunkWrangler,
    pub sender: Sender<Vec<u8>>,
    pub publishing_type: Option<PublishingType>
}

impl RtmpConnection {
    pub fn new(stream: TcpStream, sender: Sender<Vec<u8>>) -> Self {
        let socket = RtmpSocket::new(stream);
        RtmpConnection {
            socket,
            multiplexer: ChunkWrangler::new(),
            sender,
            publishing_type: None
        }
    }

    pub fn handshake(&mut self) -> Result<u8, &'static str> {
        // Read 1 byte for the CS0 version
        let mut buf = [0; 1];
        let c0 = match self.socket.read_exact(&mut buf) {
            Ok(_) => {
                let c0 = CS0::deserialize(&mut buf.as_ref()).unwrap();

                c0
            }
            Err(err) => {
                eprintln!("Error reading CS0 version: {}", err);
                return Err("Error reading CS0 version");
            }
        };

        // Now read 1536 bytes for the CS1 chunk_headers
        let mut buf2 = vec![0; 1536];
        let c1: CS1 = match self.socket.read_exact(&mut buf2) {
            Ok(_) =>
            // Pass the buf2 as a mut reference to deserialize
                match CS1::deserialize(&mut buf2.as_slice()) {
                    Ok(c1) => c1,
                    Err(err) => {
                        eprintln!("Error reading CS1: {}", err);
                        return Err("Error reading CS1");
                    }
                }
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    eprintln!("Unexpected end of file while reading CS1");
                } else {
                    eprintln!("Error reading CS1: {}", err);
                }
                return Err("Error reading CS1")
            }
        };

        // Now we send our own bytes. One byte with the same version as cs0
        // then our own cs1 chunk_headers. We need to use the same timestamp as the client but random bytes
        // for the rest.
        let s0 = c0;
        let s1 = CS1 {
            timestamp: 1,
            zero: 0,
            random_bytes: (0..1528).map(|_| { rand::random::<u8>() }).collect(),
        };
        let s2 = CS1 {
            timestamp: c1.timestamp,
            zero: 0,
            random_bytes: c1.random_bytes
        };

        // Send our own S0, S1 and S2
        self.socket.write_all(&s0.serialize().unwrap()).unwrap();
        self.socket.write_all(&s1.serialize().unwrap()).unwrap();
        self.socket.write_all(&s2.serialize().unwrap()).unwrap();

        // Now we wait for the client to send their CS2
        let mut buf3 = vec![0; 1536];
        match self.socket.read_exact(&mut buf3) {
            Ok(_) =>
                match CS1::deserialize(&mut buf3.as_slice()) {
                    Ok(c2) => c2,
                    Err(err) => {
                        eprintln!("Error reading CS2: {}", err);
                        return Err("Error reading CS2");
                    }
                }
            Err(err) => {
                if err.kind() == ErrorKind::UnexpectedEof {
                    eprintln!("Unexpected end of file while reading CS2");
                } else {
                    eprintln!("Error reading CS2: {}", err);
                }
                return Err("Error reading CS2")
            }
        };

        Ok(3)
    }

    fn handle_command_message(&mut self, mut cursor: Cursor<&Vec<u8>>) {
        // First, we get the command name as a str
        let message = match AMFMessage::deserialize(&mut cursor) {
            Ok(message) => message,
            Err(err) => {
                eprintln!("Error reading AMF message: {}", err);
                return;
            }
        };

        match message.command_name.as_str() {
            "connect" => {
                match AMFCall::deserialize(&mut cursor) {
                    Ok(amf_call) => amf_call,
                    Err(err) => {
                        eprintln!("Error reading AMF call: {}", err);
                        return;
                    }
                };

                self.socket.send_message(WindowAcknowledgementSize { window_acknowledgement_size: 5000000 }, 2, 5, 0);
                self.socket.send_message(SetPeerBandwidth { window_acknowledgement_size: 5000000, limit_type: 1 }, 2, 6, 0);
                self.socket.send_message(SetChunkSize { chunk_size: 5000 }, 2, 1, 0);

                let response_header = AMFMessage {
                    transaction_id: 1.0,
                    command_name: "_result".to_string(),
                };

                let response_body = AMFCall {
                    command_object: vec![
                        ("fmsVer".to_string(), String("FMS/3,0,1,123".to_string())),
                        ("capabilities".to_string(), Number(31.0)),
                        ("mode".to_string(), String("live".to_string())),
                        ("objectEncoding".to_string(), Number(0.0)),
                    ],
                    additional_args: vec![
                        ("level".to_string(), String("status".to_string())),
                        ("code".to_string(), String("NetConnection.Connect.Success".to_string())),
                        ("description".to_string(), String("Connection succeeded.".to_string())),
                        ("objectEncoding".to_string(), Number(0.0)),
                    ],
                };

                self.socket.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);

                println!("Successfully responded to connect request")
            }
            "createStream" => {
                let response_header = AMFMessage {
                    transaction_id: message.transaction_id,
                    command_name: "_result".to_string(),
                };

                let response_body = AMFCall {
                    command_object: Vec::new(),
                    additional_args: Vec::new(),
                };

                self.socket.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);
            }
            "publish" => {
                // expect null amf object first
                match amf::Value::read_from(&mut cursor, amf::Version::Amf0) {
                    Ok(amf::Value::Amf0(amf::Amf0Value::Null)) => {}
                    _ => {
                        eprintln!("Error reading NULL AMF object");
                        return;
                    }
                };

                // Read string
                let publishing_name = match amf::Value::read_from(&mut cursor, amf::Version::Amf0) {
                    Ok(amf::Value::Amf0(amf::Amf0Value::String(string))) => string,
                    _ => {
                        eprintln!("Error reading publishing name");
                        return;
                    }
                };

                self.publishing_type = match amf::Value::read_from(&mut cursor, amf::Version::Amf0) {
                    Ok(amf::Value::Amf0(amf::Amf0Value::String(string))) => {
                        match string.to_lowercase().as_str() {
                            "live" => Option::Some(PublishingType::Live),
                            "play" => Option::Some(PublishingType::Play),
                            _ => Option::None
                        }
                    },
                    _ => {
                        eprintln!("Error reading publishing type");
                        return;
                    }
                };

                println!("Publishing name: {}", publishing_name);
                println!("Publishing type: {:?}", self.publishing_type);

                // If the publishing type is "live" then we need to open a tx rx pair
                if self.publishing_type == Some(Live) {
                    let response_header = AMFMessage {
                        transaction_id: 0.0,
                        command_name: "onStatus".to_string(),
                    };

                    let response_body = AMFCall {
                        command_object: Vec::new(),
                        additional_args: vec![
                            ("code".to_string(), (String("NetStream.Publish.Start".to_string()))),
                            ("level".to_string(), (String("status".to_string()))),
                            ("description".to_string(), (String("Started publishing stream.".to_string()))),
                        ]
                    };

                    self.socket.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);
                }
            }
            "play" => {
                match PlayMessage::deserialize(&mut cursor) {
                    Ok(msg) => {
                        println!("{:?}", msg);
                        // First, we set our chunk_headers size to the max chunk_headers size
                    }
                    Err(err) => {
                        eprintln!("Error deserializing play message: {}", err);
                    }
                }
            }
            _ => {
                println!("Unsupported command: {}", message.command_name);
            }
        }
    }

    pub fn handle_control_stream_msg(&mut self, header: ChunkHeader, buf: &Vec<u8>) {
        println!("Received control stream message");
        match header.message_type_id {
            1 => {
                println!("Set chunk_headers size");
                self.multiplexer.max_chunk_size = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
                println!("New max chunk_headers size: {}", self.multiplexer.max_chunk_size);
            }
            2 => {
                // Used to signify to the peer that further processing is not necessary and that the stream is probably about to close.
                println!("Abort Message");
                let csid: u32 = u32::from_be_bytes(buf[0..4].try_into().unwrap());
                println!("Chunk Stream {} is aborting", csid);
            }
            3 => {
                // Sent by the client and used by the peer to acknowledge the number of bytes recv'd as specified in the window ack size
                println!("Acknowledgement");
                let sequence_number: u32 = u32::from_be_bytes(buf[0..4].try_into().unwrap());
                println!("Peer has recv'd {} bytes total thus far", sequence_number);

            }
            5 => {
                match WindowAcknowledgementSize::deserialize(&mut Cursor::new(&buf)) {
                    Ok(msg) => {
                        println!("{:?}", msg);
                    }
                    Err(err) => {
                        eprintln!("Error deserializing window acknowledgement size: {}", err);
                    }
                }
            }
            6 => {
                println!("Set Peer Bandwidth");
                match SetPeerBandwidth::deserialize(&mut Cursor::new(&buf)) {
                    Ok(msg) => {
                        println!("{:?}", msg)
                    }
                    Err(err) => {
                        eprintln!("Error deserializing set peer bandwidth: {}", err);
                    }
                }
            }
            _ => {
                println!("Unsupported control stream message type: {}", header.message_type_id);
            }
        }
    }

    pub async fn handle_connection(&mut self) {
        match self.handshake() {
            Ok(_) => {
                println!("Shook the fuk outta that hand");
            }
            Err(err) => {
                eprintln!("Handshake failed: {}", err);
                return;
            }
        }

        // Now while our connection is open, we read chunk_headers by chunk_headers
        // and print the data we receive
        loop {
            let (header, data) = match self.multiplexer.read_chunk(&mut self.socket) {
                Ok((header, data)) => (header, data),
                Err(_) => {
                    continue;
                }
            };

            if header.message_type_id == 20 {
                self.handle_command_message(Cursor::new(&data));
                continue;
            }
            // If this message is targeting the control stream, we need to parse it properly
            if header.message_stream_id == 0 {
                self.handle_control_stream_msg(header, &data);
                continue;
            }
        }
    }
}