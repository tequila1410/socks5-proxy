pub mod http_connect;
pub mod socks5;

pub struct Target {
    pub host: String,
    pub port: u16,
}
