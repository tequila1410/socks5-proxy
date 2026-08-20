use std::sync::Arc;

use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::ingress::socks5::handle_socks5_connection;
use crate::ingress::http_connect::handle_http_connection;

pub async fn run(config: Config) {
    let config = Arc::new(config);
    let semaphore = Arc::new(Semaphore::new(config.connections_limit));
    join!(accept_http_connect(Arc::clone(&config), Arc::clone(&semaphore)), accept_socks5(Arc::clone(&config), semaphore.clone()));
}

async fn accept_http_connect(config: Arc<Config>, semaphore: Arc<Semaphore>) {
    let listener = TcpListener::bind(config.http_connect_addr()).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("Accepted HTTP connection");
                let config = Arc::clone(&config);
                let semaphore = Arc::clone(&semaphore);
                let permit = match semaphore.try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(e) => {
                        println!("Error acquiring permit: {}", e);
                        continue;
                    }
                };
                tokio::spawn(async move {
                    let _permit = permit;
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

async fn accept_socks5(config: Arc<Config>, semaphore: Arc<Semaphore>) {
    let listener = TcpListener::bind(config.socks5_addr()).await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let config = Arc::clone(&config);
                let semaphore = Arc::clone(&semaphore);
                let permit = match semaphore.try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(e) => {
                        println!("Error acquiring permit: {}", e);
                        continue;
                    }
                };
                tokio::spawn(async move {
                    let _permit = permit;
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
