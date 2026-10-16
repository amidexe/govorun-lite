use super::state::Operation;
use crate::{AccountSession, Error, PromotionEffect, Result};
use serde_json::{json, Value};

impl AccountSession {
    pub(crate) fn promotion_scope(&self, epoch: u64) -> String { format!("subscription:{epoch}") }

    /// Sanitized window values, never the redeem/checkout capability from the server.
    pub(crate) fn promotion_snapshot(&self, epoch: u64) -> Value {
        let mut value = json!({"epoch":epoch,"scope":self.promotion_scope(epoch), "busy":false,
            "message":null, "promo":null, "promoAccess":null});
        self.inner.promotion_window.decorate(epoch, &mut value);
        value
    }

    pub fn decorate_promotions(&self, snapshot: &mut Value) {
        let window = self.promotion_snapshot(self.snapshot().session_generation);
        snapshot["promo"] = window["promo"].clone();
        snapshot["promoAccess"] = window["promoAccess"].clone();
        if window["busy"] == true { snapshot["busy"] = json!(true); }
        if window["message"].is_string() { snapshot["message"] = window["message"].clone(); }
    }

    pub fn promotion_busy(&self) -> bool { self.inner.promotion_window.busy() }
    pub fn clear_promotion_message(&self) { self.inner.promotion_window.clear_message(); }

    pub(super) async fn subscription_promote(&self, operation: Operation, command: &Value) -> Result<Value> {
        // Validate native capabilities before a redeem can be accepted. They do
        // not authorize payment or replace the server-selected promotion offer.
        super::client::Client::read(command)?;
        let effect = self.inner.promotion_window.execute(self, operation.epoch,
            &self.promotion_scope(operation.epoch), command).await?;
        self.current(operation.epoch)?;
        self.inner.subscription.require(operation)?;
        match effect {
            PromotionEffect::None => Ok(Value::Null),
            PromotionEffect::Redeemed(access) => Ok(json!({"kind":"promotionAccess", "access":access,
                "continuationId":self.promotion_snapshot(operation.epoch)["promoAccess"]["id"]})),
            PromotionEffect::Checkout { offer_id, preview_id } => {
                let mut checkout = json!({"action":"previewCheckout", "offerId":offer_id,
                    "promoPreviewId":preview_id, "userInitiated":true});
                for key in ["client", "displayedCatalog"] {
                    if let Some(value) = command.get(key) { checkout[key] = value.clone(); }
                }
                self.subscription_prepare(operation, &checkout).await
            }
            PromotionEffect::Rejected(_) => Err(Error::InvalidResponse),
        }
    }
}
