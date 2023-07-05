use crate::Serializable;
use std::io::Read;
use amf::{Pair, Version};
use amf::amf0::Value;

pub struct AMFMessage {
    pub command_name: String,
    pub transaction_id: f64,
}

// Similar to AMFMessage, but contains command object and additional args
pub struct AMFCall {
    pub command_object: Vec<(String, Value)>,
    pub additional_args: Vec<(String, Value)>,
}

pub struct PlayMessage {
    pub stream_name: String,
    pub start: f64,
    pub duration: f64,
    pub reset: bool,
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

        Ok(AMFMessage {
            command_name,
            transaction_id,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        Value::from(Value::String(self.command_name.clone())).write_to(&mut buf).expect("Failed to serialize response name string");
        Value::from(Value::Number(self.transaction_id)).write_to(&mut buf).expect("Failed to serialize response transaction id");
        Ok(buf)
    }
}

impl Serializable for AMFCall {
    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: Read, Self: Sized {
        let command_object: Vec<(String, Value)> = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Object {entries, ..})) => vec_pair_to_tuple(entries),
            _ => Vec::new()
        };

        let additional_args: Vec<(String, Value)> = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Object {entries, ..})) => vec_pair_to_tuple(entries),
            _ => Vec::new()
        };

        Ok(AMFCall {
            command_object,
            additional_args,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::new();
        Value::from(Value::Object { class_name: None, entries: tuple_to_vec_pair(self.command_object.clone())}).write_to(&mut buf).expect("Failed to serialize response props");
        Value::from(Value::Object { class_name: None, entries: tuple_to_vec_pair(self.additional_args.clone())}).write_to(&mut buf).expect("Failed to serialize response info");
        Ok(buf)
    }
}

impl Serializable for PlayMessage {
    fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: Read, Self: Sized {
        // Now we expect a null for the command object
        match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Null)) => {},
            _ => Err("Error reading AMF0 Null")?,
        }

        let stream_name: String = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf) => amf.try_as_str().unwrap().to_string(),
            _ => Err("Error reading AMF0 Stream Name")?,
        };
        println!("Stream name: {}", stream_name);

        let start: f64 = match amf::Value::read_from(&mut reader, Version::Amf0) {
            Ok(amf::Value::Amf0(Value::Number(x))) => x,
            _ => Err("Error reading AMF0 Start")?,
        };
        println!("Start: {}", start);


        Ok(PlayMessage {
            stream_name,
            start,
            duration: 0.0,
            reset: false,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        Ok(Vec::new())
    }
}