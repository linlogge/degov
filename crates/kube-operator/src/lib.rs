use std::future::pending;

use k8s_openapi::{api::core::v1::ConfigMap, serde_json};
use kube::{Api, Client, api::PostParams};

pub struct KubeOperator {}

impl KubeOperator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::try_default().await?;

        let config: ConfigMap = serde_json::from_value(serde_json::json!({
            "apiVersion": "v1",
            "kind": "ConfigMap",
            "metadata": {
                "name": "kube-operator",
                "namespace": "default",
                "labels": {
                    "app": "kube-operator"
                },
                "annotations": {
                    "app.kubernetes.io/name": "kube-operator"
                }
            },
            "data": {
                "test": "test"
            }
        }))?;

        let config_api: Api<ConfigMap> = Api::default_namespaced(client);

        config_api.create(&PostParams::default(), &config).await?;

        pending::<()>().await;

        Ok(())
    }
}
