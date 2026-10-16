use crate::{Error, Result};
use serde::{Deserialize, Serialize};

const LIFETIME_MILLIS: u64 = 47 * 60 * 60 * 1_000;
const COHORT_MILLIS: u64 = 6 * 60 * 60 * 1_000;
// The wire must be representable by every native date adapter, including .NET.
const MAX_DATE_MILLIS: u64 = 253_402_300_799_999;

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Preparation {
    pub(crate) now_millis: u64,
    pub(crate) key_not_after_millis: u64,
    pub(crate) access_end_millis: Option<u64>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Validity {
    pub(crate) expires_at_millis: u64,
    pub(crate) expires_with_scope: bool,
    pub(crate) replay_until_millis: u64,
}

/// This bounds cryptographic material and recovery, never entitlement or quota.
/// Signed expiry is the shared key expiry, independent of the account. Access
/// and consumed allowance still gate admission; replay recovery keeps its bound.
pub(crate) fn prepare(input: Preparation) -> Result<Validity> {
    let now = input.now_millis;
    if now > MAX_DATE_MILLIS - LIFETIME_MILLIS
        || input.key_not_after_millis > MAX_DATE_MILLIS
        || input.key_not_after_millis <= now
        || input
            .access_end_millis
            .is_some_and(|end| end <= now || end > MAX_DATE_MILLIS)
    {
        return Err(Error::InvalidRequest);
    }
    let replay_until_millis = now + LIFETIME_MILLIS;
    // Owner-authorized fallback after the rounded access-end admission failure.
    // Every account using this key has the same expiry; keep the existing take margin.
    let expires_at_millis = input.key_not_after_millis;
    if expires_at_millis < now + crate::style_inventory::TAKE_MARGIN {
        return Err(Error::InvalidRequest);
    }
    Ok(Validity {
        expires_at_millis,
        expires_with_scope: false,
        replay_until_millis,
    })
}

/// Legacy stored passes can still be counted at their former access ceiling.
pub(crate) fn cohort_end(ceiling:u64)->u64 {ceiling/COHORT_MILLIS*COHORT_MILLIS}
