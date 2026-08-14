use std::io::{Error, ErrorKind};

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Target {
    pub host: String,
    pub port: u16,
}

pub async fn handle_connection(stream: &mut TcpStream) -> Result<(), Error> {
    socks5_negotiate_no_auth(stream).await?;
    let target = socks5_read_request(stream).await?;
    // socks5_reply(stream, 0x00).await?;
    println!("Target host: {}, port: {}", target.host, target.port);
    Ok(())
}

async fn socks5_negotiate_no_auth(stream: &mut TcpStream) -> Result<(), Error> {
    let mut header = [0u8; 2];

    stream.read_exact(&mut header).await?;

    let version = header[0];
    let nmethods = header[1];

    if version != 5 {
        stream.write_all(&[5, 0xff]).await?;
        return Err(Error::new(ErrorKind::Other, "Invalid version"));
    }

    let mut methods = vec![0u8; nmethods as usize];

    stream.read_exact(&mut methods).await?;

    if methods.contains(&0) {
        stream.write_all(&[5, 0]).await?;
        return Ok(());
    } else {
        stream.write_all(&[5, 0xff]).await?;
        return Err(Error::new(ErrorKind::Other, "No supported authentication methods"));
    }
}

async fn socks5_read_request(stream: &mut TcpStream) -> Result<Target, Error> {
    let mut header = [0u8; 4];

    stream.read_exact(&mut header).await?;

    let version = header[0];
    let cmd = header[1];
    let addr_type = header[3];

    if version != 5 {
        socks5_reply(stream, 0x01).await?;
        return Err(Error::new(ErrorKind::Other, "Invalid version"));
    }

    if cmd != 1 {
        socks5_reply(stream, 0x07).await?;
        return Err(Error::new(ErrorKind::Other, "Invalid command"));
    }

    match addr_type {
        1 => {
            let mut host = [0u8; 4];
            stream.read_exact(&mut host).await?;
            let host = std::net::Ipv4Addr::from(host).to_string();

            let mut port = [0u8; 2];
            stream.read_exact(&mut port).await?;
            let port = u16::from_be_bytes(port);

            return Ok(Target { host, port });
        },
        3 => {
            let mut host_len = [0u8; 1];
            stream.read_exact(&mut host_len).await?;
            let host_len = host_len[0] as usize;
            if host_len == 0 {
                socks5_reply(stream, 0x01).await?;
                return Err(Error::new(ErrorKind::Other, "Invalid host"));
            }
            let mut host = vec![0u8; host_len];
            stream.read_exact(&mut host).await?;
            let host = match String::from_utf8(host) {
                Ok(host) => host,
                Err(_) => {
                    socks5_reply(stream, 0x01).await?;
                    return Err(Error::new(ErrorKind::Other, "Invalid host"));
                }
            };

            let mut port = [0u8; 2];
            stream.read_exact(&mut port).await?;
            let port = u16::from_be_bytes(port);

            return Ok(Target { host, port });
        },
        _ => {
            socks5_reply(stream, 0x08).await?;
            return Err(Error::new(ErrorKind::Other, "Invalid address type"));
        }
    }
}

async fn socks5_reply(stream: &mut TcpStream, rep: u8) -> Result<(), Error> {
    stream.write_all(&[0x05, rep, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).await
}
