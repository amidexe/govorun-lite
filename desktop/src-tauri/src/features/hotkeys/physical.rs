//! Direct Rust ports for existing Core assignment workflow. The document store
//! remains its owner; this adapter only reads facts and controls the OS hook.
use crate::{
    agent::{AgentFuture, AgentTransport},
    features::hotkeys::Hotkeys,
    hotkey_gestures,
    recording_runtime::Runtime,
};
use govorun_os_runtime::keyboard::{Binding, Platform, Target};
use serde_json::{json, Value};
use std::sync::{Arc, Weak};
pub struct Physical {
    runtime: Weak<Runtime>,
    owner: Weak<Hotkeys>,
    focus: Arc<dyn Fn() -> bool + Send + Sync>,
}
fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Windows
    }
}
impl Physical {
    pub fn new(runtime: &Arc<Runtime>, owner: &Arc<Hotkeys>, focus: Arc<dyn Fn() -> bool + Send + Sync>) -> Arc<Self> {
        Arc::new(Self {
            runtime: Arc::downgrade(runtime),
            owner: Arc::downgrade(owner),
            focus,
        })
    }
    fn execute(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let runtime = self.runtime.upgrade().ok_or("dictation_stopped")?;
        match method {
            "hotkeys.localFacts" if params.as_object().is_some_and(|value| value.len() == 1) => {
                let expected = if platform() == Platform::MacOS {
                    "macos"
                } else {
                    "windows"
                };
                if params["storage"]["source"] != expected {
                    return Err("hotkeys_storage_invalid");
                }
                let facts = hotkey_gestures::facts(
                    &params["storage"],
                    runtime.busy(),
                    runtime.hook_ready(),
                )?;
                serde_json::to_value(facts).map_err(|_| "hotkeys_invalid_facts")
            }
            "hotkeys.localEffect" => {
                let accepted = match params["action"].as_str() {
                    Some("start") if params.as_object().is_some_and(|value| value.len() == 2) => {
                        let id = params["captureID"]
                            .as_str()
                            .ok_or("hotkeys_invalid_capture")?
                            .to_owned();
                        let owner = self.owner.clone();
                        runtime.capture_keys(id,self.focus.clone(),Arc::new(move|event|{
                            let owner=owner.clone();
                            let preview=event.preview.map(|value|hotkey_gestures::preview(platform(),value)).unwrap_or_default();
                            let gesture=event.gesture.map(|value|hotkey_gestures::describe(platform(),value));
                            let value=json!({"captureID":event.id,"sequence":event.sequence,"preview":preview,"gesture":gesture,"cancelled":event.cancelled});
                            tauri::async_runtime::spawn(async move {if let Some(owner)=owner.upgrade(){let _=owner.captured(value).await;}});
                        })).map(|_|true)?
                    }
                    Some("stop") if params.as_object().is_some_and(|value| value.len() == 2) => {
                        runtime.stop_key_capture(
                            params["captureID"]
                                .as_str()
                                .ok_or("hotkeys_invalid_capture")?,
                        )?;
                        true
                    }
                    Some("configure")
                        if params.as_object().is_some_and(|value| value.len() == 4) =>
                    {
                        let mut bindings = Vec::new();
                        for (field, target) in [
                            ("dictation", Target::Dictation),
                            ("plain", Target::Plain),
                            ("cancel", Target::Cancel),
                        ] {
                            if field == "plain" && params[field].is_null() {
                                continue;
                            }
                            let gesture =
                                hotkey_gestures::bindings_raw(platform(), &params[field])?;
                            if !gesture.valid || gesture.reserved_for_paste {
                                return Err("hotkeys_invalid_binding");
                            }
                            bindings.push((
                                target,
                                Binding::decode(
                                    platform(),
                                    gesture.key,
                                    gesture.modifiers,
                                    gesture.modifier_only,
                                    gesture.control_side,
                                )?,
                            ));
                        }
                        runtime.configure_bindings(bindings)?;
                        true
                    }
                    _ => return Err("hotkeys_invalid_request"),
                };
                Ok(json!({"accepted":accepted}))
            }
            _ => Err("hotkeys_invalid_request"),
        }
    }
}
impl AgentTransport for Physical {
    fn invoke<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        Box::pin(async move { self.execute(method, params) })
    }
}
