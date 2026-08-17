use tokio::net::TcpListener;

use crate::{config::Config, ingress::handle_connection};

pub async fn run(config: Config) {
    let listener = TcpListener::bind(config.socks5_addr()).await.unwrap();
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            match handle_connection(stream, config.connect_timeout, config.idle_timeout).await {
                Ok(()) => {

                },
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        });
    }
}