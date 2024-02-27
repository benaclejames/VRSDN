use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

pub struct ChunkRouter {
    pub recievers: HashMap<String, Receiver<Vec<u8>>>,
}

impl ChunkRouter {
    pub fn new() -> ChunkRouter {
        ChunkRouter {
            recievers: HashMap::new(),
        }
    }
}