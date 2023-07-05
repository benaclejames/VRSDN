use std::collections::HashMap;
pub use crate::handshake::{CS0, CS1};
use std::net::TcpStream;
use std::io::{Read, ErrorKind, Write, Cursor};
use crate::chunk::{ChunkBasicHeader, ChunkHeader};
use crate::Serializable;
use crate::control_message::{SetChunkSize, SetPeerBandwidth, WindowAcknowledgementSize};
use crate::command_message::{AMFCall, AMFMessage, PlayMessage};
use amf::amf0::Value::{String, Number};
use crate::handshake::handshake;

pub struct RtmpConnection {
    pub stream: TcpStream,
    max_chunk_size: usize,
    incomplete_chunks: HashMap<u8, Vec<u8>>,
}

impl RtmpConnection {
    pub fn new(stream: TcpStream) -> Self {
        RtmpConnection {
            stream,
            max_chunk_size: 128,
            incomplete_chunks: HashMap::new(),
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

                self.send_message(WindowAcknowledgementSize { window_acknowledgement_size: 5000000 }, 2, 5, 0);
                self.send_message(SetPeerBandwidth { window_acknowledgement_size: 5000000, limit_type: 1 }, 2, 6, 0);
                self.send_message(SetChunkSize { chunk_size: 5000 }, 2, 1, 0);

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

                self.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);

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

                self.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);
            }
            "publish" => {
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

                self.send_bytes([response_header.serialize().unwrap(), response_body.serialize().unwrap()].concat(), 3, 20, 0);
            }
            "play" => {
                match PlayMessage::deserialize(&mut cursor) {
                    Ok(msg) => {
                        println!("Play message");
                        // First, we set our chunk size to the max chunk size
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

    fn handle_control_stream_msg(&mut self, header: ChunkHeader, buf: &Vec<u8>) {
        println!("Received control stream message");
        match header.message_type_id {
            1 => {
                println!("Set chunk size");
                self.max_chunk_size = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
                println!("New max chunk size: {}", self.max_chunk_size);
            }
            5 => {
                println!("Window acknowledgement size");
                match WindowAcknowledgementSize::deserialize(&buf[..]) {
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

    fn send_bytes(&mut self, msg: Vec<u8>, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) {
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
                self.stream.write_all(&buf).unwrap();
                self.stream.write_all(&msg).unwrap();
            }
            Err(err) => {
                eprintln!("Error serializing chunk header: {}", err);
            }
        }
    }

    fn send_message<S>(&mut self, msg: S, chunk_stream_id: u8, type_id: u8, message_stream_id: u32) where S : Serializable {
        let data = match msg.serialize() {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Error serializing message: {}", err);
                return;
            }
        };

        self.send_bytes(data, chunk_stream_id, type_id, message_stream_id);
    }

    pub fn handle_connection(&mut self) {
        println!("Handling connection from {}", self.stream.peer_addr().unwrap());

        match handshake(self) {
            Ok(_) => {
                println!("Shook the fuk outta that hand");
            }
            Err(err) => {
                eprintln!("Handshake failed: {}", err);
                return;
            }
        }

        // Now while our connection is open, we read chunk by chunk
        // and print the data we receive
        loop {
            let header = match ChunkHeader::deserialize(&self.stream) {
                Ok(header) => header,
                Err(err) => {
                    eprintln!("Error reading chunk header: {}", err);
                    return;
                }
            };

            // If this csid exists in the incomplete messages hashmap, we need to get the remaining bytes to read to complete the msg and pass it into the min
            let mut chunk_size = std::cmp::min(self.max_chunk_size, header.message_length as usize);
            if self.incomplete_chunks.contains_key(&header.basic_header.csid) {
                let remaining_bytes = header.message_length as usize - self.incomplete_chunks.get(&header.basic_header.csid).unwrap().clone().len();
                chunk_size = std::cmp::min(chunk_size, remaining_bytes);
            }

            let mut buf = vec![0; chunk_size];

            // Now we read the rest of the message
            match self.stream.read_exact(&mut buf) {
                Ok(_) => {
                    // RTMP will sometimes send parts of data in different chunks
                    // We need to make sure we read all of the data and parse it all at once.
                    // Essentially, we need to read until we get a message with a message length
                    // We can do this by keeping vecs of a particular chunk stream id in a hashmap and
                    // continuously reading from the socket and adding to our storec vec until we get
                    // a vec with a message length matching what we expect
                    // We can then parse the message and continue reading from the socket

                    // If the size of this message would exceed the max chunk size, we need to do special handling
                    if header.message_length > self.max_chunk_size as u32 {
                        // Retrieve the incomplete chunk vector for the chunk stream ID
                        let chunk_vec = self.incomplete_chunks.entry(header.basic_header.csid).or_insert_with(Vec::new);

                        // Append the received data to the incomplete chunk vector
                        chunk_vec.extend_from_slice(&buf);

                        // Check if we have a complete message
                        if chunk_vec.len() == header.message_length as usize {
                            // If we do, we parse it and continue
                            buf = chunk_vec.to_vec();
                            self.incomplete_chunks.remove(&header.basic_header.csid);
                        } else {
                            // Otherwise, we continue reading from the socket
                            continue;
                        }
                    }

                    if header.message_type_id == 8 {
                        println!("{:?}", buf)
                    }

                    if header.message_type_id == 20 {
                        self.handle_command_message(Cursor::new(&buf));
                        continue;
                    }
                    // If this message is targeting the control stream, we need to parse it properly
                    if header.message_stream_id == 0 {
                        self.handle_control_stream_msg(header, &buf);
                        continue;
                    }
                }
                Err(err) => {
                    eprintln!("Error reading message: {}", err);
                    return;
                }
            }
        }
    }
}