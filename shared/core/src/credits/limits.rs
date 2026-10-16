// Generated from shared/core/contracts/stylization-limits.json. Do not edit.
pub(crate) const MINIMUM_CHARACTERS: usize = 60;
pub(crate) const MAXIMUM_CHARACTERS: usize = 10000;
pub(crate) const CHARACTERS_PER_CREDIT: usize = 1000;
pub(crate) const MINIMUM_BATCH_CREDITS: usize = 12;
pub(crate) const RESERVE_REQUESTS: usize = 3;
const MAXIMUM_REQUEST_CREDITS: usize = MAXIMUM_CHARACTERS.div_ceil(CHARACTERS_PER_CREDIT);
pub(crate) const BATCH_CREDITS: usize = if MINIMUM_BATCH_CREDITS > MAXIMUM_REQUEST_CREDITS { MINIMUM_BATCH_CREDITS } else { MAXIMUM_REQUEST_CREDITS };
