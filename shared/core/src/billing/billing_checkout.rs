use serde::Serialize;
use serde_json::Value;

/// Current subscription facts may supersede a historical checkout notice.
/// This view never acknowledges a payment or grants access. Native
/// adapters select the receipt disposition and render the resulting directive.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BillingCheckoutPresentation {
    pub settled: CheckoutMessageUpdate,
    pub scheduled: CheckoutMessageUpdate,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CheckoutMessageUpdate {
    pub replace: bool,
    pub message: Option<String>,
    pub refresh_status: bool,
    pub refresh_access: bool,
}

impl CheckoutMessageUpdate {
    fn preserve(refresh_status: bool) -> Self {
        Self { replace: false, message: None, refresh_status, refresh_access: false }
    }
    fn replace(message: Option<String>, refresh_access: bool) -> Self {
        Self { replace: true, message, refresh_status: false, refresh_access }
    }
}

impl BillingCheckoutPresentation {
    pub(crate) fn from_status(status: Option<&Value>, fresh: bool) -> Self {
        let mut value = Self {
            settled: CheckoutMessageUpdate::preserve(false),
            scheduled: CheckoutMessageUpdate::preserve(true),
        };
        let Some(status) = status.filter(|_| fresh) else { return value };
        let subscription = &status["subscription"];
        if subscription["state"] == "refunded" {
            value.settled = CheckoutMessageUpdate::replace(None, false);
        }
        value.scheduled = match subscription["state"].as_str() {
            Some("active") => {
                let message = match subscription["planCode"].as_str() {
                    Some("base") => "Оплата подтверждена. Подключён тариф «Базовая».",
                    Some("pro") => "Оплата подтверждена. Подключён тариф «Про».",
                    // A future product code must never be mislabeled as Base.
                    _ => "Подписка активна.",
                };
                CheckoutMessageUpdate::replace(Some(message.into()), true)
            }
            Some("none" | "grace" | "cancelled" | "expired" | "refunded") =>
                CheckoutMessageUpdate::replace(None, false),
            // Pending, trial and future states do not prove that the scheduled
            // charge completed or disappeared. A stale read cannot decide it.
            _ => CheckoutMessageUpdate::preserve(true),
        };
        value
    }
}
