//! Account OS facts and effects; the current session generation comes from Core.
use crate::agent::Generation;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub struct AccountPorts {
    generation: OnceLock<Generation>,
    scheme: OnceLock<String>,
}
impl AccountPorts {
    pub fn new() -> Self {
        Self {
            generation: OnceLock::new(),
            scheme: OnceLock::new(),
        }
    }
    pub fn bind(&self, generation: Generation) -> Result<(), &'static str> {
        self.generation
            .set(generation)
            .map_err(|_| "account_already_initialized")
    }
    pub fn configure(&self, snapshot: &Value) -> Result<(), &'static str> {
        let scheme = snapshot["callbackScheme"]
            .as_str()
            .filter(|scheme| {
                !scheme.is_empty()
                    && scheme.len() <= 64
                    && scheme
                        .bytes()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-')
            })
            .ok_or("account_invalid_configuration")?;
        self.scheme
            .set(scheme.into())
            .map_err(|_| "account_already_initialized")
    }
    pub fn effect(&self, params: &Value) -> Result<&str, &'static str> {
        let object = params.as_object().ok_or("invalid_parameters")?;
        if object.len() != 2 || !object.contains_key("generation") || !object.contains_key("effect")
        {
            return Err("invalid_parameters");
        }
        let generation = params["generation"].as_u64().ok_or("invalid_parameters")?;
        if self.generation.get().and_then(|read| read()) != Some(generation) {
            return Err("account_session_changed");
        }
        match params["effect"].as_str() {
            Some("prepareSignIn") => Ok("prepareSignIn"),
            Some("recoverVault") => Ok("recoverVault"),
            _ => Err("unsupported_command"),
        }
    }
    pub fn prepare_sign_in(&self) -> Result<(), &'static str> {
        #[cfg(target_os = "windows")]
        {
            let scheme = self.scheme.get().ok_or("account_invalid_configuration")?;
            let executable = std::env::current_exe().map_err(|_| "storage_unavailable")?;
            super::windows_protocol::register(scheme, &executable)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(())
        }
    }
}

pub fn facts(storage: &str, device_name: String, isolated_mac: bool) -> Value {
    json!({"deviceName":device_name,"storageState":storage,
        "providers":[{"id":"yandex","enabled":true},{"id":"vk","enabled":!isolated_mac}],
        "providerNoticeCode":if isolated_mac {Some("account_provider_unavailable")} else {None}})
}

pub fn windows_access(session: &Value, vault: &Value) -> &'static str {
    for value in [session, vault] {
        match value["status"].as_str() {
            Some("available" | "missing") => {}
            Some("invalid") => return "invalidData",
            _ => return "temporarilyUnavailable",
        }
    }
    "available"
}
