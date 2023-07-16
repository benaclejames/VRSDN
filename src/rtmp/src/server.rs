pub use crate::handshake::{CS0, CS1};
use std::net::TcpStream;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use crate::Serializable;
use crate::command_message::{AMFCall, AMFMessage, PlayMessage};
use amf::amf0::Value::{String, Number};
use crate::chunk::chunk_container::ChunkContainer;
use crate::chunk::chunk_headers::ChunkHeader;
use crate::chunk::chunk_router::ChunkRouter;
use crate::chunk::chunk_wrangler::ChunkWrangler;
use crate::control_message::{SetChunkSize, SetPeerBandwidth, WindowAcknowledgementSize};
use crate::handshake::handshake;
use crate::socket::RtmpSocket;

pub struct RtmpConnection {
    pub chunk_router: Arc<Mutex<ChunkRouter>>,
    pub socket: RtmpSocket,
    pub multiplexer: ChunkWrangler,
}

impl RtmpConnection {
    pub fn new(chunk_router: Arc<Mutex<ChunkRouter>>, stream: TcpStream, sender: Sender<ChunkContainer>) -> Self {
        let socket = RtmpSocket::new(stream);
        RtmpConnection {
            chunk_router,
            socket,
            multiplexer : ChunkWrangler::new(),
        }
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

                let publishing_type = match amf::Value::read_from(&mut cursor, amf::Version::Amf0) {
                    Ok(amf::Value::Amf0(amf::Amf0Value::String(string))) => string,
                    _ => {
                        eprintln!("Error reading publishing type");
                        return;
                    }
                };

                println!("Publishing name: {}", publishing_name);
                println!("Publishing type: {}", publishing_type);

                // If the publishing type is "live" then we need to open a tx rx pair
                if publishing_type == "live" {
                    //let (tx, rx) = channel();
                }

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
            "play" => {
                match PlayMessage::deserialize(&mut cursor) {
                    Ok(msg) => {
                        println!("Play message");
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
            5 => {
                println!("Window acknowledgement size");
                match WindowAcknowledgementSize::deserialize(&mut Cursor::new(&buf)) {
                    Ok(msg) => {
                        println!("New window acknowledgement size: {}", msg.window_acknowledgement_size);
                    }
                    Err(err) => {
                        eprintln!("Error deserializing window acknowledgement size: {}", err);
                    }
                }
            }
            _ => {
                println!("Unsupported control stream message type: {}", header.message_type_id);
            }
        }
    }

    pub fn handle_connection(&mut self) {
        match handshake(self) {
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
                Err(err) => {
                    continue;
                }
            };

            if header.message_type_id == 8 {
                println!("{:?}", data)
            }

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