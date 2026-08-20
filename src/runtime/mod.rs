use std::future::Future;
use std::io::Error;
use std::sync::Arc;
use std::time::Duration;

use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::ingress::http_connect::handle_http_connection;
use crate::ingress::socks5::handle_socks5_connection;

pub async fn run(config: Config) {
    let config = Arc::new(config);
    let semaphore = Arc::new(Semaphore::new(config.connections_limit));
    join!(
        accept_loop(
            config.http_connect_addr(),
            Arc::clone(&config),
            Arc::clone(&semaphore),
            handle_http_connection,
        ),
        accept_loop(
            config.socks5_addr(),
            Arc::clone(&config),
            semaphore,
            handle_socks5_connection,
        ),
    );
}

async fn accept_loop<F, Fut>(
    addr: String,
    config: Arc<Config>,
    semaphore: Arc<Semaphore>,
    handler: F,
) where
    F: Fn(TcpStream, Duration, Duration) -> Fut + Copy + Send + 'static,
    Fut: Future<Output = Result<(), Error>> + Send,
{
    let listener = TcpListener::bind(&addr).await.unwrap();
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
                    if let Err(e) = handler(stream, config.connect_timeout, config.idle_timeout).await
                    {
                        println!("Error: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
