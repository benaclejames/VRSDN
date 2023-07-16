use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use crate::chunk::chunk_container::ChunkContainer;

pub struct ChunkRouter {
    pub recievers: HashMap<String, Receiver<ChunkContainer>>,
}

impl ChunkRouter {
    pub fn new() -> ChunkRouter {
        ChunkRouter {
            recievers: HashMap::new(),
        }
    }
}