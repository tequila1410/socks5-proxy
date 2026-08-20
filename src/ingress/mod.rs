pub mod http_connect;
pub mod socks5;

use std::io::{Error, ErrorKind};
use std::time::Duration;

use tokio::net::TcpStream;

pub struct Target {
    pub host: String,
    pub port: u16,
}

pub(crate) async fn connect_with_timeout(
    target: &Target,
    connect_timeout: Duration,
) -> Result<TcpStream, Error> {
    match tokio::time::timeout(
        connect_timeout,
        TcpStream::connect((target.host.as_str(), target.port)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(Error::new(ErrorKind::TimedOut, "timeout")),
    }
}
