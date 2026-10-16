//! Typed views of admitted Core documents. Consumers decode generated models;
//! this boundary owns field aliases/defaults, never a second workflow.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::{Error, Result};
mod account;
mod billing;
mod checkout;
mod promotion;
pub use account::*;
pub use billing::*;
pub use checkout::*;
pub use promotion::*;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Credentials, Profile, StyleAccess, OnlineConsent, AccountDevice, StyleUsageBalance,
    StarterGrant, StarterGrantOffer, BillingTrialOffer, BillingCatalog, BillingStatus,
    BillingPlan, BillingSubscription, BillingRenewalIssue, BillingNextCharge,
    BillingRefundRequest, BillingPlanChange, BillingPaymentMethod, BillingStorefront,
    TBankCheckoutPreview, TBankCheckoutSession, TBankLegalDocument, TBankLegalDocuments,
    CheckoutPresentation, BillingCheckoutPresentation, PromoProgram, PromoOffer,
    PromoOfferPreview, PromoRedemption,
}
fn rename(value: &mut Value, old: &str, new: &str) {
    if let Some(fields) = value.as_object_mut() {
        if let Some(selected) = fields.remove(old) { fields.insert(new.into(), selected); }
    }
}
fn grant_flags(value: &mut Value) {
    if value.is_object() {
        value["eligible"] = json!(value["clientCanActivate"].as_bool().unwrap_or(false));
        value.as_object_mut().unwrap().remove("clientCanActivate");
    }
}
fn program_flags(value: &mut Value) {
    if value.is_object() {
        value["visible"] = json!(value["clientVisible"].as_bool().unwrap_or(false));
        value["canActivate"] = json!(value["clientCanActivate"].as_bool().unwrap_or(false));
        let fields = value.as_object_mut().unwrap();
        fields.remove("clientVisible"); fields.remove("clientCanActivate");
    }
}
fn view<T: serde::de::DeserializeOwned + Serialize>(mut value: Value) -> Result<Value> {
    for field in ["starterGrant", "promoProgram"] {
        if value[field].is_object() {
            if field == "starterGrant" { grant_flags(&mut value[field]); }
            else { program_flags(&mut value[field]); }
        }
    }

    // Old nullable optional documents use their declared serde default. Required
    // fields still fail admission; the original server/storage object is untouched.
    fn optional_nulls(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.retain(|_, field| !field.is_null());
                for child in fields.values_mut() { optional_nulls(child); }
            }
            Value::Array(items) => for child in items { optional_nulls(child); },
            _ => (),
        }
    }
    optional_nulls(&mut value);
    let typed: T = serde_json::from_value(value).map_err(|_| Error::InvalidResponse)?;
    serde_json::to_value(typed).map_err(|_| Error::InvalidResponse)
}
pub(crate) fn evaluate(kind: Kind, mut value: Value, version: String) -> Result<Value> {
    use Kind::*;
    match kind {
        Profile => {
            value["styleAccess"] = evaluate(StyleAccess, value["styleAccess"].clone(), String::new())?;
            value["onlineServiceState"] = json!(match value["onlineService"]["state"].as_str() {
                Some("available") => "available", Some("closed") => "closed", _ => "unknown",
            });
            view::<ConsumerProfile>(value)
        }
        StyleAccess => {
            for field in ["type", "state"] { if !value[field].is_string() { return Err(Error::InvalidResponse); } }
            let observed = ["remainingToday", "remainingTotal"].iter().all(|key| !value[*key].is_null());
            value["issuanceCountersObserved"] = json!(observed);
            for field in ["remainingToday", "remainingTotal"] {
                if value[field].is_null() { value[field] = json!(0); }
                if value[field].as_i64().is_none_or(|number| !(0..=i32::MAX as i64).contains(&number)) {
                    return Err(Error::InvalidResponse);
                }
            }
            for field in ["quotaUnit", "stylizationProtocol", "reserveReturnProtocol"] {
                if value[field].as_str().is_some_and(|text| text.trim().is_empty()) { value[field] = Value::Null; }
            }
            if let Some(balance) = value.get("usageBalance").filter(|v| !v.is_null()) {
                crate::usage_ledger::balance_binding::evaluate(json!({"kind":"decode","balance":balance}))?;
            }
            view::<ConsumerStyleAccess>(value)
        }
        BillingCatalog => {
            rename(&mut value, "offers", "plans");
            if let Some(plans) = value["plans"].as_array_mut() {
                for plan in plans { rename(plan,"planCode","code"); rename(plan,"planName","name"); }
            }
            value["ruStoreEnabled"] = json!(value["channels"]["rustore"].as_bool().unwrap_or(false));
            let tbank = value["channels"]["tbank"].as_bool().unwrap_or(false);
            value["tBankEnabled"] = json!(tbank);
            value["tBankPreviewEnabled"] = json!(value["channels"]["tbankPreview"].as_bool().unwrap_or(tbank));
            if value["promoProgram"]["programVersion"].is_null() && value["promoProgram"].is_object() {
                value["promoProgram"]["programVersion"] = json!(value["programControlVersion"].as_str().unwrap_or(""));
            }
            view::<ConsumerBillingCatalog>(value)
        }
        PromoProgram => {
            program_flags(&mut value);
            if value["programVersion"].is_null() { value["programVersion"] = json!(version); }
            view::<ConsumerPromoProgram>(value)
        }
        BillingPaymentMethod => {
            let projected = crate::features::billing::snapshot::payment_method_view(&value, true);
            value["lastFour"] = projected["lastFour"].clone();
            view::<ConsumerBillingPaymentMethod>(value)
        }
        BillingStorefront => {
            if !matches!(value["state"].as_str(), Some("open" | "unavailable")) { return Err(Error::InvalidResponse); }
            view::<ConsumerBillingStorefront>(value)
        }
        TBankCheckoutPreview | TBankCheckoutSession => {
            for field in ["freeAccessEndsAt", "firstChargeAt", "renewalChargeAt", "servicePeriodEndsAt"] {
                if value[field].is_null() { value[field] = json!(""); }
            }
            if matches!(kind, TBankCheckoutPreview) { view::<ConsumerTBankCheckoutPreview>(value) }
            else {
                if value["expiresInSeconds"].as_i64().is_none_or(|n| !(0..=86_400).contains(&n)) {
                    value["expiresInSeconds"] = Value::Null;
                }
                view::<ConsumerTBankCheckoutSession>(value)
            }
        }
        Credentials => view::<crate::Credentials>(value),
        OnlineConsent => view::<ConsumerOnlineConsent>(value),
        AccountDevice => view::<ConsumerAccountDevice>(value),
        StyleUsageBalance => {
            crate::usage_ledger::balance_binding::evaluate(json!({"kind":"decode","balance":value}))?;
            view::<ConsumerStyleUsageBalance>(value)
        }
        StarterGrant => { grant_flags(&mut value); view::<ConsumerStarterGrant>(value) },
        StarterGrantOffer => view::<ConsumerStarterGrantOffer>(value),
        BillingTrialOffer => view::<ConsumerBillingTrialOffer>(value),
        BillingStatus => view::<ConsumerBillingStatus>(value),
        BillingPlan => { rename(&mut value,"planCode","code"); rename(&mut value,"planName","name"); view::<ConsumerBillingPlan>(value) },
        BillingSubscription => view::<ConsumerBillingSubscription>(value),
        BillingRenewalIssue => view::<ConsumerBillingRenewalIssue>(value),
        BillingNextCharge => view::<ConsumerBillingNextCharge>(value),
        BillingRefundRequest => view::<ConsumerBillingRefundRequest>(value),
        BillingPlanChange => view::<ConsumerBillingPlanChange>(value),
        TBankLegalDocument => view::<ConsumerTBankLegalDocument>(value),
        TBankLegalDocuments => view::<ConsumerTBankLegalDocuments>(value),
        CheckoutPresentation => view::<ConsumerCheckoutPresentation>(value),
        BillingCheckoutPresentation => view::<ConsumerBillingCheckoutPresentation>(value),
        PromoOffer => view::<ConsumerPromoOffer>(value),
        PromoOfferPreview => view::<ConsumerPromoOfferPreview>(value),
        PromoRedemption => view::<ConsumerPromoRedemption>(value),
    }
}
