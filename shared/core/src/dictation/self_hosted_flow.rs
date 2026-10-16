use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
#[path="self_hosted_flow/configuration.rs"]
pub mod configuration;
#[path="self_hosted_flow/endpoint.rs"]
pub mod endpoint;
#[path="self_hosted_flow/wire.rs"]
pub mod wire;
#[path="self_hosted_flow/settings.rs"]
pub mod settings;
#[path="self_hosted_flow/stored.rs"]
pub mod stored;
#[path="self_hosted_flow/controller.rs"]
pub mod controller;

#[path="self_hosted_flow/android_settings.rs"]
pub(crate) mod android_settings;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    Begin { input: Vec<u16> },
    Response { input: Vec<u16>, output: Vec<u16> },
    Failure { status: u16 },
}

/// A user-owned server is independent of official quotas, size limits and deadlines.
pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let (action, fallback, diagnostic) = match command {
        Command::Begin { input } => {
            let input = String::from_utf16(&input).map_err(|_| Error::InvalidRequest)?;
            (if input.trim().is_empty() { "local" } else { "request" }, false, "")
        }
        Command::Response { input, output } => {
            let input = String::from_utf16(&input).map_err(|_| Error::InvalidRequest)?;
            let output = String::from_utf16(&output).map_err(|_| Error::InvalidRequest)?;
            if output.trim().is_empty() { ("local", true, "failed") }
            else if output.trim() == input.trim() { ("output", false, "unchanged") }
            else { ("output", false, "success") }
        }
        Command::Failure { status } =>
            ("local", true, if status == 401 || status == 403 { "auth_failed" } else { "failed" }),
    };
    Ok(json!({"action": action, "fallback": fallback, "diagnostic": diagnostic}))
}
