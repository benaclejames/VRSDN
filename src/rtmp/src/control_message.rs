use crate::Serializable;
use std::io::Read;

pub struct SetChunkSize {
    // 32 bit integer
    pub chunk_size: u32,
}

impl Serializable for SetChunkSize {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.chunk_size.to_be_bytes());
        buf[0] &= 0b01111111;
        Ok(buf)
    }

    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R: Read, Self: Sized
    {
        let mut chunk_size_bytes = [0u8; 4];
        match reader.read_exact(&mut chunk_size_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading chunk_headers size")?,
        };

        Ok(SetChunkSize {
            chunk_size: u32::from_be_bytes(chunk_size_bytes),
        })
    }
}

#[derive(Debug)]
pub struct WindowAcknowledgementSize {
    // 32 bit integer
    pub window_acknowledgement_size: u32,
}

impl Serializable for WindowAcknowledgementSize {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.window_acknowledgement_size.to_be_bytes());
        Ok(buf)
    }

    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R: Read, Self: Sized
    {
        let mut window_acknowledgement_size_bytes = [0u8; 4];
        match reader.read_exact(&mut window_acknowledgement_size_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading window acknowledgement size")?,
        }

        Ok(WindowAcknowledgementSize {
            window_acknowledgement_size: u32::from_be_bytes(window_acknowledgement_size_bytes),
        })
    }
}

#[derive(Debug)]
pub struct SetPeerBandwidth {
    pub window_acknowledgement_size: u32,
    pub limit_type: u8,
}

impl Serializable for SetPeerBandwidth {
    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.window_acknowledgement_size.to_be_bytes());
        buf.push(self.limit_type);
        Ok(buf)
    }

    fn deserialize<R>(reader: &mut R) -> Result<Self, &'static str> where R: Read, Self: Sized
    {
        let mut window_acknowledgement_size_bytes = [0u8; 4];
        match reader.read_exact(&mut window_acknowledgement_size_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading window acknowledgement size")?,
        }

        let mut limit_type_bytes = [0u8; 1];
        match reader.read_exact(&mut limit_type_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading limit type")?,
        }

        Ok(SetPeerBandwidth {
            window_acknowledgement_size: u32::from_be_bytes(window_acknowledgement_size_bytes),
            limit_type: limit_type_bytes[0],
        })
    }
}