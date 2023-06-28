use crate::Serializable;
use std::io;
use std::io::Read;
use amf::{Version};
use amf::amf0::Value;

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

    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized
    {
        let mut chunk_size_bytes = [0u8; 4];
        match reader.read_exact(&mut chunk_size_bytes) {
            Ok(_) => {}
            Err(_) => Err("Error reading chunk size")?,
        };

        Ok(SetChunkSize {
            chunk_size: u32::from_be_bytes(chunk_size_bytes),
        })
    }
}

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

    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized
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

    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized
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

pub struct AMFMessage {
    pub command_name: String,
    pub transaction_id: f64,
    pub properties: Vec<(String, Value)>,
    pub information: Vec<(String, Value)>,
}

impl Serializable for AMFMessage {
    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: Read, Self: Sized {
        let command_name: String = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf) => amf.try_as_str().unwrap().to_string(),
            _ => Err("Error reading AMF0 Command Name")?,
        };

        let transaction_id: f64 = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Number(x))) => x,
            _ => Err("Error reading AMF0 Transaction ID")?,
        };

        let command_object: Vec<(String, Value)> = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Object {entries, ..})) => entries.into_iter().map(|p| (p.key, p.value)).collect(),
            _ => Err("Error reading AMF0 Command Object")?,
        };

        Ok(AMFMessage {
            command_name,
            transaction_id,
            properties: command_object,
            information: Vec::new(),
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let command_name = Value::from(Value::String(self.command_name.clone()));
        let transaction_number = Value::from(Value::Number(self.transaction_id));
        let mut buf = Vec::new();
        command_name.write_to(&mut buf).unwrap();
        transaction_number.write_to(&mut buf).unwrap();

        Ok(buf)
    }
}