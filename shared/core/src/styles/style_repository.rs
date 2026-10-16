//! Library mutations prepare current native formats before one document CAS.
#[path="style_repository/codec.rs"]
mod codec;
pub use crate::style_library::dictation::Resolved as DictationStyle;
pub use crate::style_library::dictation::ApplicationIdentity;
pub fn resolve_dictation(context: Value, rules: Value, application: Option<&str>, identity: ApplicationIdentity) -> crate::Result<DictationStyle> {
    crate::style_library::dictation::resolve(context, rules, application, identity)
}
use crate::{DocumentStore, StoreError};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleDocumentError {
    InvalidAction,
    InvalidStorage,
    StorageUnavailable,
    Conflict,
}
impl From<StoreError> for StyleDocumentError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Invalid => Self::InvalidStorage,
            StoreError::Unavailable => Self::StorageUnavailable,
            StoreError::Conflict => Self::Conflict,
        }
    }
}
pub struct StyleDocumentResult {
    pub metadata: Value,
    pub context: Value,
    pub projection: Value,
    pub application_rules: Value,
    pub stored_options_present: bool,
    pub revision: String,
    pub changed: bool,
    pub mark_dirty: bool,
    pub preferences_dirty: bool,
    pub accepted: bool,
    pub reason: String,
}
pub struct StyleRepository {
    store: Arc<dyn DocumentStore>,
    serial: Mutex<()>,
}
impl StyleRepository {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self {
        Self {
            store,
            serial: Mutex::new(()),
        }
    }

    /// The repository supplies state/context/rules. Callers cannot replace them
    /// with stale copies or inject authority into a local catalog edit.
    pub async fn execute(
        &self,
        action: Value,
        expected: Option<String>,
    ) -> Result<StyleDocumentResult, StyleDocumentError> {
        let fields = action
            .as_object()
            .ok_or(StyleDocumentError::InvalidAction)?;
        let kind = action["kind"]
            .as_str()
            .ok_or(StyleDocumentError::InvalidAction)?
            .to_owned();
        if fields.contains_key("state")
            || fields.contains_key("context")
            || fields.contains_key("rules")
        {
            return Err(StyleDocumentError::InvalidAction);
        }
        let snapshot = kind == "snapshot" && fields.len() == 1;
        let metadata_action = matches!(
            kind.as_str(),
            "setEnabled" | "recordConsent" | "dismissStarterGuidance"
        );
        let context_action = matches!(
            kind.as_str(),
            "selectStyle" | "setSelectedOption" | "saveApplicationRule" | "deleteApplicationRule"
        );
        let rules_action = matches!(
            kind.as_str(),
            "saveApplicationRule" | "deleteApplicationRule"
        );
        if !snapshot
            && !context_action
            && !metadata_action
            && !matches!(
                kind.as_str(),
                "save" | "saveActive" | "delete" | "deleteWithReferences" | "options" | "replace"
            )
        {
            return Err(StyleDocumentError::InvalidAction);
        }
        let _serial = self.serial.lock().await;
        let key = "styles.document".to_owned();
        let document = self
            .store
            .read_document(key.clone())
            .await?
            .ok_or(StyleDocumentError::InvalidStorage)?;
        if document.revision.is_empty() || document.revision.len() > 128 {
            return Err(StyleDocumentError::InvalidStorage);
        }
        if expected
            .as_ref()
            .is_some_and(|revision| revision != &document.revision)
        {
            return Err(StyleDocumentError::Conflict);
        }
        let mut physical = codec::Physical::read(document.value)?;
        let _metadata = physical.metadata()?;
        let original = physical.logical()?;
        let mut context = original.context.clone();
        let mut rules = original.rules.clone();
        let mut changed = false;
        let mut dirty = false;
        let mut accepted = true;
        let mut reason = "ok".to_owned();
        if metadata_action {
            changed = physical.metadata_action(action)?;
        } else if !snapshot {
            let mut command = action;
            if context_action {
                command["context"] = context.clone();
                if rules_action {
                    command["rules"] = rules.clone();
                }
            } else {
                command["state"] = context["state"].clone();
                if kind == "deleteWithReferences" {
                    command["rules"] = rules.clone();
                }
            }
            let decision = call(command)?;
            accepted = decision["accepted"]
                .as_bool()
                .ok_or(StyleDocumentError::InvalidStorage)?;
            reason = decision["reason"]
                .as_str()
                .ok_or(StyleDocumentError::InvalidStorage)?
                .into();
            changed = decision["write"]
                .as_bool()
                .ok_or(StyleDocumentError::InvalidStorage)?;
            dirty = decision["markDirty"]
                .as_bool()
                .ok_or(StyleDocumentError::InvalidStorage)?;
            if changed {
                if context_action {
                    if rules_action {
                        rules = decision["applicationRules"].clone();
                    }
                    for name in [
                        "state",
                        "builtInOptions",
                        "currentOptions",
                        "currentOptionsStyle",
                    ] {
                        context[name] = decision[name].clone();
                    }
                } else {
                    context["state"] = decision["state"].clone();
                    let projection = call(json!({"kind":"choices","context":context}))?;
                    if !projection["projection"]["selectedOptions"].is_null() {
                        context["currentOptions"] =
                            projection["projection"]["selectedOptions"].clone();
                    }
                    context["currentOptionsStyle"] = context["state"]["selectedStyle"].clone();
                    if let Some(value) = decision.get("applicationRules") {
                        rules = value.clone();
                    }
                }
            }
        }
        let projection = call(json!({"kind":"choices","context":context}))?["projection"].clone();
        let mut options_present = original.options_present;
        let preferences_dirty = original.context["state"]["selectedStyle"]
            != context["state"]["selectedStyle"]
            || original.context["currentOptions"] != context["currentOptions"];
        let revision = if changed {
            physical.set(&original, &context, &rules)?;
            options_present = physical.logical()?.options_present;
            let value = physical.value()?;
            let written = self
                .store
                .write_document(key, Some(document.revision), value.clone())
                .await?;
            if written.value != value || written.revision.is_empty() || written.revision.len() > 128
            {
                return Err(StyleDocumentError::InvalidStorage);
            }
            written.revision
        } else {
            document.revision
        };
        Ok(StyleDocumentResult {
            metadata: physical.metadata()?,
            context,
            projection,
            application_rules: rules,
            stored_options_present: options_present,
            revision,
            changed,
            mark_dirty: dirty,
            preferences_dirty,
            accepted,
            reason,
        })
    }
}
fn call(command: Value) -> Result<Value, StyleDocumentError> {
    let command = serde_json::from_value(command).map_err(|_| StyleDocumentError::InvalidAction)?;
    crate::style_library::evaluate(command).map_err(|_| StyleDocumentError::InvalidAction)
}
