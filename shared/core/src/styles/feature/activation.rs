//! Async effects follow the existing Core activation transition and lifetime.
use super::{actions::Action, StylesFeature};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin, sync::Arc};
#[derive(Clone, Debug)]
pub enum Effect {
    AcceptTerms { generation: u64, version: u32 },
    Refresh { generation: u64 },
    PreparePasses { generation: u64 },
    ConsentTimestamp { generation: u64 },
    SelfHosted { enabled: bool },
    Documents,
    LoadApplications,
}
pub type Io = Arc<
    dyn Fn(Effect) -> Pin<Box<dyn Future<Output = Result<Value, &'static str>> + Send>>
        + Send
        + Sync,
>;
#[derive(Clone)]
pub struct Effects {
    pub io: Io,
    pub mutation_gate: Arc<tokio::sync::Mutex<()>>,
}
#[derive(Default)]
pub(super) struct State {
    generation: u64,
    serial: u64,
    operation: Option<u64>,
    message: Option<String>,
    pub(super) stopped: bool,
}
impl StylesFeature {
    pub fn attach_effects(&self, effects: Effects) -> Result<(), &'static str> {
        self.effects
            .set(effects)
            .map_err(|_| "styles_already_initialized")
    }
    pub(super) fn presentation(
        &self,
        generation: u64,
    ) -> Result<(bool, Option<String>), &'static str> {
        let mut state = self.state.lock().unwrap();
        if state.stopped {
            return Err("styles_stopped");
        }
        if state.generation != generation {
            state.generation = generation;
            state.operation = None;
            state.message = None;
        }
        Ok((state.operation.is_some(), state.message.clone()))
    }
    pub(super) fn publish_view(&self, view: Value) {
        if self.gate.changed(&view) {
            (self.events)("styles.changed", view);
        }
    }
    pub async fn stop(&self) {
        {
            let mut state = self.state.lock().unwrap();
            state.stopped = true;
            state.operation = None;
        }
        // Admission finishes before collecting jobs; admitted HTTP is drained,
        // not dropped halfway through the durable account consent publication.
        {
            let _serial = self.serial.lock().await;
        }
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        for task in tasks {
            let _ = task.await;
        }
    }
    pub(super) async fn activation_action(
        self: &Arc<Self>,
        action: &Action,
        before: &Value,
    ) -> Result<Option<Value>, &'static str> {
        let effects = self.effects.get();
        let generation = (self.facts)().await?.access["generation"]
            .as_u64()
            .ok_or("styles_invalid_response")?;
        if (self.scope)(generation) != before["scope"] {
            return Err("styles_session_changed");
        }
        match action {
            Action::Cancel {} | Action::Disable {} => {
                {
                    let mut state = self.state.lock().unwrap();
                    state.operation = None;
                    state.message = None;
                }
                if matches!(action, Action::Disable {}) {
                    if matches!(
                        before["gate"].as_str(),
                        Some("selfHosted" | "selfHostedSetup")
                    ) {
                        (effects.ok_or("styles_effects_unavailable")?.io)(Effect::SelfHosted {
                            enabled: false,
                        })
                        .await?;
                    } else {
                        self.library
                            .execute(json!({"kind":"setEnabled","enabled":false}), None, true)
                            .await?;
                    }
                }
            }
            Action::Documents {} => {
                (effects.ok_or("styles_effects_unavailable")?.io)(Effect::Documents).await?;
            }
            Action::Select {
                selection,
                activation_options,
            }
            | Action::AcceptTerms {
                selection,
                activation_options,
                ..
            } => {
                if before["busy"] == true {
                    return Err("styles_busy");
                }
                let effects = effects.ok_or("styles_effects_unavailable")?.clone();
                let library = self
                    .library
                    .execute(json!({"kind":"snapshot"}), None, false)
                    .await?;
                let mut facts = (self.facts)().await?.access;
                if facts["generation"] != generation || (self.scope)(generation) != before["scope"]
                {
                    return Err("styles_session_changed");
                }
                facts["localTermsVersion"] =
                    library["configuration"]["metadata"]["localTermsVersion"].clone();
                if facts["selfHosted"] != true {
                    facts["enabled"] = library["configuration"]["metadata"]["enabled"].clone();
                }
                facts["presetCount"] = json!(library["configuration"]["context"]["state"]
                    ["presets"]
                    .as_array()
                    .ok_or("styles_invalid_response")?
                    .len());
                let (accept, version, confirmed) = match action {
                    Action::AcceptTerms {
                        terms_version,
                        confirmed,
                        ..
                    } => (true, *terms_version, *confirmed),
                    _ => (
                        false,
                        facts["clientTermsVersion"]
                            .as_u64()
                            .ok_or("styles_invalid_response")? as u32,
                        true,
                    ),
                };
                let supplied = activation_options
                    .as_ref()
                    .map(|v| serde_json::to_value(v).expect("style options serialize"));
                let options = if accept {
                    core(json!({"operation":"styleLibrary","command":{"kind":"activationOptions",
                        "context":library["configuration"]["context"],"facts":{"localTermsVersion":facts["localTermsVersion"],
                        "optionsPresent":library["storedOptionsPresent"]},"supplied":supplied}}))?["activationOptions"].clone()
                } else {
                    supplied.unwrap_or(Value::Null)
                };
                let id = {
                    let mut state = self.state.lock().unwrap();
                    if state.stopped {
                        return Err("styles_stopped");
                    }
                    state.serial += 1;
                    state.message = None;
                    state.serial
                };
                let exists = selection == "selfHosted"
                    || !library["projection"]["optionsBySelection"][selection].is_null();
                let frame = core(
                    json!({"operation":"styleActivation","facts":facts,"event":{"kind":"begin",
                    "operationId":id.to_string(),"selection":selection,"selectionExists":exists,"termsVersion":version,
                    "confirmed":confirmed,"acceptTerms":accept}}),
                )?;
                if frame["effect"]["kind"] == "acceptTerms" {
                    self.state.lock().unwrap().operation = Some(id);
                    self.start(
                        effects,
                        generation,
                        id,
                        Some(frame["intent"].clone()),
                        options,
                        Effect::AcceptTerms {
                            generation,
                            version,
                        },
                    );
                } else {
                    self.apply_frame(&frame, options, generation, &effects)
                        .await?;
                }
            }
            Action::Refresh {} | Action::LoadApplications {} => {
                if before["busy"] == true {
                    return Err("styles_busy");
                }
                if matches!(action, Action::Refresh {})
                    && (self.facts)().await?.access["signedIn"] != true
                {
                    return Err("styles_invalid_request");
                }
                if matches!(action, Action::LoadApplications {})
                    && matches!(
                        before["gate"].as_str(),
                        Some("selfHosted" | "selfHostedSetup")
                    )
                {
                    return Err("styles_invalid_request");
                }
                let effects = effects.ok_or("styles_effects_unavailable")?.clone();
                let id = {
                    let mut state = self.state.lock().unwrap();
                    state.serial += 1;
                    state.operation = Some(state.serial);
                    state.message = None;
                    state.serial
                };
                let effect = if matches!(action, Action::Refresh {}) {
                    Effect::Refresh { generation }
                } else {
                    Effect::LoadApplications
                };
                self.start(effects, generation, id, None, Value::Null, effect);
            }
            _ => return Ok(None),
        }
        Ok(Some(self.read_snapshot().await?))
    }
    fn start(
        self: &Arc<Self>,
        effects: Effects,
        generation: u64,
        id: u64,
        intent: Option<Value>,
        options: Value,
        effect: Effect,
    ) {
        let host = self.clone();
        let task = tokio::spawn(async move {
            let result = (effects.io)(effect).await;
            let _gate = effects.mutation_gate.lock().await;
            let _serial = host.serial.lock().await;
            let result = host
                .finish_remote(&effects, generation, id, intent, options, result)
                .await;
            {
                let mut state = host.state.lock().unwrap();
                if state.operation == Some(id) {
                    if result.is_err() {
                        state.message =
                            Some("Не удалось выполнить действие. Попробуйте ещё раз.".into());
                    }
                    state.operation = None;
                }
            }
            if let Ok(view) = host.read_snapshot().await {
                host.publish_view(view);
            }
        });
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|v| !v.is_finished());
        tasks.push(task);
    }
    async fn finish_remote(
        &self,
        effects: &Effects,
        generation: u64,
        id: u64,
        intent: Option<Value>,
        options: Value,
        result: Result<Value, &'static str>,
    ) -> Result<(), &'static str> {
        let mut facts = (self.facts)().await?.access;
        let library = self
            .library
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        facts["localTermsVersion"] =
            library["configuration"]["metadata"]["localTermsVersion"].clone();
        let lifetime = {
            let state = self.state.lock().unwrap();
            json!({"activeOperationId":state.operation.map(|v|v.to_string()),
            "cancelled":state.operation!=Some(id),"disposed":state.stopped})
        };
        let event = if result.is_ok() {
            if let Some(intent) = intent {
                let selection = intent["selection"]
                    .as_str()
                    .ok_or("styles_invalid_response")?;
                let exists = !library["projection"]["optionsBySelection"][selection].is_null();
                json!({"kind":"complete","intent":intent,"lifetime":lifetime,"selectionExists":exists})
            } else {
                return Ok(());
            }
        } else {
            json!({"kind":"failed","generation":generation,"operationId":id.to_string(),"lifetime":lifetime})
        };
        let frame = core(json!({"operation":"styleActivation","facts":facts,"event":event}))?;
        self.apply_frame(&frame, options, generation, effects).await
    }
    async fn apply_frame(
        &self,
        frame: &Value,
        options: Value,
        generation: u64,
        effects: &Effects,
    ) -> Result<(), &'static str> {
        if frame["effect"]["kind"] == "none" {
            return Ok(());
        }
        let facts = (self.facts)().await?.access;
        if facts["generation"] != generation || self.state.lock().unwrap().stopped {
            return Err("styles_session_changed");
        }
        let effect = &frame["effect"];
        match effect["kind"].as_str() {
            Some("applySelection") => {
                self.library.execute(json!({"kind":"selectStyle","selection":effect["selection"],"activationOptions":options}),None,true).await?;
                self.library
                    .execute(json!({"kind":"setEnabled","enabled":true}), None, true)
                    .await?;
                self.record_consent(effect["acceptedVersion"].clone(), generation, effects)
                    .await?;
                (effects.io)(Effect::PreparePasses { generation }).await?;
            }
            Some("recordConsent") => {
                self.record_consent(effect["acceptedVersion"].clone(), generation, effects)
                    .await?
            }
            Some("applySelfHosted") => {
                (effects.io)(Effect::SelfHosted { enabled: true }).await?;
            }
            Some("failure") => (),
            _ => return Err("styles_invalid_response"),
        }
        self.state.lock().unwrap().message = effect["message"].as_str().map(str::to_owned);
        Ok(())
    }
    async fn record_consent(
        &self,
        version: Value,
        generation: u64,
        effects: &Effects,
    ) -> Result<(), &'static str> {
        let timestamp = (effects.io)(Effect::ConsentTimestamp { generation }).await?;
        self.library
            .execute(
                json!({"kind":"recordConsent","version":version,"acceptedAtMillis":timestamp}),
                None,
                true,
            )
            .await?;
        Ok(())
    }
}
pub(super) fn core(command: Value) -> Result<Value, &'static str> {
    let reply = govorun_core::evaluate_business_command(&command.to_string());
    let value: Value = serde_json::from_str(&reply).map_err(|_| "styles_invalid_response")?;
    value.get("result").cloned().ok_or("styles_invalid_request")
}
