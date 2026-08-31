mod runtime;
mod config;
mod ingress;
mod tunnel;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::new();
    tracing::info!(
        socks5 = %config.socks5_addr(),
        http_connect = %config.http_connect_addr(),
        "starting"
    );
    runtime::run(config).await;
}
