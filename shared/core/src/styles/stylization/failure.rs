//! The common owner chooses the diagnostic reason. Platform ports decode this
//! closed DTO; they must not infer business decisions from HTTP/error messages.
use super::{AccessFailure, Failure, ServiceFailure};
use crate::Error;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub reason: &'static str,
    pub cancelled: bool,
}
pub fn decide(error: Error) -> Decision {
    let cancelled = matches!(
        error,
        Error::SessionChanged | Error::StyleTransport(Failure::Cancelled)
    );
    let reason = match error {
        Error::StyleAccess(AccessFailure::ReserveEmpty) => "style_pass_reserve_empty",
        Error::StyleAccess(AccessFailure::RecoveryPending) => "style_pass_recovery_pending",
        Error::StyleAccess(AccessFailure::UsageUnavailable | AccessFailure::DailyLimit | AccessFailure::PeriodLimit | AccessFailure::NoAccess)
        | Error::Http(401 | 402 | 403)
        | Error::StyleIssuance { status:401 | 402 | 403, .. }
        | Error::StyleQuota(_)
        | Error::SignedOut
        | Error::SessionChanged
        | Error::TermsMismatch
        | Error::ConsentUnconfirmed
        | Error::StorageUnavailable
        | Error::StorageInvalid => "account_state_changed",
        Error::StyleIssuance { .. } => "style_pass_issue_failed",
        Error::StyleService(ServiceFailure::Busy) | Error::Http(429) => "server_busy",
        Error::StyleService(ServiceFailure::Timeout) | Error::StyleTransport(Failure::Deadline)
        | Error::Http(504) => {
            "server_timeout"
        }
        Error::StyleService(ServiceFailure::Unavailable) | Error::Http(_) => "server_unavailable",
        Error::StyleTransport(Failure::Offline) => "confirmed_offline",
        Error::StyleTransport(Failure::Network | Failure::Security | Failure::Other)
        | Error::NetworkUnavailable => "transport_failed",
        Error::InvalidResponse
        | Error::InvalidRequest
        | Error::ResponseTooLarge
        | Error::StyleTransport(Failure::InvalidBody | Failure::TooLarge)
        | Error::Crypto(_) => "invalid_response",
        _ => "request_failed",
    };
    Decision { reason, cancelled }
}

/// Encode the same decision in the closed foreground-workflow vocabulary.
pub fn dictation(error: Error) -> Decision {
    let mut decision = decide(error);
    decision.reason = match decision.reason {
        "style_pass_reserve_empty" => "stylePassReserveEmpty",
        "style_pass_recovery_pending" => "stylePassRecoveryPending",
        "account_state_changed" => "accountStateChanged",
        "server_busy" => "serverBusy",
        "server_timeout" => "serverTimeout",
        "server_unavailable" => "serverUnavailable",
        "confirmed_offline" => "offline",
        "transport_failed" => "transportFailed",
        "invalid_response" => "invalidResponse",
        _ => "requestFailed",
    };
    decision
}

/// Product refusals are not technical incidents. Keep the legacy wire enum intact.
pub fn technical(reason: &str) -> bool {
    matches!(reason, "request_failed" | "server_busy" | "server_timeout" |
        "server_unavailable" | "invalid_response" | "insertion_failed" |
        "style_pass_issue_failed" | "transport_failed")
}
/// One completed official operation, excluding access/quota/offline/cancellation.
pub fn processing_result(error: Option<Error>) -> Option<bool> {
    match error {
        None => Some(false),
        Some(Error::Http(401 | 402 | 403) | Error::StyleIssuance {status:401 | 402 | 403,..}) => None,
        Some(error) => {
            let decision=decide(error);
            (!decision.cancelled && technical(decision.reason)).then_some(true)
        }
    }
}
