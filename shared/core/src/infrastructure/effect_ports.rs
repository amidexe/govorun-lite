//! Portable effect replies and content-safe lifecycle events.
use serde_json::Value;
pub type Reply = Result<Value, &'static str>;
#[derive(Clone, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NativeEvent { pub event: String, pub data: Value }

/// Private worker-to-host RPC. It is deliberately not an event and cannot be
/// serialized by the UI event delivery path.
pub struct HostRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}
