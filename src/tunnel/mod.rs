use std::io::{Error, ErrorKind};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;

pub async fn copy_bidirectional_tcp(
    client: &mut TcpStream,
    target: &mut TcpStream,
    idle: Duration,
) -> Result<(), Error> {
    let mut buf_client_target = [0u8; 1024];
    let mut buf_target_client = [0u8; 1024];
    
    let mut idle_timer = Box::pin(tokio::time::sleep(idle));

    let mut should_read_client = true;
    let mut should_read_target = true;
    loop {
        if !should_read_client && !should_read_target {
            return Ok(());
        }
        tokio::select! {
            result = client.read(&mut buf_client_target), if should_read_client => {
                let bytes_read = match result {
                    Ok(0) => {
                        should_read_client = false;
                        target.shutdown().await?;
                        continue;
                    },
                    Ok(n) => n,
                    Err(e) => {
                        return Err(e);
                    }
                };
                target.write_all(&buf_client_target[..bytes_read]).await?;
                idle_timer.as_mut().reset(Instant::now() + idle);
            }
            result = target.read(&mut buf_target_client), if should_read_target => {
                let bytes_read = match result {
                    Ok(0) => {
                        should_read_target = false;
                        client.shutdown().await?;
                        continue;
                    },
                    Ok(n) => n,
                    Err(e) => {
                        return Err(e);
                    }
                };
                client.write_all(&buf_target_client[..bytes_read]).await?;
                idle_timer.as_mut().reset(Instant::now() + idle);
            }
            _ = &mut idle_timer => {
                return Err(Error::new(ErrorKind::TimedOut, "Idle timeout"));
            }
        }
    }
}
