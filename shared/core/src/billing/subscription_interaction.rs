use crate::Result;
use serde_json::{json, Value};

/// Subscription screen intents; only the server confirms payments.
#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "SubscriptionInteractionAction"))]
#[serde(rename_all = "camelCase")]
pub enum Action { Authenticate, StartFreeAccess, ResumeFreeAccess, FreeAccessCompleted, ChoosePlan }

/// Published booleans used by the interaction planner; no other server fields are retained.
#[derive(Clone, Copy, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Access {
    signed_in: bool,
    has_entitlement: bool,
    starter_eligible: bool,
    participated: bool,
}
impl<'de> serde::Deserialize<'de> for Access {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        let wire = Value::deserialize(decoder)?;
        // Match the published wire convention exactly: only literal true is affirmative.
        Ok(Self { signed_in: wire["signedIn"] == true, has_entitlement: wire["hasEntitlement"] == true,
            starter_eligible: wire["starterEligible"] == true, participated: wire["participated"] == true })
    }
}

/// Window decisions are shared, while dialogs, browser calls and their session leases
/// remain native effects. Actual admission and changes still require the account server.
pub(crate) fn interaction(action: Action, access: &Access, accepted: bool, pending: bool) -> Result<Value> {
    let signed_in = access.signed_in;
    let mut result = json!({"kind":"none", "message":null});
    let kind = match action {
        Action::Authenticate => if signed_in { "continue" } else { "signIn" },
        Action::StartFreeAccess => if signed_in { "activate" } else { "rememberAndSignIn" },
        Action::ResumeFreeAccess if !pending => "none",
        Action::ResumeFreeAccess if !signed_in => {
            result["message"] = json!("Вход отменён. Бесплатный лимит не подключён");
            "clearHandoff"
        }
        Action::ResumeFreeAccess if !access.has_entitlement && access.starter_eligible => "activate",
        Action::ResumeFreeAccess => {
            if !access.has_entitlement {
                result["message"] = json!("Бесплатный доступ сейчас недоступен. Обновляем условия.");
            }
            "clearAndRefresh"
        }
        Action::FreeAccessCompleted => {
            if accepted { result["message"] = json!(if access.participated {
                "Промо подключено" } else { "Бесплатный лимит подключён" }); }
            "complete"
        }
        Action::ChoosePlan if !signed_in => "signIn",
        Action::ChoosePlan => "previewBank",
    };
    result["kind"] = json!(kind);
    Ok(result)
}
