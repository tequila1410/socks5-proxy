# proxy server

A small local forward proxy: **SOCKS5** (no authentication) and **HTTP CONNECT**. After the handshake it opens a TCP connection to the target and copies bytes both ways.

HTTP `GET`/`POST` through the proxy is not supported — only `CONNECT` (typical for HTTPS).

## Setup

Rust (stable) and Cargo.

```bash
cp .env.example .env
cargo run
```

`.env` (defaults from `.env.example`):

| Variable | Meaning |
|----------|---------|
| `PROXY_HOST` | Bind address (`127.0.0.1`) |
| `SOCKS5_PORT` | SOCKS5 listen port (`1080`) |
| `HTTP_CONNECT_PORT` | HTTP CONNECT listen port (`8080`) |
| `CONNECTIONS_LIMIT` | Max concurrent tunnels |

Optional: `RUST_LOG=debug` (default is `info` if unset).

Stop with Ctrl-C; a second Ctrl-C (or a short wait) aborts leftover connections.

## Use

**SOCKS5** (let the proxy resolve the hostname):

```bash
curl --socks5-hostname 127.0.0.1:1080 https://example.com/
```

**HTTP CONNECT** (HTTPS so curl uses CONNECT, not GET):

```bash
curl -x http://127.0.0.1:8080 https://example.com/
```

Plain `http://` URLs via `-x` send GET and get **405**. For HTTP-over-CONNECT you can use curl `--proxytunnel`.
