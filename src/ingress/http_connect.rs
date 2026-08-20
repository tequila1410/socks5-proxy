use std::io::{Error, ErrorKind};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::ingress::{connect_with_timeout, Target};
use crate::tunnel;

const MAX_HEADERS_BYTES: usize = 8 * 1024;

enum ReadHeadersError {
    Incomplete,
    TooLong,
    Io(Error),
}

#[derive(Debug, PartialEq, Eq)]
enum ParseError {
    MethodNotAllowed,
    BadRequest,
}

impl ParseError {
    fn status(self) -> u16 {
        match self {
            ParseError::MethodNotAllowed => 405,
            ParseError::BadRequest => 400,
        }
    }
}

pub async fn handle_http_connection(
    mut client_stream: TcpStream,
    connect_timeout: Duration,
    idle_timeout: Duration,
) -> Result<(), Error> {
    let headers = match read_headers(&mut client_stream).await {
        Ok(headers) => headers,
        Err(ReadHeadersError::TooLong) => {
            http_reply(&mut client_stream, 431).await?;
            return Err(Error::new(ErrorKind::InvalidData, "request too long"));
        }
        Err(ReadHeadersError::Incomplete) => {
            return Err(Error::new(ErrorKind::UnexpectedEof, "incomplete CONNECT request"));
        }
        Err(ReadHeadersError::Io(e)) => return Err(e),
    };

    let target = match parse_connect(&headers) {
        Ok(target) => target,
        Err(e) => {
            let status = e.status();
            http_reply(&mut client_stream, status).await?;
            return Err(Error::new(ErrorKind::InvalidData, "invalid CONNECT request"));
        }
    };

    let mut target_stream = match connect_with_timeout(&target, connect_timeout).await {
        Ok(stream) => {
            http_reply(&mut client_stream, 200).await?;
            stream
        }
        Err(e) => {
            http_reply(&mut client_stream, http_status_for_dial_error(&e)).await?;
            return Err(e);
        }
    };
    tunnel::copy_bidirectional_tcp(&mut client_stream, &mut target_stream, idle_timeout).await
}

fn http_status_for_dial_error(err: &Error) -> u16 {
    match err.kind() {
        ErrorKind::ConnectionRefused | ErrorKind::HostUnreachable | ErrorKind::NetworkUnreachable => 502,
        ErrorKind::TimedOut => 504,
        _ => 500,
    }
}

async fn read_headers(stream: &mut TcpStream) -> Result<Vec<u8>, ReadHeadersError> {
    let mut buf = [0u8; 1024];
    let mut pending = Vec::new();

    loop {
        let bytes_read = match stream.read(&mut buf).await {
            Ok(0) => return Err(ReadHeadersError::Incomplete),
            Ok(n) => n,
            Err(e) => return Err(ReadHeadersError::Io(e)),
        };

        pending.extend_from_slice(&buf[..bytes_read]);

        if pending.len() > MAX_HEADERS_BYTES {
            return Err(ReadHeadersError::TooLong);
        }

        let Some(header_end) = pending.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };

        pending.truncate(header_end);
        return Ok(pending);
    }
}

fn parse_connect(headers: &[u8]) -> Result<Target, ParseError> {
    let headers = std::str::from_utf8(headers).map_err(|_| ParseError::BadRequest)?;
    let request_line = match headers.split_once("\r\n") {
        Some((line, _)) => line,
        None => headers,
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(ParseError::BadRequest)?;
    let authority = parts.next().ok_or(ParseError::BadRequest)?;
    let version = parts.next().ok_or(ParseError::BadRequest)?;
    if parts.next().is_some() {
        return Err(ParseError::BadRequest);
    }

    if method != "CONNECT" {
        return Err(ParseError::MethodNotAllowed);
    }
    if !version.starts_with("HTTP/") {
        return Err(ParseError::BadRequest);
    }

    let (host, port) = authority.split_once(':').ok_or(ParseError::BadRequest)?;
    if host.is_empty() {
        return Err(ParseError::BadRequest);
    }
    let port = port.parse::<u16>().map_err(|_| ParseError::BadRequest)?;

    Ok(Target {
        host: host.to_string(),
        port,
    })
}

async fn http_reply(stream: &mut TcpStream, status: u16) -> Result<(), Error> {
    if status == 200 {
        let response = format!("HTTP/1.1 {status} \r\n\r\n");
        return stream.write_all(response.as_bytes()).await;
    }

    let reason = match status {
        400 => "Bad Request",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        504 => "Gateway Timeout",
        _ => "Error",
    };
    let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(response.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connect_with_extra_headers() {
        let raw = b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443";
        let target = parse_connect(raw).unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.port, 443);
    }

    #[test]
    fn parses_connect_request_line_only() {
        let raw = b"CONNECT example.com:443 HTTP/1.1";
        let target = parse_connect(raw).unwrap();
        assert_eq!(target.host, "example.com");
        assert_eq!(target.port, 443);
    }

    #[test]
    fn rejects_get() {
        let raw = b"GET / HTTP/1.1";
        assert!(matches!(parse_connect(raw), Err(ParseError::MethodNotAllowed)));
    }

    #[test]
    fn rejects_bad_port() {
        let raw = b"CONNECT example.com:99999 HTTP/1.1";
        assert!(matches!(parse_connect(raw), Err(ParseError::BadRequest)));
    }

    #[test]
    fn rejects_missing_port() {
        let raw = b"CONNECT example.com HTTP/1.1";
        assert!(matches!(parse_connect(raw), Err(ParseError::BadRequest)));
    }

    #[test]
    fn accepts_http_1_0() {
        let raw = b"CONNECT example.com:80 HTTP/1.0";
        let target = parse_connect(raw).unwrap();
        assert_eq!(target.port, 80);
    }

    #[test]
    fn maps_dial_errors_to_connect_status() {
        let cases = [
            (ErrorKind::ConnectionRefused, 502),
            (ErrorKind::HostUnreachable, 502),
            (ErrorKind::NetworkUnreachable, 502),
            (ErrorKind::TimedOut, 504),
            (ErrorKind::Other, 500),
        ];
        for (kind, status) in cases {
            let err = Error::new(kind, "test");
            assert_eq!(http_status_for_dial_error(&err), status, "{kind:?}");
        }
    }
}
