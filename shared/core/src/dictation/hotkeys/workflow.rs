//! Pure capture admission, assignment conflicts, stale events and expiry.
use super::{Facts, Gesture, Target};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HotkeySnapshot {
    pub dictation: Vec<String>, pub plain: Vec<String>, pub cancel: Vec<String>,
    pub can_capture: bool,
    #[serde(rename = "captureID")]
    pub capture_id: Option<String>, pub target: Option<Target>, pub preview: Vec<String>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<String>"))]
    pub input_mode: Option<String>,
}
pub const CAPTURE_MILLIS: u64 = 30_000;
#[derive(Clone)]
pub struct Capture { pub id: String, pub target: Target, pub deadline: u64, sequence: u64 }
#[derive(Clone, Default)]
pub struct Workflow {
    capture: Option<Capture>,
    preview: Vec<String>,
    message: Option<&'static str>,
}
#[derive(Clone)]
pub struct Assignment { pub target: Target, pub gesture: Option<Gesture>, pub reset: bool }
impl Workflow {
    pub fn capture(&self) -> Option<&Capture> { self.capture.as_ref() }
    pub fn snapshot(&self, facts: &Facts) -> Value {
        serde_json::to_value(HotkeySnapshot {
            dictation: facts.dictation.tokens.clone(),
            plain: facts.effective_plain().map(|g| g.tokens.clone()).unwrap_or_default(),
            cancel: facts.cancel.tokens.clone(), can_capture: !facts.busy && facts.permission_ready,
            capture_id: self.capture.as_ref().map(|v| v.id.clone()),
            target: self.capture.as_ref().map(|v| v.target), preview: self.preview.clone(),
            input_mode: facts.input_mode.clone(), message: self.message.map(|code| match code {
            "conflict" => "Это сочетание уже используется для другого действия.",
            "paste" if facts.input_mode.is_none() => "⌘V занят вставкой текста",
            "paste" => "Это сочетание используется для вставки текста.",
            "save" => "Не удалось сохранить сочетание. Попробуйте ещё раз.",
            "permission" => "Разрешите доступ в специальных возможностях и повторите.",
            _ => "Выберите другое сочетание.",
        }).map(str::to_owned),
        }).expect("hotkey snapshot is serializable")
    }

    pub fn start(&mut self, facts: &Facts, target: Target, owner: &str, focused: bool, id: String, now: u64)
        -> Result<(), &'static str> {
        if facts.busy || !facts.permission_ready || !focused || owner != "main" || id.is_empty() || id.len() > 64 {
            return Err("hotkeys_capture_unavailable");
        }
        self.stop();
        self.capture = Some(Capture { id, target, deadline: now.saturating_add(CAPTURE_MILLIS), sequence: 0 });
        Ok(())
    }
    pub fn cancel(&mut self, id: &str) -> bool {
        if self.capture.as_ref().is_some_and(|v| v.id == id) { self.stop(); true } else { false }
    }
    pub fn cancel_owner(&mut self, owner: &str) -> bool {
        if owner == "main" && self.capture.is_some() { self.stop(); true } else { false }
    }
    pub fn expire(&mut self, id: &str, now: u64) -> bool {
        if self.capture.as_ref().is_some_and(|v| v.id == id && now >= v.deadline) { self.stop(); true } else { false }
    }
    pub fn reset(&mut self, facts: &Facts, target: Target) -> Result<Option<Assignment>, &'static str> {
        if facts.busy { return Err("hotkeys_capture_unavailable"); }
        let gesture = match target {
            Target::Dictation => Some(facts.default_dictation.clone()),
            Target::Cancel => Some(facts.default_cancel.clone()),
            Target::Plain => None,
        };
        if gesture.as_ref().is_some_and(|g| facts.conflicts(target, g)) {
            self.message = Some("conflict"); return Ok(None);
        }
        self.stop(); Ok(Some(Assignment { target, gesture, reset: true }))
    }
    pub fn observed(&mut self, id: &str, sequence: u64, preview: Vec<String>) -> bool {
        let Some(capture) = self.capture.as_mut().filter(|v| v.id == id && sequence > v.sequence) else { return false; };
        if preview.len() > 8 || preview.iter().any(|p| p.is_empty() || p.len() > 64 || p.chars().any(char::is_control)) { return false; }
        capture.sequence = sequence;
        self.preview = preview; true
    }
    pub fn candidate(&mut self, facts: &Facts, id: &str, sequence: u64, gesture: Gesture) -> Option<Assignment> {
        if !gesture.checked() || !self.observed(id, sequence, gesture.tokens.clone()) { return None; }
        if facts.busy || !facts.permission_ready { self.stop(); return None; }
        if !gesture.valid || gesture.reserved_for_paste {
            self.message = Some(if gesture.reserved_for_paste { "paste" } else { "invalid" }); return None;
        }
        let target = self.capture.as_ref()?.target;
        if facts.conflicts(target, &gesture) { self.message = Some("conflict"); return None; }
        Some(Assignment { target, gesture: Some(gesture), reset: false })
    }
    pub fn saved(&mut self, success: bool) { self.stop(); if !success { self.message = Some("save"); } }
    pub fn capture_failed(&mut self) { self.stop(); self.message = Some("permission"); }
    pub fn stop(&mut self) { self.capture = None; self.preview.clear(); self.message = None; }
}
