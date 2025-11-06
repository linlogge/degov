use std::future::pending;

use dgv_kube_operator::KubeOperator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let kube_operator = KubeOperator::new();
    kube_operator.run().await?;

    pending::<()>().await;

    Ok(())
}
