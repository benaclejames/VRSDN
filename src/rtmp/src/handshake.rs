use crate::Serializable;
use std::io;
use std::io::{Read, Write, ErrorKind};
use crate::server::RtmpConnection;

pub struct CS0 {
    pub version: u8,
}

impl Serializable for CS0 {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.push(self.version);
        Ok(buf)
    }

    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R: io::Read, Self: Sized {
        let version = match reader.bytes().next() {
            Some(Ok(v)) => Some(v),
            _ => Err("Error reading version")?,
        };

        Ok(CS0 {
            version: version.unwrap(),
        })
    }
}

pub struct CS1 {
    pub timestamp: u32,
    pub zero: u32,
    pub random_bytes: Vec<u8>
}

impl Serializable for CS1 {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&self.zero.to_be_bytes());
        buf.extend_from_slice(&self.random_bytes);
        Ok(buf)
    }

    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R: io::Read, Self: Sized
    {
        let mut timestamp_bytes = [0u8; 4];
        match reader.read_exact(&mut timestamp_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading timestamp")?,
        };

        let mut zero_bytes = [0u8; 4];
        match reader.read_exact(&mut zero_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading zero")?,
        };

        let mut random_bytes = Vec::new();
        match reader.read_to_end(&mut random_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading random bytes")?,
        };

        Ok(CS1 {
            timestamp: u32::from_be_bytes(timestamp_bytes),
            zero: u32::from_be_bytes(zero_bytes),
            random_bytes,
        })
    }
}

pub fn handshake(connection: &mut RtmpConnection) -> Result<u8, &'static str> {
    // Read 1 byte for the CS0 version
    let mut buf = [0; 1];
    let c0 = match connection.socket.read_exact(&mut buf) {
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
    let c1: CS1 = match connection.socket.read_exact(&mut buf2) {
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
    connection.socket.write_all(&s0.serialize().unwrap()).unwrap();
    connection.socket.write_all(&s1.serialize().unwrap()).unwrap();
    connection.socket.write_all(&s2.serialize().unwrap()).unwrap();

    // Now we wait for the client to send their CS2
    let mut buf3 = vec![0; 1536];
    match connection.socket.read_exact(&mut buf3) {
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
