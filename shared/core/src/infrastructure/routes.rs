//! Typed command ownership table consumed by host dispatch and runtime view.
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Route {
    Billing,
    Account,
    About,
    Documents,
    SelfHost,
    Clipboard,
    Hotkeys,
    Diagnostics,
    Benchmark,
    RecordingSupport,
    PermissionSetup,
    Agent,
    Runtime,
}

pub fn routes() -> &'static BTreeMap<String, Route> {
    static ROUTES: OnceLock<BTreeMap<String, Route>> = OnceLock::new();
    ROUTES.get_or_init(|| {
        serde_json::from_str(include_str!("../../contracts/host-routes.json"))
            .expect("bundled feature ownership table")
    })
}

pub fn route(method: &str) -> Result<Route, &'static str> {
    routes().get(method).copied().ok_or("unsupported_command")
}
