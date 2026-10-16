//! Scoped self-hosted UI workflow. Effects are executed by the desktop host.
use super::{
    configuration::{Configuration, Provider},
    settings::{KeyState, Settings},
    stored::Stored,
};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Action {
    Provider {
        scope: String,
        provider: Provider,
    },
    Connect {
        scope: String,
        configuration: Configuration,
        confirmed: bool,
        key: String,
    },
    Disconnect {
        scope: String,
    },
    RevokeConsent {
        scope: String,
    },
    RemoveKey {
        scope: String,
        confirmed: bool,
    },
    Clear {
        scope: String,
        confirmed: bool,
    },
    Cancel {
        #[serde(rename = "operationID")]
        operation_id: String,
    },
}
pub enum Work {
    None,
    Connect(String),
    RemoveKey,
    Clear,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Cancelled,
    KeyFailed,
    CheckFailed,
    ClearFailed,
    StorageFailed,
}
#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SelfHostSnapshot {
    pub scope: String, pub configuration: Configuration,
    pub route_enabled: bool, pub styling_enabled: bool, pub configured: bool,
    pub consent: bool, pub verified: bool, pub key_state: KeyState,
    pub busy: bool, pub cancelling: bool,
    #[serde(rename = "operationID")]
    pub operation_id: Option<String>, pub operation: Option<String>,
    pub message: Option<String>, pub failed: bool,
}
#[derive(Clone)]
struct Operation {
    id: String,
    kind: &'static str,
    expected: Configuration,
    cancelling: bool,
}
#[derive(Clone)]
pub struct Controller {
    instance: String,
    revision: u64,
    sequence: u64,
    observed: Option<(Settings, KeyState)>,
    operation: Option<Operation>,
    message: Option<&'static str>,
    failed: bool,
}
impl Controller {
    pub fn new(instance: String) -> Self {
        Self {
            instance,
            revision: 0,
            sequence: 0,
            observed: None,
            operation: None,
            message: None,
            failed: false,
        }
    }
    pub fn busy(&self) -> bool {
        self.operation.is_some()
    }
    pub fn operation_id(&self) -> Option<&str> {
        self.operation.as_ref().map(|value| value.id.as_str())
    }
    pub fn cancelled(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|value| value.cancelling)
    }
    pub fn expected(&self) -> Option<Configuration> {
        self.operation.as_ref().map(|value| value.expected.clone())
    }
    pub fn storage_action_failed(&mut self) {
        // An in-flight protected-store effect retains its lease even when the
        // cancellation/settings CAS fails. A later action cannot steal it.
        if let Some(operation) = self.operation.as_mut() {
            operation.cancelling = true;
        }
        self.failed = true;
        self.message = Some("Не удалось сохранить настройки. Обработка отключена.");
    }
    pub fn storage_failed(&mut self) {
        self.operation = None;
        self.failed = true;
        self.message = Some("Не удалось сохранить настройки. Обработка отключена.");
    }
    fn scope(&mut self, settings: &Settings, key: KeyState) -> String {
        if self.observed.as_ref() != Some(&(settings.clone(), key)) {
            self.revision += 1;
            self.observed = Some((settings.clone(), key));
        }
        format!("{}:{}", self.instance, self.revision)
    }
    pub fn snapshot(&mut self, stored: &Stored, key: KeyState) -> Value {
        let s = &stored.settings;
        serde_json::to_value(SelfHostSnapshot {
            scope: self.scope(s, key), configuration: s.configuration.clone(),
            route_enabled: s.route_enabled, styling_enabled: s.styling_enabled,
            configured: s.configured(key), consent: s.consent(stored.format), verified: s.verified(),
            key_state: key, busy: self.busy(), cancelling: self.cancelled(),
            operation_id: self.operation_id().map(str::to_owned),
            operation: self.operation.as_ref().map(|v| v.kind.to_owned()),
            message: self.message.map(str::to_owned), failed: self.failed,
        }).expect("self-host snapshot is serializable")
    }
    pub fn begin(&mut self, action: Action, stored: &mut Stored, key: KeyState) -> Result<Work> {
        if let Action::Cancel { operation_id } = action {
            if let Some(operation) = self
                .operation
                .as_mut()
                .filter(|value| value.id == operation_id && value.kind == "connect")
            {
                operation.cancelling = true;
                stored.settings.disable_style();
            }
            return Ok(Work::None);
        }
        let scope = match &action {
            Action::Provider { scope, .. }
            | Action::Connect { scope, .. }
            | Action::Disconnect { scope }
            | Action::RevokeConsent { scope }
            | Action::RemoveKey { scope, .. }
            | Action::Clear { scope, .. } => scope,
            Action::Cancel { .. } => unreachable!(),
        };
        if self.busy() || *scope != self.scope(&stored.settings, key) {
            return Err(Error::InvalidRequest);
        }
        let disconnected = matches!(action, Action::Disconnect { .. });
        let work = match action {
            Action::Provider { provider, .. } => {
                stored
                    .select_provider(provider)
                    .map_err(|_| Error::InvalidRequest)?;
                Work::None
            }
            Action::Connect {
                configuration,
                confirmed,
                key,
                ..
            } => {
                if !confirmed || key.len() > 16384 || key.contains(['\r', '\n']) {
                    return Err(Error::InvalidRequest);
                }
                stored.settings.connect(configuration, stored.format)?;
                Work::Connect(key)
            }
            Action::Disconnect { .. } => {
                stored.settings.disconnect();
                Work::None
            }
            Action::RevokeConsent { .. } => {
                stored.settings.revoke();
                Work::None
            }
            Action::RemoveKey { confirmed, .. } | Action::Clear { confirmed, .. } if !confirmed => {
                return Err(Error::InvalidRequest)
            }
            Action::RemoveKey { .. } => {
                stored.settings.disable_style();
                Work::RemoveKey
            }
            Action::Clear { .. } => {
                stored.settings.disable_style();
                Work::Clear
            }
            Action::Cancel { .. } => unreachable!(),
        };
        self.message = if disconnected {
            Some("Свой сервер отключён. Настройки сохранены.")
        } else {
            None
        };
        self.failed = false;
        let kind = match work {
            Work::Connect(_) => Some("connect"),
            Work::RemoveKey => Some("removeKey"),
            Work::Clear => Some("clear"),
            Work::None => None,
        };
        if let Some(kind) = kind {
            self.sequence += 1;
            self.operation = Some(Operation {
                id: format!("{}-{}", self.instance, self.sequence),
                kind,
                expected: stored.settings.configuration.clone(),
                cancelling: false,
            });
        }
        Ok(work)
    }
    pub fn finish(&mut self, stored: &mut Stored, key: KeyState, mut outcome: Outcome) {
        let Some(operation) = self.operation.take() else {
            return;
        };
        if operation.cancelling {
            outcome = Outcome::Cancelled;
        }
        if outcome == Outcome::Success
            && operation.kind == "connect"
            && stored
                .settings
                .enable(&operation.expected, stored.format, key)
                .is_err()
        {
            outcome = Outcome::Cancelled;
        }
        if outcome != Outcome::Success {
            stored.settings.disable_style();
        }
        if outcome == Outcome::Success && operation.kind == "clear" {
            stored.clear();
        }
        self.failed = !matches!(outcome, Outcome::Success | Outcome::Cancelled);
        self.message = Some(match outcome {
            Outcome::Success => match operation.kind {
                "connect" => "Соединение проверено · свой сервер подключён",
                "removeKey" => "Ключ удалён · обработка отключена",
                _ => "Настройки сервера сброшены · обработка отключена",
            },
            Outcome::Cancelled => "Проверка отменена. Обработка отключена.",
            Outcome::KeyFailed => {
                if operation.kind == "removeKey" {
                    "Не удалось удалить ключ. Обработка отключена."
                } else {
                    "Не удалось сохранить ключ. Обработка отключена."
                }
            }
            Outcome::ClearFailed => "Не удалось удалить настройки. Обработка отключена.",
            Outcome::CheckFailed => {
                "Не удалось проверить подключение. Проверьте адрес, модель, ключ и прокси."
            }
            Outcome::StorageFailed => "Не удалось сохранить настройки. Обработка отключена.",
        });
    }
}
