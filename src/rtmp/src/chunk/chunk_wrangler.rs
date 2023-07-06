use std::collections::HashMap;
use std::io::Read;
use crate::chunk::chunk_headers::ChunkHeader;
use crate::Serializable;
use crate::socket::RtmpSocket;

// Chunk wrangler's job is to collect partial chunks and format them into full chunks

pub struct ChunkWrangler {
    pub max_chunk_size: usize,
    incomplete_chunks: HashMap<u8, Vec<u8>>,
}

impl ChunkWrangler {
    pub fn new() -> Self {
        Self {
            max_chunk_size: 128,
            incomplete_chunks: HashMap::new(),
        }
    }

    pub fn read_chunk(&mut self, socket: &mut RtmpSocket) -> Result<(ChunkHeader, Vec<u8>), &'static str> {
        let header = match ChunkHeader::deserialize(socket) {
            Ok(header) => header,
            Err(err) => return Err(err),
        };

        // If this csid exists in the incomplete messages hashmap, we need to get the remaining bytes to read to complete the msg and pass it into the min
        let mut chunk_size = std::cmp::min(self.max_chunk_size, header.message_length as usize);
        if self.incomplete_chunks.contains_key(&header.basic_header.csid) {
            let remaining_bytes = header.message_length as usize - self.incomplete_chunks.get(&header.basic_header.csid).unwrap().clone().len();
            chunk_size = std::cmp::min(chunk_size, remaining_bytes);
        }

        let mut buf = vec![0; chunk_size];

        // Now we read the rest of the message
        match socket.read_exact(&mut buf) {
            Ok(_) => {
                // RTMP will sometimes send parts of data in different chunks
                // We need to make sure we read all of the data and parse it all at once.
                // Essentially, we need to read until we get a message with a message length
                // We can do this by keeping vecs of a particular chunk_headers stream id in a hashmap and
                // continuously reading from the socket and adding to our storec vec until we get
                // a vec with a message length matching what we expect
                // We can then parse the message and continue reading from the socket

                // If the size of this message would exceed the max chunk_headers size, we need to do special handling
                if header.message_length > self.max_chunk_size as u32 {
                    // Retrieve the incomplete chunk_headers vector for the chunk_headers stream ID
                    let chunk_vec = self.incomplete_chunks.entry(header.basic_header.csid).or_insert_with(Vec::new);

                    // Append the received data to the incomplete chunk_headers vector
                    chunk_vec.extend_from_slice(&mut buf);

                    // Check if we have a complete message
                    if chunk_vec.len() == header.message_length as usize {
                        // If we do, we parse it and continue
                        buf = chunk_vec.to_vec();
                        self.incomplete_chunks.remove(&header.basic_header.csid);
                    } else {
                        // Otherwise, we continue reading from the socket
                        return Err("Incomplete chunk_headers")
                    }
                }

                Ok((header, buf))
            }
            Err(_) => Err("Error reading chunk_headers"),
        }
    }
}