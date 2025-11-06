use std::future::pending;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    pending::<()>().await;

    Ok(())
}
