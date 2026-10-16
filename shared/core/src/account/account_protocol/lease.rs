use crate::{AccountSession, Error, Result};
use serde_json::Value;

/// Bounded local lease decision against the live account owner. It performs no
/// I/O or native callbacks, so synchronous admission need not block on a worker.
/// Desktop commands and Android JNI share this exact implementation.
pub fn credit_lease(core: &AccountSession, params: &Value) -> Result<Value> {
    if params["operation"] != "creditLease" { return Err(Error::InvalidRequest); }
    let expected = params["sessionGeneration"].as_u64().ok_or(Error::InvalidRequest)?;
    let snapshot = core.snapshot();
    if snapshot.session_generation != expected { return Err(Error::SessionChanged); }
    let state = serde_json::from_value(params["state"].clone()).map_err(|_| Error::InvalidRequest)?;
    let observations = serde_json::from_value(params["observations"].clone()).map_err(|_| Error::InvalidRequest)?;
    let now = params["nowMillis"].as_u64().ok_or(Error::InvalidRequest)?;
    let grace = params["graceMillis"].as_u64();
    let decision = crate::credit_lease::transition(state, observations, snapshot.session_generation,
        now, grace, snapshot.signed_in && snapshot.storage_available && !snapshot.logout_pending)?;
    if core.snapshot().session_generation != expected { return Err(Error::SessionChanged); }
    serde_json::to_value(decision).map_err(|_| Error::InvalidResponse)
}
