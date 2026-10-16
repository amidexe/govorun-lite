use crate::features::dto::dto;
use serde_json::{json, Value};

dto!(TextStyleOptions {
    emoji_mode: String,
    reduced_punctuation: bool,
    profanity_filter: bool
});
dto!(StyleChoice {
    id: String,
    title: String,
    detail: String,
    options: TextStyleOptions
});
dto!(StylePreset {
    id: String,
    name: String,
    instruction: String,
    options: TextStyleOptions
});
dto!(ApplicationChoice {
    id: String,
    name: String
});
dto!(ApplicationRule { id:String, name:String, style:Option<String>, emoji_mode:Option<String>, reduced_punctuation:Option<bool> });
dto!(StylesSnapshot {
    scope:String, busy:bool, message:Option<String>, gate:String, availability:Option<String>,
    online_service_paused:bool, processing_readiness:String, enabled:bool, selected:Option<String>, title:String,
    styles:Vec<StyleChoice>, presets:Vec<StylePreset>, application_rules:Vec<ApplicationRule>, applications:Vec<ApplicationChoice>,
    availability_reason:String, display_state:String, limit_reason:String, resume_choice:bool,
    max_presets:u64, can_create_preset:bool, options:TextStyleOptions, terms_version:u32, terms_accepted:bool,
    processing_limits:crate::dictation_plan::ProcessingLimits
});
/// Access facts are composed with the authoritative AccountSession before this
/// read-only view. Only the resulting public view crosses the UI bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub access: Value,
    pub self_hosted_title: String,
    pub applications: Vec<ApplicationChoice>,
}
const BUILT_INS: [(&str, &str, &str); 3] = [
    (
        "formal",
        "Связно",
        "Перефразирует неудачные места и оформляет речь как готовый текст.",
    ),
    (
        "light",
        "Лёгкая правка",
        "Исправляет явные ошибки и пунктуацию, почти не меняя ваши слова.",
    ),
    (
        "concise",
        "Короче",
        "Исправляет текст, убирает вводные и повторы, оставляет главное.",
    ),
];
pub(super) fn checking(mut value: Value) -> Value {
    value["busy"] = json!(true);
    value["gate"] = json!("unavailable");
    value["availability"] = json!(crate::style_activation::CHECKING_AVAILABILITY);
    value["availabilityReason"] = json!("checking");
    value["displayState"] = json!("checking");
    value["processingReadiness"] = json!("unavailable");
    value["onlineServicePaused"] = json!(false);
    value["limitReason"] = json!("none");
    value["canCreatePreset"] = json!(false);
    value["resumeChoice"] = json!(false);
    value
}
pub(super) fn initial_checking(library: Value) -> Result<Value, &'static str> {
    // The stored terms version is a shape placeholder only: busy/checking blocks
    // actions until a current account frame supplies the actual terms version.
    let terms = library["configuration"]["metadata"]["localTermsVersion"].as_u64()
        .ok_or("styles_invalid_response")?.max(1);
    let facts = Facts {
        access: json!({"generation":0,"signedIn":false,"profileFresh":false,
            "storageAvailable":false,"permissionsReady":false,"termsMatch":false,
            "entitled":false,"starterLocallyExhausted":false,"authorized":false,
            "consentAccepted":false,"processingReady":false,"availability":"checking",
            "selfHosted":false,"selfHostedReady":false,"clientTermsVersion":terms,
            "localTermsVersion":terms,"enabled":library["configuration"]["metadata"]["enabled"],
            "paused":false,"processingReadiness":"unavailable","presetCount":0}),
        self_hosted_title: String::new(), applications: Vec::new(),
    };
    serde_json::to_value(compose(library, facts, "styles:checking".into(), true, None)?)
        .map_err(|_| "styles_invalid_response")
}
pub fn compose(
    library: Value,
    mut facts: Facts,
    scope: String,
    busy: bool,
    message: Option<String>,
) -> Result<StylesSnapshot, &'static str> {
    let invalid = "styles_invalid_response";
    if scope.is_empty() || scope.len() > 160 {
        return Err(invalid);
    }
    let context = &library["configuration"]["context"];
    let projection = &library["projection"];
    let catalog = context["state"]["presets"].as_array().ok_or(invalid)?;
    facts.access["presetCount"] = json!(catalog.len());
    let response = govorun_core::evaluate_business_command(
        &json!({"operation":"styleAccessPresentation","facts":facts.access}).to_string(),
    );
    let response: Value = serde_json::from_str(&response).map_err(|_| invalid)?;
    let access = response.get("result").ok_or(invalid)?;
    let hosted = facts.access["selfHosted"].as_bool().ok_or(invalid)?;
    let enabled = facts.access["enabled"].as_bool().ok_or(invalid)?;
    let selected = if hosted {
        Some("selfHosted".into())
    } else {
        serde_json::from_value::<Option<String>>(projection["selected"].clone())
            .map_err(|_| invalid)?
    };
    let title = if !enabled {
        "Без стилизации".into()
    } else if hosted {
        format!("{} · свой сервер", facts.self_hosted_title)
    } else if let Some(selected) = &selected {
        if let Some(id) = selected.strip_prefix("custom:") {
            let preset = catalog.iter().find(|v| v["id"] == id).ok_or(invalid)?;
            string(&preset["name"])?
        } else {
            BUILT_INS
                .iter()
                .find(|(id, _, _)| id == selected)
                .map(|(_, title, _)| title.to_string())
                .ok_or(invalid)?
        }
    } else {
        "Стиль не выбран".into()
    };
    let by_selection = &projection["optionsBySelection"];
    let styles = BUILT_INS
        .iter()
        .map(|(id, title, detail)| {
            Ok(StyleChoice {
                id: (*id).into(),
                title: (*title).into(),
                detail: (*detail).into(),
                options: options(&by_selection[*id])?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let presets = catalog
        .iter()
        .map(|preset| {
            let id = string(&preset["id"])?;
            Ok(StylePreset {
                options: options(&by_selection[format!("custom:{id}")])?,
                id,
                name: string(&preset["name"])?,
                instruction: string(&preset["instruction"])?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let rules = library["configuration"]["applicationRules"]
        .as_array()
        .ok_or(invalid)?;
    let application_rules=rules.iter().map(|rule|{
        let emoji=if !rule["emojiMode"].is_null() { rule["emojiMode"].clone() } else {
            rule["emojiEnabled"].as_bool().map(|v|json!(if v{"bright"}else{"off"})).unwrap_or(Value::Null)
        };
        serde_json::from_value(json!({"id":rule["applicationId"],"name":rule["applicationName"],"style":rule["styleValue"],
            "emojiMode":emoji,"reducedPunctuation":rule["reducedPunctuationEnabled"]})).map_err(|_|invalid)
    }).collect::<Result<Vec<_>,_>>()?;
    let current = if projection["selectedOptions"].is_null() {
        &context["currentOptions"]
    } else {
        &projection["selectedOptions"]
    };
    Ok(StylesSnapshot {
        scope,
        processing_limits: crate::dictation_plan::processing_limits(),
        busy,
        message,
        gate: string(&access["gate"])?,
        availability: serde_json::from_value(access["availability"].clone())
            .map_err(|_| invalid)?,
        online_service_paused: enabled && !hosted && facts.access["paused"] == true,
        processing_readiness: string(&facts.access["processingReadiness"])?,
        enabled,
        selected,
        title,
        styles,
        presets,
        application_rules,
        applications: facts.applications,
        availability_reason: string(&access["availabilityReason"])?,
        display_state: string(&access["displayState"])?,
        limit_reason: string(&access["limitReason"])?,
        resume_choice: access["resumeChoice"].as_bool().ok_or(invalid)?,
        max_presets: access["maxPresets"].as_u64().ok_or(invalid)?,
        can_create_preset: access["canCreatePreset"].as_bool().ok_or(invalid)?,
        options: options(current)?,
        terms_version: facts.access["clientTermsVersion"]
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(invalid)?,
        terms_accepted: access["termsAccepted"].as_bool().ok_or(invalid)?,
    })
}
fn string(value: &Value) -> Result<String, &'static str> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or("styles_invalid_response")
}
fn options(value: &Value) -> Result<TextStyleOptions, &'static str> {
    let punctuation = value["punctuationMode"]
        .as_str()
        .ok_or("styles_invalid_response")?;
    if !matches!(punctuation, "normal" | "less") {
        return Err("styles_invalid_response");
    }
    let emoji_mode = string(&value["emojiMode"])?;
    if !matches!(
        emoji_mode.as_str(),
        "off" | "restrained" | "lively" | "bright"
    ) {
        return Err("styles_invalid_response");
    }
    Ok(TextStyleOptions {
        emoji_mode,
        reduced_punctuation: punctuation == "less",
        profanity_filter: value["profanityFilterEnabled"]
            .as_bool()
            .ok_or("styles_invalid_response")?,
    })
}
