//! Build-only serde schema exporter; production has no generator dependency.
use govorun_core::features::{account, billing, dictionary, statistics, styles, about};
use serde_json::json;
use govorun_core::consumer_models::*;
#[path = "schema/consumer_models.rs"]
mod consumer_schema;
#[path = "schema/style_paths.rs"]
mod style_paths;

fn main() {
    let mut types = serde_json::Map::new();
    let mut features = serde_json::Map::new();
    macro_rules! export {
        ($feature:literal, $($ty:ty),+ $(,)?) => {{
            let mut names = Vec::new();
            $(let schema = schemars::schema_for!($ty);
              let name = schema.schema.metadata.as_ref().unwrap().title.as_ref().unwrap().clone();
              names.push(name.clone());
              types.insert(name, serde_json::to_value(schema).unwrap());)+
            features.insert($feature.into(), json!(names));
        }};
    }
    export!("account", account::dto::AccountProvider, account::dto::AccountDevice, account::dto::AccountStorage, account::dto::AccountSnapshot);
    export!("billing", billing::dto::BillingBalance, billing::dto::BillingPlan, billing::dto::BillingStorefront, billing::dto::BillingStatus, billing::dto::BillingCheckout, billing::dto::BillingSnapshot, billing::dto::PromoProgram, billing::dto::PromoPreview, billing::dto::BillingPromotionAccess);
    export!("dictionary", dictionary::DictionaryEntry, dictionary::DictionarySnapshot, dictionary::DictionaryToggle);
    export!("statistics", statistics::StatisticsTotal, statistics::StatisticsDay, statistics::StatisticsSnapshot, statistics::StatisticsReset);
    export!("styles", styles::view::TextStyleOptions, styles::view::StyleChoice, styles::view::StylePreset, styles::view::ApplicationChoice, styles::view::ApplicationRule, styles::view::StylesSnapshot);
    export!("about", about::NativeBuildFacts, about::ApplicationLicense, about::AboutSnapshot, about::AboutOpenLink);
    export!("access-actions", govorun_core::AccessInteractionAction, govorun_core::SubscriptionInteractionAction);
    export!("hotkeys", govorun_core::hotkeys::workflow::HotkeySnapshot);
    export!("clipboard", govorun_core::clipboard_settings::ClipboardSnapshot);
    export!("self-host", govorun_core::self_hosted_flow::controller::SelfHostSnapshot);
    export!("diagnostics", govorun_core::features::diagnostics::DiagnosticsSnapshot);
    export!("benchmark", govorun_core::benchmark_report::Snapshot, govorun_core::benchmark_report::Timing, govorun_core::benchmark_report::Measurement, govorun_core::benchmark_report::Device);
    export!("dictation", govorun_core::permission_setup::Snapshot);
    export!("recording-diagnostics", govorun_core::recording_support::Snapshot);
    export!("appearance", govorun_core::boundary_snapshots::AppearanceSnapshot, govorun_core::boundary_snapshots::ThemeMode);
    export!("setup", govorun_core::boundary_snapshots::SetupSnapshot, govorun_core::boundary_snapshots::SetupProgress, govorun_core::boundary_snapshots::SetupStep);
    export!("updates", govorun_core::boundary_snapshots::UpdateSnapshot);
    export!("consumer-models", govorun_core::Credentials, ConsumerStyleAccess, ConsumerOnlineConsent, ConsumerAccountDevice, ConsumerStarterGrant, ConsumerPromoProgram, ConsumerBillingStorefront, ConsumerBillingPaymentMethod, ConsumerPromoAccessDays, ConsumerPromoBillingPeriod, ConsumerPromoBonus, ConsumerPromoOffer, ConsumerPromoOfferPreview, ConsumerPromoRedemption, ConsumerBillingPlan, ConsumerBillingCatalog, ConsumerStarterGrantOffer, ConsumerBillingTrialOffer, ConsumerBillingStatus, ConsumerBillingSubscription, ConsumerBillingRenewalIssue, ConsumerBillingNextCharge, ConsumerBillingRefundRequest, ConsumerBillingPlanChange, ConsumerTBankCheckoutPreview, ConsumerCheckoutPreviewAction, ConsumerTBankLegalDocument, ConsumerTBankLegalDocuments, ConsumerTBankCheckoutSession, ConsumerCheckoutPresentation, ConsumerCheckoutMessageUpdate, ConsumerBillingCheckoutPresentation, ConsumerProfile, ConsumerStyleUsageBalance, ConsumerStyleRequestCount);
    let defaults = consumer_schema::constructor_defaults();
    let field_order = consumer_schema::field_order();
    for name in features["consumer-models"].as_array().unwrap() {
        let key = name.as_str().unwrap();
        types[key]["x-consumer-view"] = json!(true);
        types[key]["x-consumer-field-order"] = field_order[key].clone();
        types[key]["x-kotlin-defaults"] = defaults[key].as_object().map(|v| json!(v)).unwrap_or_else(|| json!({}));
    }
    types.insert("OfficialStylePath".into(), style_paths::schema());
    features.insert("styles-transport".into(), json!(["OfficialStylePath"]));
    let schema = govorun_core::command_schema();
    types.insert("BusinessCommand".into(), serde_json::to_value(schema).unwrap());
    features.insert("business-command".into(), json!(["BusinessCommand"]));
    println!("{}", serde_json::to_string_pretty(&json!({"schemaVersion":1,"features":features,"types":types})).unwrap());
}
