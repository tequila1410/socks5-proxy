use std::time::Duration;

pub struct Config {
    pub proxy_host: String,
    pub socks5_port: u16,
    pub http_connect_port: u16,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
}

impl Config {
    pub fn new() -> Self {
        let proxy_host = std::env::var("PROXY_HOST").expect("PROXY_HOST is not set");
        let socks5_port = std::env::var("SOCKS5_PORT").expect("SOCKS5_PORT is not set").parse::<u16>().expect("SOCKS5_PORT is not a valid port");
        let http_connect_port = std::env::var("HTTP_CONNECT_PORT").expect("HTTP_CONNECT_PORT is not set").parse::<u16>().expect("HTTP_CONNECT_PORT is not a valid port");
        
        Self {
            proxy_host,
            socks5_port,
            http_connect_port,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
        }
    }

    pub fn socks5_addr(&self) -> String {
        format!("{}:{}", self.proxy_host, self.socks5_port)
    }

    pub fn http_connect_addr(&self) -> String {
        format!("{}:{}", self.proxy_host, self.http_connect_port)
    }
}