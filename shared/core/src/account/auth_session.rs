use crate::{AccountSession, Future, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Facts needed by authorization, without credentials or platform UI state.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthSessionSnapshot {
    pub session_generation: u64,
    pub signed_in: bool,
    pub signing_in: bool,
    pub storage_available: bool,
    pub logout_pending: bool,
}

/// Authorization owns admission and attempt lifetime. A session supplies its
/// atomic identity boundary, protected restoration and confirmed installation.
pub trait AuthSession: Send + Sync {
    fn auth_snapshot(&self) -> AuthSessionSnapshot;
    fn cancel_sign_in(&self);
    fn finish_auth_cancellation(&self) -> Future<'_, Result<()>>;
    fn restore_for_auth(&self) -> Future<'_, Result<()>>;
    fn exchange_auth(&self, expected_epoch: u64, body: Value) -> Future<'_, Result<()>>;
    fn pending_device_limit(&self) -> Option<Value>;
}

impl AuthSession for AccountSession {
    fn auth_snapshot(&self) -> AuthSessionSnapshot {
        let snapshot = self.snapshot();
        AuthSessionSnapshot {
            session_generation: snapshot.session_generation,
            signed_in: snapshot.signed_in,
            signing_in: snapshot.signing_in,
            storage_available: snapshot.storage_available,
            logout_pending: snapshot.logout_pending,
        }
    }

    fn cancel_sign_in(&self) {
        AccountSession::cancel_sign_in(self);
    }

    fn finish_auth_cancellation(&self) -> Future<'_, Result<()>> {
        Box::pin(self.finish_sign_in_cancellation())
    }

    fn restore_for_auth(&self) -> Future<'_, Result<()>> {
        Box::pin(self.restore(true))
    }

    fn exchange_auth(&self, expected_epoch: u64, body: Value) -> Future<'_, Result<()>> {
        Box::pin(self.exchange_for_generation(expected_epoch, body))
    }

    fn pending_device_limit(&self) -> Option<Value> {
        self.state().pending_device_limit.clone()
    }
}
