//! Atomic account envelope and durable Defaults fences over private files.
#[path = "macos_secrets/account_access.rs"]
mod account_access;
#[path = "macos_secrets/checkout.rs"]
mod checkout;
#[path = "macos_secrets/self_host.rs"]
mod self_host;
#[path = "macos_secrets/restart.rs"]
mod restart;
#[path = "macos_secrets/system_files.rs"]
mod system_files;
pub use system_files::{Files, UserKeys};
#[cfg(target_os = "macos")]
pub use system_files::SystemUserKeys;
use super::{
    defaults_documents::{Cell, Changes, Domain},
    macos_envelope::{AccountEnvelope, Credentials},
    secure_files::{checkout_key, revision},
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Unavailable,
    Conflict,
}
pub trait Keychain: Send + Sync {
    fn new_session_envelope(&self, _: &str) -> Result<AccountEnvelope, Error> {
        Ok(AccountEnvelope::default())
    }
    /// Explicit re-login preserves the existing journal and uses one stable store.
    fn restart(&self, service: &str) -> Result<String, Error> {
        let value = match self.read(service, ENVELOPE, false, false)? {
            Some(raw) => AccountEnvelope::decode(&raw).map_err(|_| Error::Invalid)?,
            None => AccountEnvelope::default(),
        };
        self.write(service, ENVELOPE, &value.encoded().map_err(|_| Error::Invalid)?, false, false)?;
        Ok(service.into())
    }
    fn read(
        &self,
        service: &str,
        key: &str,
        allow: bool,
        llm: bool,
    ) -> Result<Option<String>, Error>;
    fn write(
        &self,
        service: &str,
        key: &str,
        raw: &str,
        allow: bool,
        llm: bool,
    ) -> Result<(), Error>;
    fn delete(&self, services: &[&str], key: &str, allow: bool, llm: bool) -> Result<(), Error>;
}
const ENVELOPE: &str = "govorunSession.v2";
const AUTH: &str = "govorunSharedAuthAttempt.v1";
const MARKER: &str = "govorunAccountStoredSession";
const PROFILE: &str = "govorunSharedAccountProfile.v1";
#[derive(Default)]
struct Cache {
    epoch: u64,
    service: String,
    value: Option<AccountEnvelope>,
    missing: bool,
    corrupt: bool,
    fingerprint: Option<String>,
}
pub struct Owner<D, K> {
    domain: D,
    keychain: K,
    identity: String,
    state: Mutex<Cache>,
    epoch: AtomicU64,
    ended: AtomicBool,
}
impl<D: Domain, K: Keychain> Owner<D, K> {
    pub fn new(domain: D, keychain: K, identity: String) -> Self {
        Self {
            domain,
            keychain,
            identity,
            state: Mutex::new(Cache::default()),
            epoch: AtomicU64::new(0),
            ended: AtomicBool::new(false),
        }
    }
    pub fn advance_boundary(&self, ending: bool) {
        self.ended.store(ending, Ordering::SeqCst);
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
    fn service(&self, _: bool) -> Result<String, Error> {
        Ok(super::macos_profile::account_service(&self.identity))
    }
    fn load(
        &self,
        state: &mut Cache,
        service: &str,
        allow: bool,
    ) -> Result<AccountEnvelope, Error> {
        let epoch = self.epoch.load(Ordering::SeqCst);
        if state.epoch != epoch || state.service != service {
            *state = Cache {
                epoch,
                service: service.into(),
                ..Cache::default()
            };
        }
        if let Some(value) = &state.value {
            return Ok(value.clone());
        }
        if state.corrupt {
            return Err(Error::Invalid);
        }
        if !state.missing {
            let raw = self.keychain.read(service, ENVELOPE, allow, false)?;
            if self.epoch.load(Ordering::SeqCst) != epoch {
                return Err(Error::Unavailable);
            }
            match raw {
                Some(raw) => match AccountEnvelope::decode(&raw) {
                    Ok(value) => {
                        state.value = Some(value.clone());
                        return Ok(value);
                    }
                    Err(()) => {
                        state.corrupt = true;
                        return Err(Error::Invalid);
                    }
                },
                None => state.missing = true,
            }
        }
        Ok(AccountEnvelope::default())
    }
    fn commit(
        &self,
        state: &mut Cache,
        service: &str,
        value: AccountEnvelope,
        epoch: u64,
        allow: bool,
    ) -> Result<(), Error> {
        if self.epoch.load(Ordering::SeqCst) != epoch {
            return Err(Error::Unavailable);
        }
        let raw = value.encoded().map_err(|_| Error::Invalid)?;
        self.keychain.write(service, ENVELOPE, &raw, allow, false)?;
        if self.epoch.load(Ordering::SeqCst) != epoch {
            // Delete only the original service. Recovery may already point elsewhere.
            let _ = self.keychain.delete(&[service], ENVELOPE, false, false);
            *state = Cache::default();
            return Err(Error::Unavailable);
        }
        state.value = Some(value);
        state.missing = false;
        Ok(())
    }
    fn auth_load(&self, service: &str) -> Result<Value, Error> {
        if let Some(raw) = self.keychain.read(service, AUTH, false, false)? {
            let value = parse(&raw, 8192)?;
            return Ok(if value.is_null() {
                json!({"status":"missing"})
            } else {
                json!({"status":"available","value":value})
            });
        }
        Ok(json!({"status":"missing"}))
    }

    fn auth_save(&self, service: &str, value: &Value, allow: bool) -> Result<Value, Error> {
        let raw = value.to_string();
        parse(&raw, 8192)?;
        self.keychain
            .write(service, AUTH, &raw, allow && !value.is_null(), false)?;
        Ok(json!({"status":"available"}))
    }
    fn changed(&self, changes: Changes) -> Result<(), Error> {
        self.domain.write(changes).map_err(|_| Error::Unavailable)
    }
    fn session_end(&self, service: &str) -> Result<Value, Error> {
        self.auth_save(service, &Value::Null, false)?;
        self.changed(
            [
                (MARKER.into(), Some(Cell::Scalar(json!(false)))),
                (PROFILE.into(), None),
            ]
            .into(),
        )?;
        self.advance_boundary(true);
        if let Ok(mut state) = self.state.try_lock() {
            *state = Cache::default();
            let _ = self.keychain.delete(&[service], ENVELOPE, false, false);
        }
        Ok(json!({"status":"available"}))
    }
    fn execute(&self, method: &str, params: &Value) -> Result<Value, Error> {
        let key = params["key"].as_str().ok_or(Error::Invalid)?;
        if key.starts_with("selfHost.") {
            return self.self_host(method, key, params);
        }
        let service = self.service(params["allowInteraction"] == true)?;
        if key == "account.authAttempt" {
            return match method {
                "secret.read" => self.auth_load(&service),
                "secret.write" if params["value"].is_object() => self.auth_save(
                    &service,
                    &params["value"],
                    params["allowInteraction"] == true,
                ),
                "secret.delete" => self.auth_save(&service, &Value::Null, false),
                _ => Err(Error::Invalid),
            };
        }
        if checkout_key(key) {
            return self.checkout(&service, method, key, params);
        }
        if key == "account.session" && method == "secret.delete" {
            return self.session_end(&service);
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        // Account/pass operations use local files on a blocking worker. Ordinary
        // contention must serialize, not revoke the account's storage authority.
        let mut state = self.state.lock().map_err(|_| Error::Unavailable)?;
        if method != "secret.restart" && params["allowInteraction"] != true
            && self.epoch.load(Ordering::SeqCst) != epoch {
            // A queued write belongs to the session before logout/recovery.
            return Err(Error::Unavailable);
        }
        if key == "account.session" && method == "secret.restart" {
            return self.restart(&service, &mut state);
        }
        let has_session = self.domain.boolean(MARKER);
        let allow = params["allowInteraction"] == true;
        if key == "account.session" {
            match method {
                "secret.read" => {
                    if !has_session {
                        return Ok(json!({"status":"missing"}));
                    }
                    if self.ended.load(Ordering::SeqCst) {
                        return Err(Error::Unavailable);
                    }
                    let value = self.load(&mut state, &service, allow)?;
                    let Some(credentials) = value.credentials else {
                        return Ok(json!({"status":"missing"}));
                    };
                    state.fingerprint = Some(revision(
                        &(credentials.access_token.clone() + "\0" + &credentials.refresh_token),
                    ));
                    let values = self.domain.read().map_err(|_| Error::Unavailable)?;
                    let profile = values.get(PROFILE).and_then(|v| {
                        if let Cell::Data(bytes) = v {
                            serde_json::from_slice::<Value>(bytes)
                                .ok()
                                .filter(Value::is_object)
                        } else {
                            None
                        }
                    });
                    return Ok(
                        json!({"status":"available","value":{"version":1,"tokens":{"accessToken":credentials.access_token,"refreshToken":credentials.refresh_token,"displayName":string_cell(values.get("govorunAccountDisplayName")).unwrap_or_else(||"Аккаунт подключён".into()),"deviceId":string_cell(values.get("govorunAccountCurrentDeviceId")).unwrap_or_default(),"deviceLimit":self.domain.integer("govorunAccountDeviceLimit")},"lastConfirmedProfile":profile}}),
                    );
                }
                "secret.write" => {
                    let value = &params["value"];
                    if value["version"] != 1 {
                        return Err(Error::Invalid);
                    }
                    let access = value["tokens"]["accessToken"]
                        .as_str()
                        .filter(|s| !s.is_empty() && *s != "host-owned-session")
                        .ok_or(Error::Invalid)?;
                    let refresh = value["tokens"]["refreshToken"]
                        .as_str()
                        .filter(|s| !s.is_empty() && *s != "host-owned-session")
                        .ok_or(Error::Invalid)?;
                    let fingerprint = revision(&(access.to_owned() + "\0" + refresh));
                    if state.fingerprint.as_ref() != Some(&fingerprint) {
                        if !has_session {
                            self.advance_boundary(false);
                            *state = Cache {
                                epoch: self.epoch.load(Ordering::SeqCst),
                                service: service.clone(),
                                ..Cache::default()
                            };
                        }
                        if self.ended.load(Ordering::SeqCst) {
                            return Err(Error::Unavailable);
                        }
                        let epoch = self.epoch.load(Ordering::SeqCst);
                        let mut envelope = if has_session {
                            self.load(&mut state, &service, allow)?
                        } else {
                            self.keychain.new_session_envelope(&service)?
                        };
                        envelope.credentials = Some(Credentials {
                            access_token: access.into(),
                            refresh_token: refresh.into(),
                        });
                        self.commit(&mut state, &service, envelope, epoch, allow)?;
                        state.fingerprint = Some(fingerprint);
                    }
                    let tokens = &value["tokens"];
                    let mut changes: Changes = [
                        (MARKER.into(), Some(Cell::Scalar(json!(true)))),
                        (
                            "govorunAccountDisplayName".into(),
                            tokens
                                .get("displayName")
                                .filter(|v| v.is_string())
                                .cloned()
                                .map(Cell::Scalar),
                        ),
                        (
                            "govorunAccountCurrentDeviceId".into(),
                            tokens
                                .get("deviceId")
                                .filter(|v| v.is_string())
                                .cloned()
                                .map(Cell::Scalar),
                        ),
                        (
                            "govorunAccountDeviceLimit".into(),
                            tokens
                                .get("deviceLimit")
                                .filter(|v| v.as_i64().is_some())
                                .cloned()
                                .map(Cell::Scalar),
                        ),
                    ]
                    .into();
                    if value["lastConfirmedProfile"].is_object() {
                        changes.insert(
                            PROFILE.into(),
                            Some(Cell::Data(
                                serde_json::to_vec(&value["lastConfirmedProfile"])
                                    .map_err(|_| Error::Invalid)?,
                            )),
                        );
                    }
                    self.changed(changes)?;
                    return Ok(json!({"status":"available"}));
                }
                _ => return Err(Error::Invalid),
            }
        }
        if key != "stylePass.vault"
            || !params.as_object().is_some_and(|p| {
                p.keys().all(|k| {
                    matches!(
                        k.as_str(),
                        "key" | "value" | "expectedRevision" | "allowInteraction"
                    )
                })
            })
            || params
                .get("allowInteraction")
                .is_some_and(|v| !v.is_boolean())
        {
            return Err(Error::Invalid);
        }
        if self.ended.load(Ordering::SeqCst) || !has_session {
            return if method == "secret.read" {
                Ok(json!({"status":"missing"}))
            } else {
                Err(Error::Unavailable)
            };
        }
        // The pool is separate, but never usable without readable current credentials.
        if self.load(&mut state, &service, allow)?.credentials.is_none() {
            return if method == "secret.read" { Ok(json!({"status":"missing"})) } else { Err(Error::Unavailable) };
        }
        let slot = "govorunReadyPassPool.v1";
        let raw = self.keychain.read(&service, slot, false, false)?;
        match method {
            "secret.read" => Ok(match raw {
                None => json!({"status":"missing"}),
                Some(raw) => json!({"status":"available","revision":revision(&raw),"value":raw}),
            }),
            "secret.write" => {
                let value = params["value"].as_str().filter(|s| !s.is_empty() && s.len() <= 262144).ok_or(Error::Invalid)?;
                if expected(params)? != raw.as_deref().map(revision).as_deref() { return Err(Error::Conflict); }
                self.keychain.write(&service, slot, value, false, false)?;
                Ok(json!({"status":"available","revision":revision(value)}))
            }
            _ => Err(Error::Invalid),
        }

    }
    pub fn invoke(&self, method: &str, params: &Value) -> Value {
        self.execute(method,params).unwrap_or_else(|e|json!({"status":match e{Error::Invalid=>"invalid",Error::Unavailable|Error::Denied=>"unavailable",Error::Conflict=>"conflict"}}))
    }
}
fn parse(raw: &str, max: usize) -> Result<Value, Error> {
    if raw.len() > max {
        return Err(Error::Invalid);
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
    if !value.is_object() && !value.is_null() {
        return Err(Error::Invalid);
    }
    Ok(value)
}
fn expected(p: &Value) -> Result<Option<&str>, Error> {
    let v = p.get("expectedRevision").ok_or(Error::Invalid)?;
    if v.is_null() {
        Ok(None)
    } else {
        v.as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .map(Some)
            .ok_or(Error::Invalid)
    }
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|v| v.len() == keys.len() && keys.iter().all(|k| v.contains_key(*k)))
}
fn string_cell(cell: Option<&Cell>) -> Option<String> {
    match cell {
        Some(Cell::Scalar(Value::String(v))) => Some(v.clone()),
        _ => None,
    }
}
