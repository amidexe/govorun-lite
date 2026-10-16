use crate::{BillingStatusSnapshot, CheckoutSnapshot, SessionSnapshot};
use serde::Serialize;

/// Actions offered for the current confirmed state. Execution still rechecks
/// confirmation, identity and the protected checkout journal in the command owner.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BillingActions {
    pub epoch: u64,
    pub cancel_renewal: bool,
    pub cancel_plan_change: bool,
    pub remove_card: bool,
    pub request_refund: bool,
}

impl BillingActions {
    /// Decode an already-admitted action view at the wire boundary.
    /// Displaying an action does not allow it: commands recheck the session and facts.
    pub fn from_projection(projection: &serde_json::Value, epoch: u64) -> Self {
        let closed = Self { epoch, ..Self::default() };
        let value = &projection["billingActions"];
        if value["epoch"].as_u64() != Some(epoch) { return closed; }
        Self {
            cancel_renewal: value["cancelRenewal"] == true,
            cancel_plan_change: value["cancelPlanChange"] == true,
            remove_card: value["removeCard"] == true,
            request_refund: value["requestRefund"] == true,
            ..closed
        }
    }
    pub(crate) fn project(session: &SessionSnapshot, billing: &BillingStatusSnapshot,
        checkout: &CheckoutSnapshot) -> Self {
        let closed = Self { epoch: session.session_generation, ..Self::default() };
        if !session.signed_in || !session.storage_available || billing.epoch != closed.epoch ||
            checkout.epoch != closed.epoch || !billing.fresh || billing.refreshing ||
            billing.mutation_pending || checkout.pending || checkout.working ||
            billing.management.as_ref().is_some_and(|m| m.running || m.needs_check) {
            return closed;
        }
        let Some(status) = &billing.status else { return closed };
        let subscription = &status["subscription"];
        Self {
            cancel_renewal: subscription["renewsAutomatically"] == true,
            cancel_plan_change: crate::catalog::identifier(&subscription["offerId"], 3, true) &&
                crate::catalog::identifier(&subscription["pendingOfferId"], 3, true),
            // The server's active/canRemove pair is authoritative. Removing the
            // payment method also stops renewal; platforms must not add a second prerequisite.
            remove_card: subscription["paymentMethod"]["state"] == "active" &&
                subscription["paymentMethod"]["canRemove"] == true,
            request_refund: true,
            ..closed
        }
    }
}
