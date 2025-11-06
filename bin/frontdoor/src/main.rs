use std::time::Duration;

use dgv_frontdoor::{Server, ServicesConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let cancel_fut = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    let server = Server::builder()
        .with_listen_address("0.0.0.0:8080".parse().unwrap())
        .build()?;

    let (config_sender, serve_watch) = server.serve_watch();

    tokio::spawn(async move {
        loop {
            let config_sender = config_sender.clone();
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = config_sender.send(ServicesConfig::default());
        }
    });

    serve_watch.with_graceful_shutdown(cancel_fut).await?;

    Ok(())
}
