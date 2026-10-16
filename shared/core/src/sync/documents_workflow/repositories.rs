//! Synchronization uses the exact document owners used by local commands.
use super::{Local, Scope, Stage, Work};
use crate::features::{
    dictionary::DictionaryFeature, statistics::StatisticsFeature,
    style_library::StyleLibraryFeature,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct Repositories {
    pub dictionary: Arc<DictionaryFeature>,
    pub styles: Arc<StyleLibraryFeature>,
    pub statistics: Arc<StatisticsFeature>,
}
impl Local for Repositories {
    fn snapshot<'a>(&'a self, stage: Stage, _: &'a Scope) -> Work<'a> {
        Box::pin(async move {
            let library = self
                .styles
                .execute(json!({"kind":"snapshot"}), None, false)
                .await?;
            let context = &library["configuration"]["context"];
            match stage {
                Stage::Content => {
                    let dictionary = self
                        .dictionary
                        .execute(json!({"kind":"snapshot"}), None, false)
                        .await?;
                    let selected = if context["state"]["selectedStyle"] == "custom" {
                        library["projection"]["activeId"].as_str().unwrap_or("")
                    } else {
                        ""
                    };
                    let document = json!({"schemaVersion":1,"dictionary":{
                        "enabled":dictionary["storage"]["enabled"],"text":dictionary["storage"]["text"]},
                        "selectedCustomStyleId":selected,"customStyles":context["state"]["presets"]});
                    Ok(evaluate(
                        "contentSync",
                        json!({"kind":"validate","document":document}),
                    )?["document"]
                        .clone())
                }
                Stage::Preferences => Ok(json!({"defaultStyle":context["state"]["selectedStyle"],
                    "punctuationMode":context["currentOptions"]["punctuationMode"],
                    "emojiMode":context["currentOptions"]["emojiMode"],
                    "profanityFilterEnabled":context["currentOptions"]["profanityFilterEnabled"]})),
                _ => Err("sync_invalid_request"),
            }
        })
    }
    fn apply<'a>(&'a self, stage: Stage, _: &'a Scope, value: Value) -> Work<'a> {
        Box::pin(async move {
            match stage {
                Stage::Content => {
                    let value =
                        evaluate("contentSync", json!({"kind":"validate","document":value}))?
                            ["document"]
                            .clone();
                    let library = self
                        .styles
                        .execute(json!({"kind":"snapshot"}), None, false)
                        .await?;
                    let action = json!({"kind":"replace","presets":value["customStyles"],"selectedId":value["selectedCustomStyleId"]});
                    // Validate the catalog transition before the first document
                    // write. Storage/ACK failures retain the old journal for retry.
                    let mut command = action.clone();
                    command["state"] = library["configuration"]["context"]["state"].clone();
                    let prepared = evaluate("styleLibrary", command)?;
                    if prepared["accepted"] != true {
                        return Err("styles_invalid_request");
                    }
                    self.dictionary
                        .execute(
                            json!({"kind":"replaceText","raw":value["dictionary"]["text"],
                        "enabled":value["dictionary"]["enabled"]}),
                            None,
                            false,
                        )
                        .await?;
                    self.styles
                        .execute(
                            action,
                            Some(
                                library["configuration"]["revision"]
                                    .as_str()
                                    .ok_or("styles_invalid_response")?
                                    .into(),
                            ),
                            false,
                        )
                        .await
                }
                Stage::Preferences => {
                    self.styles
                        .execute(
                            json!({"kind":"selectStyle","selection":value["defaultStyle"],
                        "activationOptions":{"emojiMode":value["emojiMode"],
                        "reducedPunctuation":value["punctuationMode"]=="less",
                        "profanityFilter":value["profanityFilterEnabled"]}}),
                            None,
                            false,
                        )
                        .await
                }
                _ => Err("sync_invalid_request"),
            }
        })
    }
    fn statistics<'a>(&'a self, _: &'a Scope, event: Value) -> Work<'a> {
        Box::pin(async move {
            Ok(
                self.statistics.execute(event, false, false).await?["configuration"]["result"]
                    .clone(),
            )
        })
    }
}
fn evaluate(operation: &str, command: Value) -> super::Result {
    let wire = govorun_core::evaluate_business_command(
        &json!({"operation":operation,"command":command}).to_string(),
    );
    let response: Value = serde_json::from_str(&wire).map_err(|_| "sync_invalid_response")?;
    response
        .get("result")
        .cloned()
        .ok_or("sync_invalid_request")
}
