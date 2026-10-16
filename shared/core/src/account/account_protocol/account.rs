//! Closed account operations: the core owns wire paths, methods and bodies.
use crate::{AccountSession, Error};
use serde_json::{json, Value};

pub(super) fn request(params: Value) -> Result<Value, Error> {
    let epoch = params["sessionGeneration"].as_u64().ok_or(Error::InvalidRequest)?;
    let (method, path, body) = match params["operation"].as_str() {
        Some("accountProfile") => ("GET", "/v1/account".to_owned(), Value::Null),
        Some("accountDevices") => ("GET", "/v1/devices".to_owned(), Value::Null),
        Some("accountUpdateDevice") => {
            let name = params["deviceName"].as_str().filter(|name| !name.is_empty() && name.len() <= 256)
                .ok_or(Error::InvalidRequest)?;
            ("PUT", "/v1/devices/current".to_owned(), json!({"deviceName":name}))
        }
        Some("accountRevokeDevice") => {
            let id = params["deviceId"].as_str().ok_or(Error::InvalidRequest)?;
            if !crate::request::uuid(id) { return Err(Error::InvalidRequest); }
            ("DELETE", format!("/v1/devices/{id}"), Value::Null)
        }
        Some("accountConsent") => {
            let version = params["version"].as_u64().and_then(|value| u32::try_from(value).ok())
                .ok_or(Error::InvalidRequest)?;
            ("POST", "/v1/consents/online-stylization".to_owned(), json!({"version":version}))
        }
        Some("accountDelete") => {
            if params["confirmed"] != true { return Err(Error::InvalidRequest); }
            ("DELETE", "/v1/account".to_owned(), json!({"confirmation":"delete_account"}))
        }
        _ => return Err(Error::InvalidRequest),
    };
    Ok(json!({"sessionGeneration":epoch,"method":method,"path":path,
        "body":body,"confirmed":params["confirmed"] == true}))
}
pub(super) async fn dispatch(core: &AccountSession, params: Value) -> Result<Value, Error> {
    super::http(core, request(params)?).await
}
