use std::sync::Arc;

use tokio::join;
use tokio::net::TcpListener;

use crate::config::Config;
use crate::ingress::socks5::handle_socks5_connection;
use crate::ingress::http_connect::handle_http_connection;

pub async fn run(config: Config) {
    let config = Arc::new(config);
    join!(accept_http_connect(Arc::clone(&config)), accept_socks5(Arc::clone(&config)));
}

async fn accept_http_connect(config: Arc<Config>) {
    let listener = TcpListener::bind(config.http_connect_addr()).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    match handle_http_connection(stream, config.connect_timeout, config.idle_timeout).await {
                        Ok(()) => {
                            
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

async fn accept_socks5(config: Arc<Config>) {
    let listener = TcpListener::bind(config.socks5_addr()).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    match handle_socks5_connection(stream, config.connect_timeout, config.idle_timeout).await {
                        Ok(()) => {
                           
                        }, 
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
