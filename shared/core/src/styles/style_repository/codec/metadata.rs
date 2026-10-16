//! Existing device enable/consent mirrors. These never authorize server access.
use super::{windows, Error, Source};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Action {
    DismissStarterGuidance,
    SetEnabled {
        enabled: bool,
    },
    RecordConsent {
        version: u32,
        accepted_at_millis: Option<u64>,
    },
}
pub(super) fn read(source: &Source, values: &Value) -> Result<Value, Error> {
    let root = match source {
        Source::Macos => values.clone(),
        Source::Windows => windows::root(values)?,
    };
    let field = |mac, win| match source {
        Source::Macos => root.get(mac).unwrap_or(&Value::Null),
        Source::Windows => windows::field(&root, win),
    };
    let enabled = boolean(field("govorunGatewayEnabled", "stylingEnabled"))?;
    let version = field(
        "govorunGatewayAcceptedTermsVersion",
        "acceptedOnlineTermsVersion",
    );
    let version = if version.is_null() {
        0
    } else {
        version
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::InvalidStorage)?
    };
    let at = match source {
        Source::Windows => None,
        Source::Macos => match root.get("govorunGatewayAcceptedTermsAt") {
            None => None,
            Some(value) => {
                let seconds = value.as_f64().ok_or(Error::InvalidStorage)?;
                if !seconds.is_finite()
                    || seconds < 0.0
                    || seconds * 1000.0 > 9_007_199_254_740_991.0
                {
                    return Err(Error::InvalidStorage);
                }
                Some((seconds * 1000.0).round() as u64)
            }
        },
    };
    let starter = match source {
        Source::Macos => boolean(
            root.get("govorunStarterGuidanceVisibleV1")
                .unwrap_or(&Value::Null),
        )?,
        Source::Windows => boolean(windows::field(&root, "starterGuidanceVisible"))?,
    };
    Ok(
        json!({"enabled":enabled,"localTermsVersion":version,"acceptedAtMillis":at,"starterGuidanceVisible":starter}),
    )
}
pub(super) fn apply(source: &Source, values: &mut Value, action: Value) -> Result<bool, Error> {
    let action: Action = serde_json::from_value(action).map_err(|_| Error::InvalidAction)?;
    let original = read(source, values)?;
    let mut next = original.clone();
    match action {
        Action::DismissStarterGuidance => next["starterGuidanceVisible"] = json!(false),
        Action::SetEnabled { enabled } => next["enabled"] = json!(enabled),
        Action::RecordConsent {
            version,
            accepted_at_millis,
        } => {
            if version == 0 || accepted_at_millis.is_some_and(|n| n > 9_007_199_254_740_991) {
                return Err(Error::InvalidAction);
            }
            next["localTermsVersion"] = json!(version);
            next["acceptedAtMillis"] = match source {
                Source::Macos => json!(accepted_at_millis),
                Source::Windows => Value::Null,
            };
            if matches!(source, Source::Windows) {
                next["starterGuidanceVisible"] = json!(false);
            }
        }
    }
    if next == original {
        return Ok(false);
    }
    match source {
        Source::Macos => {
            let object = values.as_object_mut().ok_or(Error::InvalidStorage)?;
            for (field, key) in [
                ("starterGuidanceVisible", "govorunStarterGuidanceVisibleV1"),
                ("enabled", "govorunGatewayEnabled"),
                ("localTermsVersion", "govorunGatewayAcceptedTermsVersion"),
            ] {
                if next[field] != original[field] {
                    object.insert(key.into(), next[field].clone());
                }
            }
            if next["acceptedAtMillis"] != original["acceptedAtMillis"] {
                match next["acceptedAtMillis"].as_u64() {
                    Some(millis) => {
                        object.insert(
                            "govorunGatewayAcceptedTermsAt".into(),
                            // JSON/NSNumber ports emit integral seconds without
                            // a fractional suffix. Prepare that same number so
                            // the durable ACK remains exact across bindings.
                            if millis % 1000 == 0 { json!(millis / 1000) }
                            else { json!(millis as f64 / 1000.0) },
                        );
                    }
                    None => {
                        object.remove("govorunGatewayAcceptedTermsAt");
                    }
                }
            }
        }
        Source::Windows => {
            let mut root = windows::root(values)?;
            for (field, key) in [
                ("enabled", "stylingEnabled"),
                ("localTermsVersion", "acceptedOnlineTermsVersion"),
                ("starterGuidanceVisible", "starterGuidanceVisible"),
            ] {
                if next[field] != original[field] {
                    windows::put(&mut root, key, next[field].clone());
                }
            }
            *values = json!(root.to_string());
        }
    }
    Ok(true)
}
fn boolean(value: &Value) -> Result<bool, Error> {
    if value.is_null() {
        Ok(false)
    } else {
        value.as_bool().ok_or(Error::InvalidStorage)
    }
}
