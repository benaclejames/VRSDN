use std::io;
use rtmp;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Awaiting connection!");

    let server = rtmp::RtmpServer::new();
    server.start().await
}
