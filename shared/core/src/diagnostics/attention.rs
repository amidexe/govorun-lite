//! Shared attention reasons and state; OS hosts render their own tray assets.
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason { Microphone, Accessibility, Hotkey, Model, Runtime, Update }

impl Reason {
    pub fn caption(self) -> &'static str {
        match self {
            Self::Microphone => "Разрешите доступ к микрофону",
            Self::Accessibility => "Разрешите доступ к универсальному доступу",
            Self::Hotkey => "Настройте клавиши диктовки",
            Self::Model => "Не удалось подготовить распознавание — открыть программу",
            Self::Runtime => "Диктовка недоступна — открыть программу",
            Self::Update => "Доступно обновление",
        }
    }

    pub fn route(self) -> &'static str {
        match self {
            Self::Update => "about",
            _ => "home",
        }
    }
}

#[derive(Default)]
pub struct Attention {
    unavailable: bool, microphone: bool, accessibility: bool, hotkey: bool,
    model: bool, update: bool,
}

impl Attention {
    pub fn runtime(&mut self, event: &str, data: &Value) {
        if event == "runtime.unavailable" { self.unavailable = true; return; }
        if event != "runtime.snapshot" { return; }
        if !matches!(data["phase"].as_str(), Some("idle" | "starting" | "recording" | "processing" | "fallback")) {
            return;
        }
        self.unavailable = false;
        let readiness = &data["readiness"];
        for (field, blocked) in [("microphone", &mut self.microphone),
            ("accessibility", &mut self.accessibility), ("hotkey", &mut self.hotkey)] {
            // Missing fields are not evidence that a previously reported error resolved.
            if let Some(available) = readiness[field].as_bool() { *blocked = !available; }
        }
        match readiness["model"].as_str() {
            Some("failed" | "stopped") => self.model = true,
            Some("ready" | "standby" | "loading" | "unprepared") => self.model = false,
            _ => {},
        }
    }

    pub fn updates(&mut self, data: &Value) {
        self.update = data["release"].is_object() && data["dismissed"] == false
            && !matches!(data["status"].as_str(), Some("downloading" | "installing"));
    }

    pub fn reason(&self) -> Option<Reason> {
        [(self.unavailable, Reason::Runtime), (self.microphone, Reason::Microphone),
         (self.accessibility, Reason::Accessibility), (self.hotkey, Reason::Hotkey),
         (self.model, Reason::Model), (self.update, Reason::Update)]
            .into_iter().find_map(|(active, reason)| active.then_some(reason))
    }
}
