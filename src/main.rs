mod runtime;
mod config;
mod ingress;
mod tunnel;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let config = Config::new();
    runtime::run(config).await;
}
