use crate::{Error, Result};

const OFFICIAL_ORIGIN: &str = "https://accounts.govorun.club";

/// A distributor can select another account service at build time. Neither
/// renderer preferences nor a native worker may expand this trust boundary.
pub fn configured_account_origin() -> &'static str {
    option_env!("GOVORUN_ACCOUNT_ORIGIN").unwrap_or(OFFICIAL_ORIGIN)
}

/// Import is a capability of a distributor-selected account service. The
/// official service and renderer-selected origins never enable this test port.
pub fn configured_refresh_import_allowed(origin: &str) -> bool {
    let configured = configured_account_origin();
    configured != OFFICIAL_ORIGIN && origin == configured
}

pub fn accounts_origin(value: &str) -> Result<url::Url> {
    let origin = url::Url::parse(value).map_err(|_| Error::InvalidRequest)?;
    if origin.scheme() != "https" || origin.host_str().is_none() || origin.port().is_some()
        || !origin.username().is_empty() || origin.password().is_some() || origin.path() != "/"
        || origin.query().is_some() || origin.fragment().is_some() {
        return Err(Error::InvalidRequest);
    }
    let trusted = [OFFICIAL_ORIGIN, configured_account_origin()].iter().any(|candidate|
        url::Url::parse(candidate).is_ok_and(|candidate| candidate == origin));
    if !trusted { return Err(Error::InvalidRequest); }
    Ok(origin)
}

pub(crate) fn callback_scheme(origin: &url::Url) -> Result<&'static str> {
    if origin.as_str() == "https://accounts.govorun.club/" { return Ok("govorun"); }
    let scheme = option_env!("GOVORUN_AUTH_SCHEME").ok_or(Error::InvalidRequest)?;
    if scheme.is_empty() || scheme.len() > 64 || scheme == "govorun"
        || !scheme.as_bytes()[0].is_ascii_lowercase()
        || !scheme.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()
            || matches!(byte, b'+' | b'-' | b'.')) {
        return Err(Error::InvalidRequest);
    }
    Ok(scheme)
}
