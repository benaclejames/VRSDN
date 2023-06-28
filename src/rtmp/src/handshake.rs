use crate::Serializable;
use std::io;

pub struct CS0 {
        pub version: u8,
    }

    impl Serializable for CS0 {
        fn serialize(&self) -> Result<Vec<u8>, &'static str> {
            let mut buf = Vec::new();
            buf.push(self.version);
            Ok(buf)
        }

        fn deserialize<R>(reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized {
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

        fn deserialize<R>(mut reader: R) -> Result<Self, &'static str> where R: io::Read, Self: Sized
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
