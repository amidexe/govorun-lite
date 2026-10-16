//! Official anonymous gateway boundary. Account requests use a separate port.
mod execution;
pub mod failure;
pub mod foreground;
mod leases;
mod network;
pub mod outage;
pub mod reconnect;
pub mod recovery;
pub mod reliability;
pub mod reporting;
mod request;
pub(crate) mod request_wire;
pub mod scheduling;
pub mod timing;
use crate::{Future, Result};
pub use request::{Kind, Request};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessFailure {
    NoAccess,
    ReserveEmpty,
    RecoveryPending,
    UsageUnavailable,
    DailyLimit,
    PeriodLimit,
}
/// Cryptographically random, ephemeral request entropy; no fallback identifier.
pub fn random_request_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| crate::Error::InvalidRequest)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}
impl AccessFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::NoAccess => "style_access_unavailable",
            Self::ReserveEmpty => "style_pass_reserve_empty",
            Self::RecoveryPending => "style_pass_recovery_pending",
            Self::UsageUnavailable => "style_usage_unavailable",
            Self::DailyLimit => "style_daily_limit_exhausted",
            Self::PeriodLimit => "style_period_limit_exhausted",
        }
    }
}

/// Closed service reasons; never retain a remote body in an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceFailure {
    Busy,
    Timeout,
    Unavailable,
}
impl ServiceFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::Busy => "style_server_busy",
            Self::Timeout => "style_server_timeout",
            Self::Unavailable => "style_server_unavailable",
        }
    }
}

/// Original body text preserves duplicate-field detection in job policy.
pub struct Response {
    pub status: u16,
    pub body: String,
    pub retry_after: Option<String>,
    pub date: Option<String>,
    pub last_modified: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Network,
    Security,
    Offline,
    Cancelled,
    Deadline,
    Other,
    TooLarge,
    InvalidBody,
}
impl Failure {
    pub(super) fn start(self) -> crate::style_start::Failure {
        match self {
            Self::Network => crate::style_start::Failure::Network,
            Self::Security => crate::style_start::Failure::Security,
            Self::Offline => crate::style_start::Failure::Offline,
            Self::Cancelled => crate::style_start::Failure::Cancelled,
            Self::Deadline => crate::style_start::Failure::Deadline,
            _ => crate::style_start::Failure::Other,
        }
    }
    pub fn error(self) -> crate::Error {
        match self {
            Self::TooLarge => crate::Error::ResponseTooLarge,
            Self::InvalidBody => crate::Error::InvalidResponse,
            Self::Network => crate::Error::NetworkUnavailable,
            _ => crate::Error::StyleTransport(self),
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Network => "style_network_unavailable",
            Self::Security => "style_transport_security",
            Self::Offline => "style_confirmed_offline",
            Self::Cancelled => "style_cancelled",
            Self::Deadline => "style_deadline",
            Self::Other => "style_transport_other",
            Self::TooLarge => "style_response_too_large",
            Self::InvalidBody => "style_invalid_response",
        }
    }
}
pub trait Gateway: Send + Sync {
    /// Optional TCP/TLS preparation for one ephemeral request, without HTTP.
    fn prepare_connection(&self, _request_id: String) {}
    fn close_connection(&self, _request_id: Option<&str>) {}
    fn send(&self, request: Request) -> Future<'_, std::result::Result<Response, Failure>>;
    /// Closed transport facts only; no headers, body, endpoint or identity.
    fn send_observed<'a>(
        &'a self,
        request: Request,
        observe: &'a (dyn Fn(timing::ExchangeEvent) + Send + Sync),
    ) -> Future<'a, std::result::Result<Response, Failure>> {
        Box::pin(async move {
            let response = self.send(request).await;
            if let Ok(response) = &response {
                observe(timing::ExchangeEvent::Headers(response.status));
                observe(timing::ExchangeEvent::Complete);
            }
            response
        })
    }
}

/// A build selects its service; UI preferences cannot expand the trusted origin.
pub fn configured_origin() -> &'static str {
    option_env!("GOVORUN_STYLES_ORIGIN").unwrap_or("https://styles.govorun.club")
}
pub fn origin(value: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(value).map_err(|_| crate::Error::InvalidRequest)?;
    let trusted = url::Url::parse(configured_origin()).map_err(|_| crate::Error::InvalidRequest)?;
    if parsed != trusted
        || parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || parsed.port().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(crate::Error::InvalidRequest);
    }
    Ok(parsed)
}
