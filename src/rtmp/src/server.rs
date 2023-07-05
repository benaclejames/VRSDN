use std::collections::HashMap;
pub use crate::handshake::{CS0, CS1};
use std::net::TcpStream;
use std::io::{Read, ErrorKind, Write, Cursor};
use crate::chunk::{ChunkBasicHeader, ChunkHeader};
use crate::Serializable;
use crate::protocontrol::{AMFMessage, SetChunkSize, SetPeerBandwidth, WindowAcknowledgementSize};
use amf::amf0::Value::{String, Number};

pub struct RtmpConnection {
    stream: TcpStream,
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

    fn handshake(&mut self) -> Result<u8, &'static str> {
        // Read 1 byte for the CS0 version
        let mut buf = [0; 1];
        let c0 = match self.stream.read_exact(&mut buf) {
            Ok(_) => {
                let c0 = CS0::deserialize(&buf[..]);

                c0
            }
            Err(err) => {
                eprintln!("Error reading CS0 version: {}", err);
                return Err("Error reading CS0 version");
            }
        };

        // Now read 1536 bytes for the CS1 chunk
        let mut buf2 = vec![0; 1536];
        let c1: CS1 = match self.stream.read_exact(&mut buf2) {
            Ok(_) =>
                match CS1::deserialize(&buf2[..]) {
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
        // then our own cs1 chunk. We need to use the same timestamp as the client but random bytes
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
        self.stream.write_all(&s0.unwrap().serialize().unwrap()).unwrap();
        self.stream.write_all(&s1.serialize().unwrap()).unwrap();
        self.stream.write_all(&s2.serialize().unwrap()).unwrap();

        // Now we wait for the client to send their CS2
        let mut buf3 = vec![0; 1536];
        match self.stream.read_exact(&mut buf3) {
            Ok(_) =>
                match CS1::deserialize(&buf3[..]) {
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
                self.send_message(WindowAcknowledgementSize { window_acknowledgement_size: 5000000 }, 2, 5, 0);
                self.send_message(SetPeerBandwidth { window_acknowledgement_size: 5000000, limit_type: 1 }, 2, 6, 0);
                self.send_message(SetChunkSize { chunk_size: 5000 }, 2, 1, 0);

                self.send_message(AMFMessage {
                    transaction_id: 1.0,
                    command_name: "_result".to_string(),
                    properties: vec![
                        ("fmsVer".to_string(), String("FMS/3,0,1,123".to_string())),
                        ("capabilities".to_string(), Number(31.0)),
                        ("mode".to_string(), String("live".to_string())),
                        ("objectEncoding".to_string(), Number(0.0)),
                    ],
                    information: vec![
                        ("level".to_string(), String("status".to_string())),
                        ("code".to_string(), String("NetConnection.Connect.Success".to_string())),
                        ("description".to_string(), String("Connection succeeded.".to_string())),
                        ("objectEncoding".to_string(), Number(0.0)),
                    ],
                }, 3, 20, 0);

                println!("Successfully responded to connect request")
            }
            "createStream" => {
                self.send_message(AMFMessage {
                    transaction_id: message.transaction_id,
                    command_name: "_result".to_string(),
                    properties: Vec::new(),
                    information: Vec::new(),
                }, 3, 20, 0)
            }
            "publish" => {
                self.send_message(AMFMessage {
                    transaction_id: 0.0,
                    command_name: "onStatus".to_string(),
                    properties: Vec::new(),
                    information: vec![
                        ("code".to_string(), (String("NetStream.Publish.Start".to_string()))),
                        ("level".to_string(), (String("status".to_string()))),
                        ("description".to_string(), (String("Started publishing stream.".to_string()))),
                    ]
                }, 3, 20, 0)
            }
            _ => {
                println!("Unsupported command: {} with data: {:?}", message.command_name, message.information);
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
            _ => {
                println!("Unsupported control stream message type: {}", header.message_type_id);
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

        let header = ChunkHeader {
            basic_header: ChunkBasicHeader {
                fmt: 0,
                csid: chunk_stream_id,
            },
            timestamp: 0,
            message_length: data.len() as u32,
            message_type_id: type_id,
            message_stream_id,
        };

        match header.serialize() {
            Ok(buf) => {
                self.stream.write_all(&buf).unwrap();
                self.stream.write_all(&data).unwrap();
            }
            Err(err) => {
                eprintln!("Error serializing message: {}", err);
            }
        }
    }

    pub fn handle_connection(&mut self) {
        println!("Handling connection from {}", self.stream.peer_addr().unwrap());

        match self.handshake() {
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
                        println!("CSID {} has message length {} which exceeds max chunk size {}", header.basic_header.csid, header.message_length, self.max_chunk_size);

                        // Retrieve the incomplete chunk vector for the chunk stream ID
                        let chunk_vec = self.incomplete_chunks.entry(header.basic_header.csid).or_insert_with(Vec::new);

                        // Append the received data to the incomplete chunk vector
                        chunk_vec.extend_from_slice(&buf);

                        // Check if we have a complete message
                        if chunk_vec.len() == header.message_length as usize {
                            // If we do, we parse it and continue
                            println!("Found complete message of length {} and target length {}", chunk_vec.len(), header.message_length);
                            buf = chunk_vec.to_vec();
                            self.incomplete_chunks.remove(&header.basic_header.csid);
                        } else {
                            // Otherwise, we continue reading from the socket
                            println!("Incomplete message of length {} and target length {}", chunk_vec.len(), header.message_length);
                            continue;
                        }
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
                    println!("Received message: {:?}", buf);
                }
                Err(err) => {
                    eprintln!("Error reading message: {}", err);
                    return;
                }
            }
        }
    }
}