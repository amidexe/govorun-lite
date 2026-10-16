use super::AccountHost;
use crate::features::ports::Reply;
use serde_json::Value;

impl AccountHost {
    pub async fn billing(&self, method: &str, params: Value) -> Reply {
        self.billing
            .get()
            .ok_or("account_storage_unavailable")?
            .invoke(method, params)
            .await
    }
    pub async fn publish_billing(&self) {
        if let Some(billing) = self.billing.get() {
            billing.publish().await;
        }
    }
}
