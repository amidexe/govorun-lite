use super::CheckoutService;
use crate::{Error, HttpResponse, Result};
use serde::Serialize;
use serde_json::json;

/// The recovery owner distinguishes an empty discovery from an unknown payment.
/// A native consumer must not reconstruct this decision from an HTTP status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CheckoutRecoveryFailure {
    #[serde(skip)]
    pub(crate) cause_code: &'static str,
    #[serde(skip)]
    pub(crate) http_status: Option<u16>,
    pub discovery_empty: bool,
    pub message: Option<&'static str>,
}

impl CheckoutService {
    pub(super) fn recovery_result(&self, epoch: u64, identity: u64,
        discovery: bool, cancelling: bool, result: Result<HttpResponse>) -> Result<HttpResponse> {
        // The caller still holds the operation lock. A later recovery/cancel must
        // not alter the decision attached to this result.
        let state = self.checked(epoch, identity)?;
        let empty_allowed = discovery && !cancelling && state.account.is_some()
            && state.observed.is_none() && !state.attempt.as_ref().is_some_and(|attempt| attempt.unresolved());
        drop(state);
        let cause = match &result {
            Err(error @ (Error::SignedOut | Error::SessionChanged)) => return Err(*error),
            Err(error) => *error,
            Ok(reply) if !(200..300).contains(&reply.status) => Error::Http(reply.status),
            _ => {
                self.checked(epoch, identity)?.discovery = super::discovery::Discovery::Verified;
                return result;
            }
        };
        let discovery_empty = empty_allowed && cause.http_status() == Some(404);
        self.checked(epoch, identity)?.discovery = if discovery_empty {
            super::discovery::Discovery::Verified
        } else { super::discovery::Discovery::Inconclusive };
        let message = if discovery_empty { None } else if cancelling {
            Some("Не удалось закрыть попытку. Проверьте состояние подключения.")
        } else if cause.http_status() == Some(404) && !discovery {
            Some("Не удалось найти операцию. Обновите сведения о подписке.")
        } else if empty_allowed && cause == Error::NetworkUnavailable {
            Some("Нет доступа к интернету. Проверьте подключение и настройки VPN, затем повторите попытку.")
        } else if empty_allowed {
            Some("Не удалось загрузить сведения о подписке. Попробуйте ещё раз.")
        } else {
            Some("Пока не удалось проверить подключение. Повторять оплату не нужно. Проверка продолжится автоматически.")
        };
        let failure = CheckoutRecoveryFailure { cause_code: cause.code(),
            http_status: cause.http_status(), discovery_empty, message };
        match result {
            Err(_) => Err(Error::CheckoutRecovery(failure)),
            Ok(mut reply) => {
                // Transport fields are untrusted. Only this owner sets the directive.
                if !reply.body.is_object() { reply.body = json!({}); }
                reply.body["clientCheckoutRecoveryFailure"] = json!(failure);
                Ok(reply)
            }
        }
    }
}
