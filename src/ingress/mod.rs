use std::io::{Error, ErrorKind};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::tunnel;

pub struct Target {
    pub host: String,
    pub port: u16,
}

pub async fn handle_connection(mut client_stream: TcpStream, connect_timeout: Duration, idle_timeout: Duration) -> Result<(), Error> {
    socks5_negotiate_no_auth(&mut client_stream).await?;
    let target = socks5_read_request(&mut client_stream).await?;

    let mut target_stream = match tokio::time::timeout(connect_timeout, TcpStream::connect((target.host.as_str(), target.port))).await {
        Ok(stream_result) => {
            match stream_result {
                Ok(stream) => stream,
                Err(e) => {
                    socks5_reply(&mut client_stream, socks_rep_for_dial_error(&e)).await?;
                    return Err(e);
                }
            }
        },
        Err(_) => {
            let err = Error::new(ErrorKind::TimedOut, "Timeout");
            socks5_reply(&mut client_stream, socks_rep_for_dial_error(&err)).await?;
            return Err(err);
        }
    };


    socks5_reply(&mut client_stream, 0x00).await?;
    tunnel::copy_bidirectional_tcp(&mut client_stream, &mut target_stream, idle_timeout).await
}

fn socks_rep_for_dial_error(err: &Error) -> u8 {
    match err.kind() {
        ErrorKind::ConnectionRefused => 0x05,
        ErrorKind::TimedOut | ErrorKind::HostUnreachable => 0x04,
        ErrorKind::NetworkUnreachable => 0x03,
        _ => 0x01,
    }
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
