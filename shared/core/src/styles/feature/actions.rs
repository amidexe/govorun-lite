//! Closed UI commands map to the same durable library owner as sync.
use super::{view::TextStyleOptions, StylesFeature};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum Action {
    Select {
        selection: String,
        activation_options: Option<TextStyleOptions>,
    },
    AcceptTerms {
        selection: String,
        terms_version: u32,
        confirmed: bool,
        activation_options: Option<TextStyleOptions>,
    },
    Disable {},
    Cancel {},
    Refresh {},
    Documents {},
    LoadApplications {},
    SavePreset {
        id: Option<String>,
        name: String,
        instruction: String,
        options: Option<TextStyleOptions>,
    },
    DeletePreset {
        id: String,
        confirmed: bool,
    },
    Option {
        name: String,
        value: Value,
    },
    SaveApplicationRule {
        id: String,
        style: Option<String>,
        custom_options: bool,
        emoji_mode: Option<String>,
        reduced_punctuation: Option<bool>,
    },
    DeleteApplicationRule {
        id: String,
        confirmed: bool,
    },
}
impl StylesFeature {
    pub async fn action(
        self: &std::sync::Arc<Self>,
        mut params: Value,
    ) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        let scope = params
            .as_object_mut()
            .ok_or("styles_invalid_request")?
            .remove("scope")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or("styles_invalid_request")?;
        let action: Action =
            serde_json::from_value(params).map_err(|_| "styles_invalid_request")?;
        let before = self.read_snapshot().await?;
        if before["scope"] != scope {
            return Err("styles_session_changed");
        }
        if before["displayState"] == "checking" {
            return Err("styles_busy");
        }
        if let Some(view) = self.activation_action(&action, &before).await? {
            self.publish_view(view.clone());
            return Ok(view);
        }
        if before["busy"] == true {
            return Err("styles_busy");
        }
        if matches!(
            before["gate"].as_str(),
            Some("selfHosted" | "selfHostedSetup")
        ) {
            return Err("styles_invalid_request");
        }
        let library = self
            .library
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        // A fresh authority frame after the cache ACK fences stale UI commands.
        let current = self.read_snapshot().await?;
        if current["scope"] != scope {
            return Err("styles_session_changed");
        }
        if current["displayState"] == "checking" {
            return Err("styles_busy");
        }
        let command = match action {
            Action::SavePreset {
                id,
                name,
                instruction,
                options,
            } => {
                let options = options.map(|v| {
                    json!({"emojiMode":v.emoji_mode,
                    "punctuationMode":if v.reduced_punctuation { "less" } else { "normal" },
                    "profanityFilterEnabled":v.profanity_filter})
                });
                json!({"kind":"save","id":id,"newId":new_id()?,"name":name,"instruction":instruction,"options":options,"selectSaved":false})
            }
            Action::DeletePreset { id, confirmed } => {
                if !confirmed {
                    return Err("styles_invalid_request");
                }
                json!({"kind":"deleteWithReferences","id":id})
            }
            Action::Option { name, value } => {
                json!({"kind":"setSelectedOption","name":name,"value":value})
            }
            Action::SaveApplicationRule {
                id,
                style,
                custom_options,
                emoji_mode,
                reduced_punctuation,
            } => {
                // Names are OS catalog facts or already stored device facts; UI
                // cannot inject a name or an arbitrary replacement rule set.
                let name = before["applications"]
                    .as_array()
                    .and_then(|v| v.iter().find(|v| v["id"] == id))
                    .or_else(|| {
                        before["applicationRules"]
                            .as_array()
                            .and_then(|v| v.iter().find(|v| v["id"] == id))
                    })
                    .and_then(|v| v["name"].as_str())
                    .ok_or("styles_invalid_request")?;
                if custom_options && (emoji_mode.is_none() || reduced_punctuation.is_none()) {
                    return Err("styles_invalid_request");
                }
                json!({"kind":"saveApplicationRule","rule":{"applicationId":id,"applicationName":name,"styleValue":style,
                    "emojiEnabled":null,"emojiMode":if custom_options { emoji_mode } else { None },
                    "reducedPunctuationEnabled":if custom_options { reduced_punctuation } else { None }}})
            }
            Action::DeleteApplicationRule { id, confirmed } => {
                json!({"kind":"deleteApplicationRule","id":id,"confirmed":confirmed})
            }
            _ => return Err("unsupported_command"),
        };
        let revision = library["configuration"]["revision"]
            .as_str()
            .ok_or("styles_invalid_response")?
            .to_owned();
        let result = self.library.execute(command, Some(revision), true).await?;
        if result["accepted"] != true {
            return Err("styles_invalid_request");
        }
        let view = self.read_snapshot().await?;
        self.publish_view(view.clone());
        Ok(view)
    }
}
fn new_id() -> Result<String, &'static str> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "styles_clock_unavailable")?
        .as_nanos();
    // Core rejects any existing ID; even a clock rollback cannot overwrite a preset.
    Ok(format!(
        "custom_{nanos:x}_{:x}_{:x}",
        crate::instance::nonce(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
