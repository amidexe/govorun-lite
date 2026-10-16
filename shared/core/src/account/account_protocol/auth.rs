use crate::{AccountSession, AuthDevice, AuthFlow, AuthIntent, AuthProvider, Error};
use serde_json::{json, Value};
use std::sync::Mutex;

pub fn reserve_native(
    flow: &AuthFlow,
    params: &Value,
    latest: &Mutex<u64>,
) -> Result<AuthIntent, Error> {
    let operation = params["operation"].as_str();
    let changes = matches!(
        operation,
        Some("authBegin" | "authImportRefresh" | "cancelSignIn" | "logout" | "restartStorage")
    );
    if !changes && !matches!(operation, Some("authCallback" | "authReplace")) {
        return reserve(flow, params);
    }
    let incoming = params["authIntent"].as_u64().ok_or(Error::InvalidRequest)?;
    let mut latest = latest
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if (changes && incoming <= *latest) || (!changes && incoming != *latest) {
        return Err(Error::SessionChanged);
    }
    let intent = reserve(flow, params)?;
    // A rejected Begin must not supersede a pending Logout.
    if changes {
        *latest = incoming;
    }
    Ok(intent)
}

pub fn reserve(flow: &AuthFlow, params: &Value) -> Result<AuthIntent, Error> {
    match params["operation"].as_str() {
        Some("authImportRefresh") => flow.reserve_refresh_import(
            params["refreshToken"].as_str().ok_or(Error::InvalidRequest)?),
        Some("authBegin") => {
            serde_json::from_value::<AuthProvider>(params["provider"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            flow.reserve_begin()
        }
        Some("restartStorage") if params["confirmed"] != true => Err(Error::InvalidRequest),
        Some("cancelSignIn" | "logout" | "restartStorage") => Ok(flow.reserve_cancel()),
        _ if logout_request(params) => Ok(flow.reserve_cancel()),
        _ => Ok(flow.pending_intent()),
    }
}

fn logout_request(params: &Value) -> bool {
    params["operation"] == "http"
        && params["method"] == "POST"
        && params["path"] == "/v1/auth/logout"
}

pub async fn dispatch(
    core: &AccountSession,
    flow: &AuthFlow,
    intent: AuthIntent,
    params: Value,
) -> Result<Value, Error> {
    if !matches!(
        params["operation"].as_str(),
        Some(
            "authBegin"
                | "authImportRefresh"
                | "authCallback"
                | "authReplace"
                | "cancelSignIn"
                | "logout"
                | "restartStorage"
                | "authAbandon"
        )
    ) && !logout_request(&params)
    {
        return super::dispatch(core, params).await;
    }
    flow.check_intent(intent)?;
    let device = || -> Result<AuthDevice, Error> {
        Ok(AuthDevice {
            installation_id: params["installationId"]
                .as_str()
                .ok_or(Error::InvalidRequest)?
                .into(),
            name: params["deviceName"]
                .as_str()
                .ok_or(Error::InvalidRequest)?
                .into(),
        })
    };
    let exchange = match params["operation"].as_str() {
        Some("authImportRefresh") => {
            flow.import_refresh_reserved(intent,
                params["refreshToken"].as_str().ok_or(Error::InvalidRequest)?.into()).await?;
            return Ok(core.native_projection());
        }
        Some("authAbandon") => {
            flow.abandon_browser_reserved(intent).await?;
            return Ok(core.native_projection());
        }
        Some("authBegin") => {
            let provider = serde_json::from_value(params["provider"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            return Ok(json!({"url": flow.begin_reserved(intent, provider).await?}));
        }
        Some("authCallback") => {
            flow.callback_reserved(
                intent,
                params["url"].as_str().ok_or(Error::InvalidRequest)?,
                device()?,
            )
            .await
        }
        Some("authReplace") => {
            flow.replace_reserved(
                intent,
                params["deviceId"].as_str().ok_or(Error::InvalidRequest)?,
                params["confirmed"] == true,
                device()?,
            )
            .await
        }
        Some("cancelSignIn") => {
            flow.cancel_reserved(intent).await?;
            return Ok(core.native_projection());
        }
        Some("logout") => {
            flow.logout_reserved(intent).await?;
            return Ok(core.native_projection());
        }
        Some("restartStorage") => {
            if params["confirmed"] != true {
                return Err(Error::InvalidRequest);
            }
            flow.restart_storage_reserved(intent).await?;
            return Ok(core.native_projection());
        }
        _ if logout_request(&params) => {
            flow.logout_reserved(intent).await?;
            return Ok(json!({"status": 204, "body": null}));
        }
        _ => return super::dispatch(core, params).await,
    };
    let pending = core.native_projection()["pendingDeviceLimit"].clone();
    match exchange {
        Ok(()) => Ok(json!({"status": 200, "body": core.native_projection()["tokens"]})),
        Err(Error::Http(409)) if pending.is_object() => Ok(json!({"status": 409, "body": pending})),
        Err(error) => Err(error),
    }
}
