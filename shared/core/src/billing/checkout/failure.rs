use super::CheckoutService;
use crate::{Error, HttpResponse, Result};
use serde::Serialize;
use serde_json::json;

/// A decision by the attempt owner, captured before releasing its operation lock.
/// The cause stays diagnostic data; consumers only render the supplied directive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CheckoutFailure {
    #[serde(skip)]
    pub(crate) cause_code: &'static str,
    #[serde(skip)]
    pub(crate) http_status: Option<u16>,
    pub discard_confirmation: bool,
    pub message: &'static str,
}

impl CheckoutFailure {
    pub(super) fn invalid_contact() -> Self {
        Self { cause_code: "account_invalid_request", http_status: None,
            discard_confirmation: false,
            message: "Укажите корректный email для кассового чека." }
    }
}

impl CheckoutService {
    /// UI decisions follow the protected attempt, never an HTTP status alone.
    /// An ambiguous earlier dispatch survives a later rejection of its retry.
    pub(super) fn present_failure(&self, epoch: u64, identity: u64,
        mut response: HttpResponse) -> Result<HttpResponse> {
        let failure = self.confirmation_failure(epoch, identity, Error::Http(response.status))?;
        // Never accept a UI directive supplied by the transport/server body.
        if !response.body.is_object() { response.body = json!({}); }
        response.body["clientCheckoutFailure"] = json!(failure);
        Ok(response)
    }

    pub(super) fn confirmation_failure(&self, epoch: u64, identity: u64,
        cause: Error) -> Result<CheckoutFailure> {
        let state = self.checked(epoch, identity)?;
        // A failed restore cannot prove the absence of a durable purchase.
        let discard = state.account.is_some() && state.observed.is_none()
            && !state.attempt.as_ref().is_some_and(|attempt| attempt.unresolved());
        // A correctable form error keeps the admitted disclosure only before
        // dispatch. An existing uncertain attempt always keeps its recovery directive.
        if discard {
            if let Error::Checkout(input_failure) = cause { return Ok(input_failure); }
        }
        let message = if discard {
            "Не удалось оформить подписку. Попробуйте выбрать тариф ещё раз."
        } else {
            "Не удалось получить результат операции. Повторите подтверждение, чтобы проверить результат."
        };
        Ok(CheckoutFailure { cause_code: cause.code(), http_status: cause.http_status(),
            discard_confirmation: discard, message })
    }
}
