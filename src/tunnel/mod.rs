use std::io::Error;

use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

pub async fn copy_bidirectional_tcp(
    client: &mut TcpStream,
    target: &mut TcpStream,
) -> Result<(), Error> {
    copy_bidirectional(client, target).await?;
    Ok(())
}
