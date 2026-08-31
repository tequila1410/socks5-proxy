use std::future::Future;
use std::io::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::Instrument;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

use crate::config::Config;
use crate::ingress::http_connect::handle_http_connection;
use crate::ingress::socks5::handle_socks5_connection;

const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn run(config: Config) {
    let config = Arc::new(config);
    let semaphore = Arc::new(Semaphore::new(config.connections_limit));
    let join_set = Arc::new(Mutex::new(JoinSet::new()));
    tokio::select! {
        _ = accept_loop(
            config.http_connect_addr(),
            Arc::clone(&config),
            Arc::clone(&semaphore),
            Arc::clone(&join_set),
            handle_http_connection,
        ) => {
            tracing::info!("HTTP CONNECT listener exited, shutting down");
        }
        _ = accept_loop(
            config.socks5_addr(),
            Arc::clone(&config),
            Arc::clone(&semaphore),
            Arc::clone(&join_set),
            handle_socks5_connection,
        ) => {
            tracing::info!("SOCKS5 listener exited, shutting down");
        }
        result = tokio::signal::ctrl_c() => {
            match result {
                Ok(()) => tracing::info!("Ctrl+C received, shutting down"),
                Err(e) => tracing::error!(error = %e, "error listening for Ctrl+C"),
            }
        }
    }

    let mutex = Arc::try_unwrap(join_set)
        .expect("join set still shared after accept loops stopped");
    let join_set = mutex
        .into_inner()
        .expect("join set mutex poisoned");
    drain_join_set(join_set).await;
}

async fn drain_join_set(mut join_set: JoinSet<()>) {
    if join_set.is_empty() {
        return;
    }

    let drain_deadline = tokio::time::sleep(DRAIN_TIMEOUT);
    tokio::pin!(drain_deadline);

    loop {
        tokio::select! {
            _ = &mut drain_deadline => {
                tracing::warn!("drain timeout, aborting remaining connections");
                abort_remaining(&mut join_set).await;
                return;
            }
            result = tokio::signal::ctrl_c() => {
                match result {
                    Ok(()) => tracing::info!("second Ctrl+C, aborting remaining connections"),
                    Err(e) => tracing::error!(error = %e, "error listening for Ctrl+C"),
                }
                abort_remaining(&mut join_set).await;
                return;
            }
            joined = join_set.join_next() => {
                match joined {
                    None => return,
                    Some(Ok(())) => {}
                    Some(Err(e)) => tracing::error!(error = %e, "connection task ended"),
                }
            }
        }
    }
}

async fn abort_remaining(join_set: &mut JoinSet<()>) {
    join_set.abort_all();
    while join_set.join_next().await.is_some() {}
}

async fn accept_loop<F, Fut>(
    addr: String,
    config: Arc<Config>,
    semaphore: Arc<Semaphore>,
    join_set: Arc<Mutex<JoinSet<()>>>,
    handler: F,
) where
    F: Fn(TcpStream, Duration, Duration) -> Fut + Copy + Send + 'static,
    Fut: Future<Output = Result<(), Error>> + Send,
{
    let listener = TcpListener::bind(&addr).await.unwrap();
    tracing::info!(%addr, "listening");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let config = Arc::clone(&config);
                let semaphore = Arc::clone(&semaphore);
                let permit = match semaphore.try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(e) => {
                        tracing::warn!(error = %e, "connection limit reached, dropping");
                        continue;
                    }
                };
                let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
                let mut join_set_lock = join_set.lock().expect("Failed to lock join set");
                join_set_lock.spawn(
                    async move {
                        let _permit = permit;
                        if let Err(e) =
                            handler(stream, config.connect_timeout, config.idle_timeout).await
                        {
                            tracing::error!(error = %e, "connection handler failed");
                        }
                    }
                    .instrument(tracing::info_span!("conn", conn_id)),
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "accept failed");
            }
        }
    }
}
