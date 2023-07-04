use crate::Serializable;
use std::io;
use std::io::Read;
use amf::{Pair, Version};
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

fn vec_pair_to_tuple(source: Vec<Pair<String, Value>>) -> Vec<(String, Value)> {
    source.into_iter().map(|p| (p.key, p.value)).collect()
}

fn tuple_to_vec_pair(source: Vec<(String, Value)>) -> Vec<Pair<String, Value>> {
    source.iter().map(|(k, v)| Pair { key: k.clone(), value: v.clone() }).collect()
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
            Ok(amf::Value::Amf0(Value::Object {entries, ..})) => vec_pair_to_tuple(entries),
            _ => Err("Error reading AMF0 Command Object")?,
        };

        let optional_info: Vec<(String, Value)> = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Object {entries, ..})) => vec_pair_to_tuple(entries),
            _ => Vec::new()
        };

        Ok(AMFMessage {
            command_name,
            transaction_id,
            properties: command_object,
            information: optional_info
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        Value::from(Value::String(self.command_name.clone())).write_to(&mut buf).expect("Failed to serialize response name string");
        Value::from(Value::Number(self.transaction_id)).write_to(&mut buf).expect("Failed to serialize response transaction id");
        Value::from(Value::Object { class_name: None, entries: tuple_to_vec_pair(self.properties.clone())}).write_to(&mut buf).expect("Failed to serialize response props");
        Value::from(Value::Object { class_name: None, entries: tuple_to_vec_pair(self.information.clone())}).write_to(&mut buf).expect("Failed to serialize response info");
        Ok(buf)
    }
}