use tokio::net::TcpListener;

use crate::ingress::handle_connection;

pub async fn run(socks5_addr: String) {
    let listener = TcpListener::bind(socks5_addr).await.unwrap();
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut stream = stream;
            match handle_connection(&mut stream).await {
                Ok(()) => {

                },
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        });
    }
}