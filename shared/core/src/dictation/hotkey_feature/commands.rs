use super::Hotkeys;
use govorun_core::hotkeys::{Gesture, Target};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, atomic::Ordering};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
enum Action {
    Start { target: Target, owner: String, focused: bool },
    Reset { target: Target },
    #[serde(rename_all = "camelCase")]
    Cancel { #[serde(rename = "captureID")] capture_id: String },
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureEvent {
    #[serde(rename = "captureID")]
    capture_id: String,
    sequence: u64,
    preview: Vec<String>,
    gesture: Option<Gesture>,
    cancelled: bool,
}
impl Hotkeys {
    pub async fn invoke(self: &Arc<Self>, method: &str, params: Value) -> Result<Value, &'static str> {
        let action = match method {
            "hotkeys.snapshot" if params.as_object().is_some_and(|p| p.is_empty()) => None,
            "hotkeys.action" => Some(serde_json::from_value::<Action>(params.clone()).map_err(|_| "hotkeys_invalid_request")?),
            "hotkeys.cancel" if params.as_object().is_some_and(|p| p.len() == 1) && params["owner"].is_string() => None,
            _ => return Err("hotkeys_invalid_request"),
        };
        let mut state = self.serial.lock().await;
        self.refresh(&mut state).await?;
        let state = state.as_mut().ok_or("hotkeys_unavailable")?;
        if method == "hotkeys.cancel" {
            let id = state.flow.capture().map(|c|c.id.clone());
            if state.flow.cancel_owner(params["owner"].as_str().ok_or("hotkeys_invalid_request")?) {
                if let Some(id) = id { self.effect(json!({"action":"stop","captureID":id})).await?; }
            }
        }
        match action {
            Some(Action::Start { target, owner, focused }) => {
                let old = state.flow.capture().map(|c|c.id.clone());
                let id = format!("{}-{}-{}-{}", crate::instance::nonce(), self.instance, self.now(), self.next_id.fetch_add(1, Ordering::Relaxed));
                state.flow.start(&state.facts, target, &owner, focused, id.clone(), self.now())?;
                if let Some(old) = old { self.effect(json!({"action":"stop","captureID":old})).await?; }
                match self.effect(json!({"action":"start","captureID":id})).await {
                    Ok(()) => self.schedule_expiry(id, state.flow.capture().ok_or("hotkeys_unavailable")?.deadline),
                    Err(code) => {
                        // A focus/busy/backend failure is not an OS permission denial.
                        let cleanup = self.effect(json!({"action":"stop","captureID":id})).await;
                        if code == "hotkeys_permission_required" { state.flow.capture_failed(); }
                        else { state.flow.stop(); }
                        if let Err(cleanup_code) = cleanup { self.publish(state); return Err(cleanup_code); }
                        if code != "hotkeys_permission_required" { self.publish(state); return Err(code); }
                    }
                }
            }
            Some(Action::Cancel { capture_id }) => {
                if state.flow.cancel(&capture_id) { self.effect(json!({"action":"stop","captureID":capture_id})).await?; }
            }
            Some(Action::Reset { target }) => {
                let old = state.flow.capture().map(|c|c.id.clone());
                if let Some(change) = state.flow.reset(&state.facts, target)? {
                    if let Some(id) = old { self.effect(json!({"action":"stop","captureID":id})).await?; }
                    let _ = self.save(state, change).await;
                }
            }
            None => { self.configure(state).await?; }
        }
        Ok(self.publish(state))
    }
    pub async fn captured(self: &Arc<Self>, value: Value) -> Result<(), &'static str> {
        let event: CaptureEvent = serde_json::from_value(value).map_err(|_| "hotkeys_invalid_capture")?;
        let mut state = self.serial.lock().await;
        let Some(current) = state.as_ref() else { return Ok(()); };
        if !current.flow.capture().is_some_and(|c| c.id == event.capture_id) { return Ok(()); }
        self.refresh(&mut state).await?;
        let state = state.as_mut().ok_or("hotkeys_unavailable")?;
        let active = event.capture_id.clone();
        let mut stopped_by_save = false;
        if event.cancelled {
            if !state.flow.observed(&event.capture_id, event.sequence, vec![]) { return Ok(()); }
            state.flow.cancel(&event.capture_id);
        }
        else if let Some(gesture) = event.gesture {
            if let Some(change) = state.flow.candidate(&state.facts, &event.capture_id, event.sequence, gesture) {
                stopped_by_save = true;
                let _ = self.save(state, change).await;
            }
        } else { state.flow.observed(&event.capture_id, event.sequence, event.preview); }
        if state.flow.capture().is_none() && !stopped_by_save { let _ = self.effect(json!({"action":"stop","captureID":active})).await; }
        self.publish(state); Ok(())
    }
}
