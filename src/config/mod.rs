pub struct Config {
    pub socks5_host: String,
    pub socks5_port: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            socks5_host: std::env::var("SOCKS5_HOST").expect("SOCKS5_HOST is not set"),
            socks5_port: std::env::var("SOCKS5_PORT").expect("SOCKS5_PORT is not set"),
        }
    }

    pub fn socks5_addr(&self) -> String {
        format!("{}:{}", self.socks5_host, self.socks5_port)
    }
}