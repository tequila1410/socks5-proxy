use tokio::net::TcpStream;

pub async fn handle_connection(stream: TcpStream) {
    println!("New connection: {:?}", stream);
}