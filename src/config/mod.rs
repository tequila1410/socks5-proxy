use std::time::Duration;

pub struct Config {
    pub socks5_host: String,
    pub socks5_port: String,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
}

impl Config {
    pub fn new() -> Self {
        Self {
            socks5_host: std::env::var("SOCKS5_HOST").expect("SOCKS5_HOST is not set"),
            socks5_port: std::env::var("SOCKS5_PORT").expect("SOCKS5_PORT is not set"),
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
        }
    }

    pub fn socks5_addr(&self) -> String {
        format!("{}:{}", self.socks5_host, self.socks5_port)
    }
}