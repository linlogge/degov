use dgv_frontdoor::{Server, ServicesConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let cancel_fut = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    println!("Starting frontdoor server");

    let server = Server::builder()
        .with_listen_address("0.0.0.0:8080".parse().unwrap())
        .build()?;

    server
        .serve(ServicesConfig::default())
        .with_graceful_shutdown(cancel_fut)
        .await?;

    Ok(())
}
