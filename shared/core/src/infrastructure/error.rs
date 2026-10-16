use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// No server body, URL, credential, user text or OS error enters diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Crypto(crate::style_pass_crypto::CryptoError),
    StyleService(crate::stylization::ServiceFailure),
    StyleTransport(crate::stylization::Failure),
    StyleAccess(crate::stylization::AccessFailure),
    StyleQuota(crate::stylization::AccessFailure),
    SignedOut,
    SessionChanged,
    StorageUnavailable,
    StorageInvalid,
    InvalidResponse,
    InvalidRequest,
    NetworkUnavailable,
    ResponseTooLarge,
    TermsMismatch,
    ConsentUnconfirmed,
    Http(u16),
    StyleIssuance { stage: &'static str, status: u16 },
    Checkout(crate::CheckoutFailure),
    CheckoutRecovery(crate::CheckoutRecoveryFailure),
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::StyleService(failure) => failure.code(),
            Self::StyleTransport(failure) => failure.code(),
            Self::StyleAccess(failure) | Self::StyleQuota(failure) => failure.code(),
            Self::Crypto(failure) => match failure {
                crate::style_pass_crypto::CryptoError::InvalidKey => "style_pass_invalid_key",
                crate::style_pass_crypto::CryptoError::InvalidExpiry => "style_pass_invalid_expiry",
                crate::style_pass_crypto::CryptoError::InvalidRequest => {
                    "style_pass_invalid_request"
                }
                crate::style_pass_crypto::CryptoError::InvalidSignature => {
                    "style_pass_invalid_signature"
                }
                crate::style_pass_crypto::CryptoError::RandomUnavailable => {
                    "style_pass_random_unavailable"
                }
            },
            Self::SignedOut => "account_signed_out",
            Self::SessionChanged => "account_session_changed",
            Self::StorageUnavailable => "account_storage_unavailable",
            Self::StorageInvalid => "account_storage_invalid",
            Self::InvalidResponse => "account_invalid_response",
            Self::InvalidRequest => "account_invalid_request",
            Self::NetworkUnavailable => "account_network_unavailable",
            Self::ResponseTooLarge => "account_response_too_large",
            Self::TermsMismatch => "account_terms_mismatch",
            Self::ConsentUnconfirmed => "account_consent_unconfirmed",
            Self::Http(_) => "account_http_error",
            Self::StyleIssuance { .. } => "style_pass_issue_failed",
            Self::Checkout(failure) => failure.cause_code,
            Self::CheckoutRecovery(failure) => failure.cause_code,
        }
    }

    pub fn http_status(self) -> Option<u16> {
        match self {
            Self::Http(status) | Self::StyleIssuance { status, .. } => Some(status),
            Self::StyleQuota(_) => Some(429),
            Self::Checkout(failure) => failure.http_status,
            Self::CheckoutRecovery(failure) => failure.http_status,
            _ => None,
        }
    }

    /// Shared IPC error contract. Native/JNI transports must preserve the decision
    /// made by the business owner instead of reconstructing it from a status code.
    pub fn wire(self) -> serde_json::Value {
        let mut value = serde_json::json!({"error": self.code(), "status": self.http_status()});
        if let Self::Checkout(failure) = self {
            value["clientCheckoutFailure"] = serde_json::json!(failure);
        }
        if let Self::CheckoutRecovery(failure) = self {
            value["clientCheckoutRecoveryFailure"] = serde_json::json!(failure);
        }
        value
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Error {}
