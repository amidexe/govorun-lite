use serde_json::{json, Value};

pub(super) fn confirmation_path(preview: &Value) -> &'static str {
    if matches!(preview["purchaseKind"].as_str(),
        Some("trial_plan_upgrade" | "scheduled_offer_change")) {
        "/v1/billing/change"
    } else {
        "/v1/billing/tbank/checkout"
    }
}

/// Project only after admission. UI executes this decision without inferring a
/// payment from the purchase kind or recombining catalog and preview flags.
pub(crate) fn project_preview(mut preview: Value) -> Value {
    let path = confirmation_path(&preview);
    preview["clientCheckoutPreview"] = json!({
        "canConfirm": preview["checkoutAvailable"] == true,
        "confirmationPath": path,
        "receiptContactRequired": path == "/v1/billing/tbank/checkout"
    });
    preview
}

pub(super) fn admit_confirmation(preview: &mut Value, catalog: &Value) {
    let bank = confirmation_path(preview) == "/v1/billing/tbank/checkout";
    let channel = !bank || catalog["channels"]["tbank"] == true;
    // A missing server capability is a display-only disclosure. A bank
    // confirmation also requires the fresh account catalog's payment channel.
    preview["checkoutAvailable"] = json!(channel && preview["checkoutAvailable"] == true);
}
