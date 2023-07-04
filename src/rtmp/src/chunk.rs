use std::io;
use crate::Serializable;

pub struct ChunkBasicHeader {
    pub fmt: u8,
    pub csid: u8,
}

pub struct ChunkHeader {
    pub basic_header: ChunkBasicHeader,

    pub timestamp: u32,
    pub message_length: u32,
    pub message_type_id: u8,
    pub message_stream_id: u32,
}

impl Serializable for ChunkBasicHeader {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        return Ok(vec![self.fmt << 6 | self.csid]);
    }

    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized {
        let mut buf = [0; 1];
        reader.read_exact(&mut buf).unwrap();
        let fmt = buf[0] >> 6;  // Fmt is the first 2 bits
        let mut csid = buf[0] & 0b00111111;

        if csid == 0 {
            match reader.read_exact(&mut buf) {
                Ok(_) => csid = buf[0] + 64,
                _ => Err("Error reading basic header csid 1")?,
            }
        }

        if csid == 1 {
            let mut buf = [0; 2];
            match reader.read_exact(&mut buf) {
                Ok(_) => csid = u16::from_be_bytes(buf) as u8 + 64,
                _ => Err("Error reading basic header csid 2")?,
            }
        }

        println!("fmt: {}, csid: {}", fmt, csid);

        Ok(ChunkBasicHeader {
            fmt,
            csid,
        })
    }
}

impl Serializable for ChunkHeader {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.basic_header.serialize().unwrap());

        // 3 byte timestamp
        buf.extend_from_slice(&self.timestamp.to_be_bytes()[1..4]);
        // 3 byte message length
        buf.extend_from_slice(&self.message_length.to_be_bytes()[1..4]);
        // 1 byte message type id
        buf.push(self.message_type_id);
        // 4 byte message stream id
        buf.extend_from_slice(&self.message_stream_id.to_be_bytes());

        Ok(buf)
    }

    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized {
        // First we read the basic header to determine the fmt to attempt to read
        let basic_header = match ChunkBasicHeader::deserialize(&mut reader) {
            Ok(bh) => bh,
            _ => Err("Error reading basic header")?,
        };

        // Now depending on the fmt, we read the rest of the header. If the fmt is 0, we read the
        // timestamp, message length and message type id.
        match basic_header.fmt {
            0 => {
                let mut buf = [0; 11];
                match reader.read_exact(&mut buf) {
                    Ok(_) =>  {
                        // 3 byte timestamp (we need to pad this to 4 bytes)
                        let timestamp = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);
                        // 3 byte message length
                        let message_length = u32::from_be_bytes([0, buf[3], buf[4], buf[5]]);
                        // 1 byte message type id
                        let message_type_id = buf[6];
                        // 4 byte message stream id
                        let message_stream_id = u32::from_be_bytes(buf[7..11].try_into().unwrap());

                        Ok(ChunkHeader {
                            basic_header,
                            timestamp,
                            message_length,
                            message_type_id,
                            message_stream_id,
                        })
                    }
                    _ => Err("Error reading full header")?,
                }
            }
            _ => Err("Unsupported fmt")?,
        }
    }
}