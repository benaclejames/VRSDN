use rtmp;

fn main() {
    println!("Awaiting connection!");

    let mut server = rtmp::RtmpServer::new();
    server.start();
}
